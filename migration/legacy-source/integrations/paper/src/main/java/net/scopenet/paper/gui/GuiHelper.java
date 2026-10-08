package net.scopenet.paper.gui;

import org.bukkit.ChatColor;
import org.bukkit.Material;
import org.bukkit.inventory.ItemStack;
import org.bukkit.inventory.meta.ItemMeta;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

public final class GuiHelper {
    private GuiHelper() {}

    public static ItemStack createItem(Material material, String name, String... lore) {
        return createItem(material, 1, name, lore);
    }

    public static ItemStack createItem(Material material, int amount, String name, String... lore) {
        ItemStack item = new ItemStack(material, Math.max(1, amount));
        ItemMeta meta = item.getItemMeta();
        if (meta != null) {
            if (name != null) {
                meta.setDisplayName(ChatColor.translateAlternateColorCodes('&', name));
            }
            if (lore != null && lore.length > 0) {
                List<String> list = new ArrayList<>();
                for (String line : lore) {
                    list.add(ChatColor.translateAlternateColorCodes('&', line));
                }
                meta.setLore(list);
            }
            item.setItemMeta(meta);
        }
        return item;
    }

    public static ItemStack createBorder(Material material) {
        return createItem(material, " ");
    }
}
