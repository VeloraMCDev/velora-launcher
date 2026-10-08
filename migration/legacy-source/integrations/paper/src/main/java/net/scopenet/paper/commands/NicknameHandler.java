package net.scopenet.paper.commands;

import org.bukkit.ChatColor;
import org.bukkit.command.Command;
import org.bukkit.command.CommandExecutor;
import org.bukkit.command.CommandSender;
import org.bukkit.configuration.file.YamlConfiguration;
import org.bukkit.entity.Player;
import org.bukkit.event.EventHandler;
import org.bukkit.event.Listener;
import org.bukkit.event.player.PlayerJoinEvent;
import org.bukkit.plugin.Plugin;

import java.io.File;
import java.io.IOException;

/** Player-owned chat nicknames, keyed by UUID and restored on every join. */
public final class NicknameHandler implements CommandExecutor, Listener {
    private final Plugin plugin;
    private final File file;
    private final YamlConfiguration config;

    public NicknameHandler(Plugin plugin) {
        this.plugin = plugin;
        this.file = new File(plugin.getDataFolder(), "nicknames.yml");
        this.config = YamlConfiguration.loadConfiguration(file);
    }

    @EventHandler public void join(PlayerJoinEvent event) {
        Player player = event.getPlayer();
        String saved = config.getString(player.getUniqueId().toString());
        if (saved != null && !saved.isBlank()) player.setDisplayName(ChatColor.translateAlternateColorCodes('&', saved));
    }

    @Override public boolean onCommand(CommandSender sender, Command command, String label, String[] args) {
        if (!(sender instanceof Player player)) { sender.sendMessage("Only players can set a nickname."); return true; }
        if (args.length != 1) { player.sendMessage(ChatColor.YELLOW + "Usage: /nick <name|off>"); return true; }
        String value = args[0];
        if (value.equalsIgnoreCase("off")) {
            config.set(player.getUniqueId().toString(), null);
            player.setDisplayName(player.getName());
            save();
            player.sendMessage(ChatColor.GREEN + "Nickname removed.");
            return true;
        }
        String plain = ChatColor.stripColor(ChatColor.translateAlternateColorCodes('&', value));
        if (plain.length() < 2 || plain.length() > 24 || !plain.matches("[A-Za-z0-9_ -]+")) {
            player.sendMessage(ChatColor.RED + "Nicknames must be 2–24 letters, numbers, spaces, _ or -.");
            return true;
        }
        if (!player.hasPermission("scopenet.chat.color")) value = plain;
        config.set(player.getUniqueId().toString(), value);
        player.setDisplayName(ChatColor.translateAlternateColorCodes('&', value));
        save();
        player.sendMessage(ChatColor.GREEN + "Nickname set to " + player.getDisplayName() + ChatColor.GREEN + ".");
        return true;
    }

    private void save() {
        try { config.save(file); }
        catch (IOException e) { plugin.getLogger().warning("Failed to save nicknames: " + e.getMessage()); }
    }
}
