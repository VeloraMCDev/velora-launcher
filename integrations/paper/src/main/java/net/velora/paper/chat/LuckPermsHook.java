package net.velora.paper.chat;

import org.bukkit.Bukkit;
import org.bukkit.entity.Player;

import java.util.UUID;

public final class LuckPermsHook {
    private final boolean available;

    public LuckPermsHook() {
        boolean present = false;
        try {
            if (Bukkit.getPluginManager().getPlugin("LuckPerms") != null) {
                Class.forName("net.luckperms.api.LuckPermsProvider");
                present = true;
            }
        } catch (Throwable ignored) {
            present = false;
        }
        this.available = present;
    }

    public boolean isAvailable() {
        return available;
    }

    public String getPrefix(Player player) {
        if (!available || player == null) return "";
        try {
            var lp = net.luckperms.api.LuckPermsProvider.get();
            var user = lp.getUserManager().getUser(player.getUniqueId());
            if (user != null) {
                String prefix = user.getCachedData().getMetaData().getPrefix();
                if (prefix != null && !prefix.isBlank()) {
                    return prefix.stripTrailing() + " ";
                }
            }
        } catch (Throwable ignored) {}
        return "";
    }

    public String getSuffix(Player player) {
        if (!available || player == null) return "";
        try {
            var lp = net.luckperms.api.LuckPermsProvider.get();
            var user = lp.getUserManager().getUser(player.getUniqueId());
            if (user != null) {
                String suffix = user.getCachedData().getMetaData().getSuffix();
                if (suffix != null && !suffix.isBlank()) {
                    return " " + suffix.stripLeading();
                }
            }
        } catch (Throwable ignored) {}
        return "";
    }

    public String getPrimaryGroup(Player player) {
        if (!available || player == null) return "";
        try {
            var lp = net.luckperms.api.LuckPermsProvider.get();
            var user = lp.getUserManager().getUser(player.getUniqueId());
            if (user != null) {
                String group = user.getPrimaryGroup();
                return group != null ? group : "";
            }
        } catch (Throwable ignored) {}
        return "";
    }
}
