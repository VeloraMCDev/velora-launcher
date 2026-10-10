package net.scopenet.paper.commands;

import net.scopenet.paper.gui.GuiHelper;
import org.bukkit.Material;
import org.bukkit.enchantments.Enchantment;
import org.bukkit.entity.Player;
import org.bukkit.inventory.Inventory;
import org.bukkit.inventory.InventoryHolder;
import org.bukkit.inventory.ItemFlag;
import org.bukkit.inventory.ItemStack;
import org.bukkit.inventory.meta.ItemMeta;
import org.bukkit.NamespacedKey;

import java.util.Locale;

/**
 * The point-and-click way to sell: hold an item, run {@code /market sell}, and pick Buy Now or Auction, the price and (for
 * auctions) how long it runs. No command arguments to remember.
 */
final class MarketWizard {
    static final String TITLE = "§6Sell on the market";
    private static final int[] DELTAS = {-1000, -100, -10, -1, 0, 1, 10, 100, 1000};
    private static final int[] HOURS = {1, 6, 24, 72};
    private static final int SLOT_BUY_NOW = 18, SLOT_AUCTION = 19, SLOT_CONFIRM = 25, SLOT_CANCEL = 26, SLOT_HOURS = 21;

    /** What the player has chosen so far. */
    static final class Holder implements InventoryHolder {
        final boolean asGuild;
        final ItemStack item;
        boolean auction;
        double price;
        int hours = 24;
        Inventory inventory;
        Holder(boolean asGuild, ItemStack item, boolean auction, double price) {
            this.asGuild = asGuild; this.item = item; this.auction = auction; this.price = price;
        }
        @Override public Inventory getInventory() { return inventory; }
    }

    interface Seller { void list(Player player, ItemStack item, double price, boolean auction, int hours, boolean asGuild); }

    private final Seller seller;

    MarketWizard(Seller seller) { this.seller = seller; }

    void open(Player player, ItemStack hand, boolean auction, double startPrice, boolean asGuild) {
        Holder holder = new Holder(asGuild, hand.clone(), auction, startPrice);
        Inventory inv = org.bukkit.Bukkit.createInventory(holder, 27, TITLE);
        holder.inventory = inv;
        draw(holder);
        player.openInventory(inv);
    }

    private static String money(double v) { return "$" + String.format(Locale.US, "%,.2f", v); }

    private static ItemStack marked(ItemStack stack, boolean on) {
        if (!on) return stack;
        ItemMeta meta = stack.getItemMeta();
        if (meta == null) return stack;
        Enchantment glint = Enchantment.getByKey(NamespacedKey.minecraft("unbreaking"));
        if (glint != null) meta.addEnchant(glint, 1, true);
        meta.addItemFlags(ItemFlag.HIDE_ENCHANTS);
        stack.setItemMeta(meta);
        return stack;
    }

    private void draw(Holder h) {
        Inventory inv = h.inventory;
        ItemStack border = GuiHelper.createBorder(Material.GRAY_STAINED_GLASS_PANE);
        for (int i = 0; i < inv.getSize(); i++) inv.setItem(i, border);
        inv.setItem(4, h.item.clone());
        for (int i = 0; i < DELTAS.length; i++) {
            int d = DELTAS[i];
            if (d == 0) {
                inv.setItem(9 + i, GuiHelper.createItem(h.auction ? Material.CLOCK : Material.GOLD_INGOT, (h.auction ? "&bStarting bid: &a" : "&6Price: &a") + money(h.price),
                        "&7Use the buttons to change it,", "&7or type &e/market " + (h.auction ? "auction" : "sell") + " <price>"));
            } else {
                inv.setItem(9 + i, GuiHelper.createItem(d < 0 ? Material.RED_STAINED_GLASS_PANE : Material.LIME_STAINED_GLASS_PANE, (d < 0 ? "&c" : "&a") + (d < 0 ? "-" : "+") + Math.abs(d)));
            }
        }
        inv.setItem(SLOT_BUY_NOW, marked(GuiHelper.createItem(Material.GOLD_INGOT, "&6Buy Now",
                "&7Anyone can buy it instantly", "&7for your price.", "", h.auction ? "&eClick to choose" : "&a✔ Selected"), !h.auction));
        inv.setItem(SLOT_AUCTION, marked(GuiHelper.createItem(Material.CLOCK, "&bAuction",
                "&7Players bid against each other.", "&7Highest bid wins when time is up.", "", h.auction ? "&a✔ Selected" : "&eClick to choose"), h.auction));
        for (int i = 0; i < HOURS.length; i++) {
            inv.setItem(SLOT_HOURS + i, h.auction
                    ? marked(GuiHelper.createItem(Material.PAPER, HOURS[i], "&f" + HOURS[i] + (HOURS[i] == 1 ? " hour" : " hours"), h.hours == HOURS[i] ? "&a✔ Selected" : "&eClick to choose"), h.hours == HOURS[i])
                    : border);
        }
        inv.setItem(SLOT_CONFIRM, GuiHelper.createItem(Material.LIME_CONCRETE, "&a&lConfirm",
                h.auction ? "&7Auction for &f" + h.hours + "h &7from &a" + money(h.price) : "&7Sell for &a" + money(h.price),
                h.asGuild ? "&7Paid into your guild's bank." : "&7The item is held safely until it sells."));
        inv.setItem(SLOT_CANCEL, GuiHelper.createItem(Material.RED_CONCRETE, "&c&lCancel"));
    }

    /** Handles a click in the wizard. The event is already cancelled by the caller. */
    void click(Player player, Holder h, int slot) {
        if (slot >= 9 && slot < 18) {
            int d = DELTAS[slot - 9];
            if (d != 0) h.price = Math.max(1, Math.min(1_000_000_000, h.price + d));
            draw(h);
        } else if (slot == SLOT_BUY_NOW) { h.auction = false; draw(h); }
        else if (slot == SLOT_AUCTION) { h.auction = true; draw(h); }
        else if (h.auction && slot >= SLOT_HOURS && slot < SLOT_HOURS + HOURS.length) { h.hours = HOURS[slot - SLOT_HOURS]; draw(h); }
        else if (slot == SLOT_CANCEL) player.closeInventory();
        else if (slot == SLOT_CONFIRM) {
            player.closeInventory();
            ItemStack now = player.getInventory().getItemInMainHand();
            if (now.getType().isAir() || !now.isSimilar(h.item) || now.getAmount() != h.item.getAmount()) {
                player.sendMessage("§cYou're no longer holding that item. Hold it again and run /market sell.");
                return;
            }
            seller.list(player, now.clone(), h.price, h.auction, h.hours, h.asGuild);
        }
    }
}
