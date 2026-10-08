package net.scopenet.paper.commands;

import com.google.gson.*;
import net.scopenet.integration.Integration;
import net.scopenet.integration.PanelClient;
import net.scopenet.paper.ScopenetPlugin;
import org.bukkit.Bukkit;
import org.bukkit.ChatColor;
import org.bukkit.configuration.file.YamlConfiguration;
import org.bukkit.entity.Player;
import org.bukkit.inventory.ItemStack;
import java.io.File;
import java.util.*;

/** Durable escrow plus idempotent panel requests. All inventory/file access is on the server thread. */
final class EconomyOperations {
    private final ScopenetPlugin plugin;
    private final Integration integration;
    private final File file;
    private final Map<String, JsonObject> jobs = new LinkedHashMap<>();
    private final Set<String> running = new HashSet<>();

    EconomyOperations(ScopenetPlugin plugin, Integration integration) {
        this.plugin = plugin;
        this.integration = integration;
        file = new File(plugin.getDataFolder(), "economy-pending.yml");
        YamlConfiguration yaml = YamlConfiguration.loadConfiguration(file);
        for (String id : yaml.getKeys(false)) jobs.put(id, JsonParser.parseString(yaml.getString(id)).getAsJsonObject());
        Bukkit.getScheduler().runTaskTimer(plugin, this::tick, 20, 100);
    }

    static String encode(ItemStack item) {
        YamlConfiguration yaml = new YamlConfiguration();
        yaml.set("item", item);
        return yaml.saveToString();
    }

    static ItemStack decode(String data) throws Exception {
        YamlConfiguration yaml = new YamlConfiguration();
        yaml.loadFromString(data);
        ItemStack item = yaml.getItemStack("item");
        if (item == null) throw new IllegalArgumentException("Missing item data");
        return item;
    }

    static void give(Player player, ItemStack item) {
        player.getInventory().addItem(item).values().forEach(left -> player.getWorld().dropItemNaturally(player.getLocation(), left));
    }

    boolean enqueue(Player player, String endpoint, JsonObject payload, List<ItemStack> escrow, ItemStack purchase) {
        String id = UUID.randomUUID().toString();
        payload.addProperty("operation_id", id);
        JsonObject job = new JsonObject();
        job.addProperty("player", player.getUniqueId().toString());
        job.addProperty("endpoint", endpoint);
        job.add("payload", payload);
        JsonArray items = new JsonArray();
        for (ItemStack item : escrow) items.add(encode(item));
        job.add("escrow", items);
        if (purchase != null) job.addProperty("purchase", encode(purchase));
        jobs.put(id, job);
        try { save(); } catch (Exception e) {
            jobs.remove(id);
            player.sendMessage(ChatColor.RED + "Cannot save transaction; nothing was purchased or sold.");
            return false;
        }
        player.sendMessage(ChatColor.GRAY + "Transaction queued. Items are kept safe while the panel responds.");
        return true;
    }

    private void save() throws Exception {
        YamlConfiguration yaml = new YamlConfiguration();
        jobs.forEach((id, job) -> yaml.set(id, job.toString()));
        File temp = new File(file.getParentFile(), file.getName() + ".tmp");
        yaml.save(temp);
        java.nio.file.Files.move(temp.toPath(), file.toPath(), java.nio.file.StandardCopyOption.REPLACE_EXISTING);
    }

    private void tick() {
        for (var entry : new ArrayList<>(jobs.entrySet())) {
            String id = entry.getKey();
            JsonObject job = entry.getValue();
            if (job.has("result") || job.has("error")) { deliver(id, job); continue; }
            if (!running.add(id)) continue;
            String endpoint = job.get("endpoint").getAsString();
            JsonObject payload = job.getAsJsonObject("payload").deepCopy();
            Bukkit.getScheduler().runTaskAsynchronously(plugin, () -> {
                JsonObject result = null;
                String error = null;
                try { result = integration.client().post(endpoint, payload); }
                catch (PanelClient.HttpFailure e) { if (e.status >= 400 && e.status < 500) error = e.getMessage(); }
                catch (Exception e) { /* Unknown outcome: retry the same operation ID; never duplicate a credit. */ }
                final JsonObject response = result;
                final String failure = error;
                if (!plugin.isEnabled()) return;
                Bukkit.getScheduler().runTask(plugin, () -> {
                    running.remove(id);
                    if (response != null) job.add("result", response);
                    if (failure != null) job.addProperty("error", failure);
                    try { save(); } catch (Exception e) { plugin.getLogger().severe("Could not persist economy result: " + e.getMessage()); return; }
                    if (response != null || failure != null) deliver(id, job);
                });
            });
        }
    }

    private void deliver(String id, JsonObject job) {
        Player player = Bukkit.getPlayer(UUID.fromString(job.get("player").getAsString()));
        if (player == null || !player.isOnline()) return;
        try {
            List<ItemStack> items = new ArrayList<>();
            if (job.has("error")) {
                for (JsonElement encoded : job.getAsJsonArray("escrow")) items.add(decode(encoded.getAsString()));
            } else if (job.has("purchase")) {
                items.add(decode(job.get("purchase").getAsString()));
            } else if (job.has("result") && job.getAsJsonObject("result").has("items") && job.getAsJsonObject("result").get("items").isJsonArray()) {
                // Mailbox collections and cancelled listings come back as a list of stacks.
                for (JsonElement e : job.getAsJsonObject("result").getAsJsonArray("items")) {
                    JsonObject it = e.getAsJsonObject();
                    if (it.has("item_data") && !it.get("item_data").isJsonNull()) items.add(decode(it.get("item_data").getAsString()));
                    else items.add(new ItemStack(org.bukkit.Material.valueOf(it.get("item_id").getAsString()), it.get("amount").getAsInt()));
                }
            } else if (job.get("endpoint").getAsString().equals("economy/market/buy")) {
                JsonObject result = job.getAsJsonObject("result");
                if (result.has("item_data") && !result.get("item_data").isJsonNull()) items.add(decode(result.get("item_data").getAsString()));
                else items.add(new ItemStack(org.bukkit.Material.valueOf(result.get("item_id").getAsString()), result.get("amount").getAsInt()));
            }
            for (ItemStack item : items) give(player, item);
            jobs.remove(id);
            save();
            if (job.has("error")) {
                player.sendMessage(ChatColor.RED + "Transaction rejected; held items returned: " + job.get("error").getAsString());
            } else {
                JsonObject result = job.getAsJsonObject("result");
                if (result.has("message") && result.get("message").isJsonPrimitive()) {
                    // Guild bank operations explain themselves ("Deposited $50.00 into [IRON] bank.").
                    JsonElement balanceValue = result.get("balance");
                    String bank = balanceValue != null && balanceValue.isJsonPrimitive()
                            ? (job.get("endpoint").getAsString().startsWith("guilds/") ? " Bank: $" : " Balance: $") + String.format(Locale.US, "%,.2f", balanceValue.getAsDouble()) : "";
                    player.sendMessage(ChatColor.GREEN + result.get("message").getAsString() + bank);
                } else {
                    JsonElement balanceValue = result.has("new_balance") ? result.get("new_balance") : result.get("balance");
                    String balance = balanceValue != null ? " Balance: $" + String.format(Locale.US, "%,.2f", balanceValue.getAsDouble()) : "";
                    player.sendMessage(ChatColor.GREEN + "Transaction completed." + balance);
                }
            }
        } catch (Exception e) { plugin.getLogger().severe("Economy delivery requires attention for " + id + ": " + e.getMessage()); }
    }
}
