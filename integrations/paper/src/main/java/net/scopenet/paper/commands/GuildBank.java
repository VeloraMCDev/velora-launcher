package net.scopenet.paper.commands;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import net.scopenet.integration.Integration;
import net.scopenet.paper.ScopenetPlugin;
import org.bukkit.Bukkit;
import org.bukkit.ChatColor;
import org.bukkit.entity.Player;
import java.util.Collections;
import java.util.Locale;

/**
 * /guild bank, /guild pay: the guild's money as seen in game. Balances are
 * kept by the panel (and shown in the launcher); spending rights are checked
 * there too, so the plugin only has to pass on who is asking.
 */
final class GuildBank {
    private final ScopenetPlugin plugin;
    private final Integration integration;
    private final EconomyOperations operations;

    GuildBank(ScopenetPlugin plugin, Integration integration, EconomyOperations operations) {
        this.plugin = plugin;
        this.integration = integration;
        this.operations = operations;
    }

    private static String money(double value) {
        return "$" + String.format(Locale.US, "%,.2f", value);
    }

    private static Double amount(Player player, String text) {
        try {
            double value = Double.parseDouble(text.replace(",", ""));
            if (Double.isNaN(value) || Double.isInfinite(value) || value < 0.01) throw new NumberFormatException();
            return Math.round(value * 100.0) / 100.0;
        } catch (NumberFormatException e) {
            player.sendMessage(ChatColor.RED + "Enter an amount of at least 0.01, for example 25 or 12.50.");
            return null;
        }
    }

    /** /guild bank [deposit|withdraw <amount>] */
    void bank(Player player, String[] args) {
        if (args.length >= 2 && (args[0].equalsIgnoreCase("deposit") || args[0].equalsIgnoreCase("withdraw"))) {
            String action = args[0].toLowerCase();
            if (!player.hasPermission("scopenet.command.guild.bank." + action)) {
                player.sendMessage(ChatColor.RED + "You do not have permission to " + action + " guild money.");
                return;
            }
            Double value = amount(player, args[1]);
            if (value == null) return;
            JsonObject payload = new JsonObject();
            payload.addProperty("uuid", player.getUniqueId().toString());
            payload.addProperty("username", player.getName());
            payload.addProperty("amount", value);
            payload.addProperty("action", action);
            operations.enqueue(player, "guilds/bank/transfer", payload, Collections.emptyList(), null);
            return;
        }
        if (args.length > 0 && !args[0].equalsIgnoreCase("balance") && !args[0].equalsIgnoreCase("info")) {
            player.sendMessage(ChatColor.RED + "Usage: /guild bank [deposit|withdraw <amount>]");
            return;
        }
        Bukkit.getScheduler().runTaskAsynchronously(plugin, () -> {
            try {
                JsonObject info = integration.client().post("guilds/bank", who(player));
                if (!info.has("guild") || info.get("guild").isJsonNull()) {
                    player.sendMessage(ChatColor.RED + "You are not in a guild on this server.");
                    return;
                }
                JsonObject guild = info.getAsJsonObject("guild");
                player.sendMessage(ChatColor.GOLD + "=== [" + guild.get("tag").getAsString() + "] " + guild.get("name").getAsString() + " Bank ===");
                player.sendMessage(ChatColor.YELLOW + "Balance: " + ChatColor.GREEN + money(info.get("balance").getAsDouble())
                        + ChatColor.GRAY + "   Your wallet: " + money(info.get("my_balance").getAsDouble())
                        + "   Role: " + info.get("role").getAsString());
                JsonArray recent = info.getAsJsonArray("recent");
                for (JsonElement element : recent) {
                    JsonObject row = element.getAsJsonObject();
                    String kind = row.get("kind").getAsString();
                    boolean out = kind.equals("withdraw") || kind.equals("purchase") || kind.equals("transfer_out");
                    player.sendMessage(ChatColor.GRAY + " " + (out ? ChatColor.RED + "-" : ChatColor.GREEN + "+")
                            + money(row.get("amount").getAsDouble()) + ChatColor.GRAY + " " + kind.replace('_', ' ') + " by "
                            + row.get("who").getAsString());
                }
                player.sendMessage(ChatColor.GRAY + "/guild bank deposit|withdraw <amount>, /guild sell, /guild market, /guild pay <tag> <amount>");
            } catch (Exception e) {
                player.sendMessage(ChatColor.RED + "Could not load the guild bank: " + e.getMessage());
            }
        });
    }

    /** /guild pay <tag> <amount> */
    void pay(Player player, String[] args) {
        if (args.length < 2) {
            player.sendMessage(ChatColor.RED + "Usage: /guild pay <guild tag> <amount>");
            return;
        }
        Double value = amount(player, args[1]);
        if (value == null) return;
        JsonObject payload = new JsonObject();
        payload.addProperty("uuid", player.getUniqueId().toString());
        payload.addProperty("to_tag", args[0]);
        payload.addProperty("amount", value);
        operations.enqueue(player, "guilds/bank/pay", payload, Collections.emptyList(), null);
    }

    private static JsonObject who(Player player) {
        JsonObject body = new JsonObject();
        body.addProperty("uuid", player.getUniqueId().toString());
        return body;
    }
}
