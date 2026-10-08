package net.scopenet.paper.commands;

import net.md_5.bungee.api.chat.ClickEvent;
import net.md_5.bungee.api.chat.ComponentBuilder;
import net.md_5.bungee.api.chat.HoverEvent;
import net.md_5.bungee.api.chat.TextComponent;
import org.bukkit.Bukkit;
import org.bukkit.ChatColor;
import org.bukkit.Material;
import org.bukkit.command.Command;
import org.bukkit.command.CommandExecutor;
import org.bukkit.command.CommandSender;
import org.bukkit.entity.Player;
import org.bukkit.event.EventHandler;
import org.bukkit.event.Listener;
import org.bukkit.event.inventory.InventoryClickEvent;
import org.bukkit.event.inventory.InventoryDragEvent;
import org.bukkit.inventory.Inventory;
import org.bukkit.inventory.ItemStack;
import org.bukkit.plugin.Plugin;

import java.util.Map;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;

/** Share a read-only snapshot of the held item through a clickable chat message. */
public final class HandHandler implements CommandExecutor, Listener {
    private record Shared(ItemStack item, long expires) {}
    private final Map<String, Shared> shares = new ConcurrentHashMap<>();
    private static final String TITLE = ChatColor.DARK_GRAY + "Shared item";

    public HandHandler(Plugin plugin) {
        Bukkit.getScheduler().runTaskTimer(plugin, () -> shares.entrySet().removeIf(e -> e.getValue().expires < System.currentTimeMillis()), 1200, 1200);
    }

    @Override public boolean onCommand(CommandSender sender, Command command, String label, String[] args) {
        if (!(sender instanceof Player player)) { sender.sendMessage("Only players can use this command."); return true; }
        if (command.getName().equalsIgnoreCase("handview")) {
            if (args.length != 1) return false;
            Shared shared = shares.get(args[0]);
            if (shared == null || shared.expires < System.currentTimeMillis()) { player.sendMessage(ChatColor.RED + "That item share has expired."); return true; }
            Inventory view = Bukkit.createInventory(null, 9, TITLE);
            view.setItem(4, shared.item.clone());
            player.openInventory(view);
            return true;
        }
        ItemStack item = player.getInventory().getItemInMainHand();
        if (item.getType() == Material.AIR) { player.sendMessage(ChatColor.RED + "Hold an item first."); return true; }
        String token = UUID.randomUUID().toString().substring(0, 8);
        shares.put(token, new Shared(item.clone(), System.currentTimeMillis() + 300_000));
        String name = item.hasItemMeta() && item.getItemMeta().hasDisplayName()
                ? item.getItemMeta().getDisplayName() : item.getType().name().toLowerCase().replace('_', ' ');
        TextComponent link = new TextComponent("[" + name + " ×" + item.getAmount() + "]");
        link.setColor(net.md_5.bungee.api.ChatColor.AQUA);
        link.setClickEvent(new ClickEvent(ClickEvent.Action.RUN_COMMAND, "/handview " + token));
        link.setHoverEvent(new HoverEvent(HoverEvent.Action.SHOW_TEXT, new ComponentBuilder("Click to inspect " + name + " (5 minutes)").create()));
        TextComponent line = new TextComponent(player.getDisplayName() + ChatColor.WHITE + " is holding ");
        line.addExtra(link);
        for (Player target : Bukkit.getOnlinePlayers()) {
            target.spigot().sendMessage(line);
        }
        return true;
    }

    @EventHandler public void onInventoryClick(InventoryClickEvent event) {
        if (TITLE.equals(event.getView().getTitle())) event.setCancelled(true);
    }
    @EventHandler public void onInventoryDrag(InventoryDragEvent event) {
        if (TITLE.equals(event.getView().getTitle())) event.setCancelled(true);
    }
}
