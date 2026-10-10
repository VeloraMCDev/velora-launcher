package net.scopenet.paper;

import net.scopenet.core.Pos;
import net.scopenet.core.ShopPoints;
import org.bukkit.Bukkit;
import org.bukkit.Location;
import org.bukkit.NamespacedKey;
import org.bukkit.command.Command;
import org.bukkit.command.CommandExecutor;
import org.bukkit.command.CommandSender;
import org.bukkit.command.TabCompleter;
import org.bukkit.entity.Entity;
import org.bukkit.entity.EntityType;
import org.bukkit.entity.LivingEntity;
import org.bukkit.entity.Mob;
import org.bukkit.entity.Player;
import org.bukkit.entity.Villager;
import org.bukkit.event.EventHandler;
import org.bukkit.event.EventPriority;
import org.bukkit.event.Listener;
import org.bukkit.event.entity.EntityDamageEvent;
import org.bukkit.event.player.PlayerInteractEntityEvent;
import org.bukkit.event.entity.PlayerLeashEntityEvent;
import org.bukkit.inventory.EquipmentSlot;
import org.bukkit.persistence.PersistentDataType;
import org.bukkit.util.RayTraceResult;

import java.util.ArrayList;
import java.util.List;
import java.util.Locale;
import java.util.Optional;
import java.util.UUID;
import java.util.function.BiConsumer;

/**
 * Shopkeepers: a villager (or any mob) that opens the market or server shop when right-clicked. Making a mob a shopkeeper
 * removes its AI, makes it invulnerable, silent and stationary, and drops a pin on the map. {@code /shopkeeper spawn} makes
 * a new one; {@code /shopkeeper link} turns the mob you are looking at into one.
 */
final class ShopkeeperHandler implements CommandExecutor, TabCompleter, Listener {
    static final String NODE = ShopPoints.NODE;
    private final ScopenetPlugin plugin;
    private final ShopPoints shops;
    private final BiConsumer<Player, String> opener;
    private final NamespacedKey key;

    ShopkeeperHandler(ScopenetPlugin plugin, ShopPoints shops, BiConsumer<Player, String> opener) {
        this.plugin = plugin;
        this.shops = shops;
        this.opener = opener;
        this.key = new NamespacedKey(plugin, "shopkeeper");
        // Keep the map pins on the mob when it has been pushed or moved by something else.
        Bukkit.getScheduler().runTaskTimer(plugin, this::syncPins, 200L, 20L * 60);
    }

    private boolean isKeeper(Entity e) { return e.getPersistentDataContainer().has(key, PersistentDataType.STRING); }

    /** Turns a living entity into a shopkeeper. */
    private void configure(LivingEntity e, String name, String kind) {
        e.setAI(false);
        e.setInvulnerable(true);
        e.setSilent(true);
        e.setPersistent(true);
        e.setRemoveWhenFarAway(false);
        e.setCollidable(false);
        e.setCanPickupItems(false);
        e.setCustomName("§e§l" + name.replace('_', ' '));
        e.setCustomNameVisible(true);
        if (e instanceof Mob mob) mob.setTarget(null);
        if (e instanceof Villager v) v.setProfession(Villager.Profession.LIBRARIAN);
        e.getPersistentDataContainer().set(key, PersistentDataType.STRING, name + ":" + kind);
    }

    /** Gives a mob back to the world as it was. */
    private void release(LivingEntity e) {
        e.getPersistentDataContainer().remove(key);
        e.setAI(true);
        e.setInvulnerable(false);
        e.setSilent(false);
        e.setCustomNameVisible(false);
        e.setCustomName(null);
        e.setCollidable(true);
    }

    private static Pos pos(Location l) {
        return new Pos(ScopenetPlugin.dimension(l.getWorld()), l.getX(), l.getY(), l.getZ(), l.getYaw(), l.getPitch());
    }

    private LivingEntity lookedAt(Player p) {
        RayTraceResult hit = p.getWorld().rayTraceEntities(p.getEyeLocation(), p.getEyeLocation().getDirection(), 6, 0.3,
                e -> e instanceof LivingEntity && !(e instanceof Player) && !e.equals(p));
        return hit == null || !(hit.getHitEntity() instanceof LivingEntity living) ? null : living;
    }

    @Override public boolean onCommand(CommandSender sender, Command command, String label, String[] args) {
        if (!(sender instanceof Player p)) { sender.sendMessage("Run this in game."); return true; }
        if (!p.hasPermission(NODE)) { p.sendMessage("§cYou do not have permission to manage shopkeepers."); return true; }
        String sub = args.length == 0 ? "help" : args[0].toLowerCase(Locale.ROOT);
        switch (sub) {
            case "spawn" -> {
                if (args.length < 2) { p.sendMessage("§cUsage: /shopkeeper spawn <name> [market|shop] [mob type]"); return true; }
                String kind = args.length > 2 ? args[2].toLowerCase(Locale.ROOT) : "market";
                EntityType type = EntityType.VILLAGER;
                if (args.length > 3) {
                    try { type = EntityType.valueOf(args[3].toUpperCase(Locale.ROOT)); } catch (IllegalArgumentException e) { p.sendMessage("§cUnknown mob type '" + args[3] + "'. Try villager, zombie, iron_golem, allay…"); return true; }
                }
                if (type.getEntityClass() == null || !LivingEntity.class.isAssignableFrom(type.getEntityClass()) || type == EntityType.PLAYER || type == EntityType.ARMOR_STAND) {
                    p.sendMessage("§cThat can't be a shopkeeper. Pick a living mob."); return true;
                }
                String problem = shops.named(args[1]).isPresent() ? "There is already a shop point or shopkeeper called '" + args[1] + "'." : null;
                if (problem != null) { p.sendMessage("§c" + problem); return true; }
                Location at = p.getLocation();
                Entity spawned = p.getWorld().spawnEntity(at, type);
                if (!(spawned instanceof LivingEntity living)) { spawned.remove(); p.sendMessage("§cThat can't be a shopkeeper."); return true; }
                String error = shops.addNpc(args[1], kind, pos(at), living.getUniqueId().toString());
                if (error != null) { living.remove(); p.sendMessage("§c" + error); return true; }
                configure(living, args[1], kind);
                p.sendMessage("§aShopkeeper §e" + args[1] + "§a is open for business (" + kind + "). Right-click them to trade; they're marked on the map.");
            }
            case "link" -> {
                if (args.length < 2) { p.sendMessage("§cUsage: /shopkeeper link <name> [market|shop] - look at the mob first"); return true; }
                LivingEntity target = lookedAt(p);
                if (target == null) { p.sendMessage("§cLook straight at the mob you want to link (within 6 blocks)."); return true; }
                if (isKeeper(target)) { p.sendMessage("§cThat mob is already a shopkeeper."); return true; }
                String kind = args.length > 2 ? args[2].toLowerCase(Locale.ROOT) : "market";
                String error = shops.addNpc(args[1], kind, pos(target.getLocation()), target.getUniqueId().toString());
                if (error != null) { p.sendMessage("§c" + error); return true; }
                configure(target, args[1], kind);
                p.sendMessage("§aLinked! §e" + args[1] + "§a now opens the " + kind + " when right-clicked, and no longer moves or takes damage.");
            }
            case "remove", "delete" -> {
                if (args.length < 2) { p.sendMessage("§cUsage: /shopkeeper remove <name> [kill]"); return true; }
                Optional<ShopPoints.Shop> gone = shops.named(args[1]).filter(ShopPoints.Shop::isNpc);
                if (gone.isEmpty()) { p.sendMessage("§cNo shopkeeper called '" + args[1] + "'."); return true; }
                shops.removeNamed(args[1]);
                Entity e = entityOf(gone.get());
                if (e instanceof LivingEntity living) {
                    if (args.length > 2 && args[2].equalsIgnoreCase("kill")) living.remove(); else release(living);
                }
                p.sendMessage("§aShopkeeper §e" + args[1] + "§a removed" + (e == null ? " (their mob wasn't loaded, so it may still stand there)." : "."));
            }
            case "tp" -> {
                Optional<ShopPoints.Shop> shop = args.length < 2 ? Optional.empty() : shops.named(args[1]).filter(ShopPoints.Shop::isNpc);
                if (shop.isEmpty()) { p.sendMessage("§cUsage: /shopkeeper tp <name>"); return true; }
                Entity e = entityOf(shop.get());
                p.teleport(e != null ? e.getLocation() : toLocation(shop.get().pos()));
            }
            case "list" -> {
                p.sendMessage("§6=== Shopkeepers ===");
                int n = 0;
                for (ShopPoints.Shop s : shops.all()) {
                    if (!s.isNpc()) continue;
                    n++;
                    p.sendMessage("§e" + s.name() + "§7 (" + s.kind() + ") " + s.pos().blockX() + ", " + s.pos().blockY() + ", " + s.pos().blockZ());
                }
                if (n == 0) p.sendMessage("§7None yet. Try §e/shopkeeper spawn Bazaar market");
            }
            default -> {
                p.sendMessage("§6Shopkeepers §7- villagers (or any mob) that open the market");
                p.sendMessage(" §e/shopkeeper spawn <name> [market|shop] [mob]§7  create one where you stand");
                p.sendMessage(" §e/shopkeeper link <name> [market|shop]§7  turn the mob you look at into one");
                p.sendMessage(" §e/shopkeeper remove <name> [kill]  §7free (or remove) the mob");
                p.sendMessage(" §e/shopkeeper list§7, §e/shopkeeper tp <name>");
            }
        }
        return true;
    }

    private Location toLocation(Pos pos) {
        for (org.bukkit.World w : Bukkit.getWorlds()) {
            if (ScopenetPlugin.dimension(w).equals(pos.world())) return new Location(w, pos.x(), pos.y(), pos.z(), pos.yaw(), pos.pitch());
        }
        return Bukkit.getWorlds().get(0).getSpawnLocation();
    }

    private Entity entityOf(ShopPoints.Shop shop) {
        try { return Bukkit.getEntity(UUID.fromString(shop.entity())); } catch (IllegalArgumentException e) { return null; }
    }

    private void syncPins() {
        for (ShopPoints.Shop s : shops.all()) {
            if (!s.isNpc()) continue;
            Entity e = entityOf(s);
            if (e == null || !e.isValid()) continue;
            Pos now = pos(e.getLocation());
            if (!now.world().equals(s.pos().world()) || Math.abs(now.x() - s.pos().x()) > 1.5 || Math.abs(now.z() - s.pos().z()) > 1.5 || Math.abs(now.y() - s.pos().y()) > 2) {
                shops.moveNpc(s.name(), now);
            }
        }
    }

    @Override public List<String> onTabComplete(CommandSender sender, Command command, String alias, String[] args) {
        List<String> out = new ArrayList<>();
        if (args.length == 1) out.addAll(List.of("spawn", "link", "remove", "list", "tp"));
        else if (args.length == 2 && List.of("remove", "tp").contains(args[0].toLowerCase(Locale.ROOT))) shops.all().stream().filter(ShopPoints.Shop::isNpc).forEach(s -> out.add(s.name()));
        else if (args.length == 3 && List.of("spawn", "link").contains(args[0].toLowerCase(Locale.ROOT))) out.addAll(List.of("market", "shop"));
        else if (args.length == 4 && args[0].equalsIgnoreCase("spawn")) out.addAll(List.of("villager", "wandering_trader", "iron_golem", "allay", "zombie", "piglin", "witch", "fox", "cat"));
        String last = args.length == 0 ? "" : args[args.length - 1].toLowerCase(Locale.ROOT);
        out.removeIf(s -> !s.toLowerCase(Locale.ROOT).startsWith(last));
        return out;
    }

    // ---- events -----------------------------------------------------------------------------------------

    @EventHandler(priority = EventPriority.HIGH)
    public void interact(PlayerInteractEntityEvent event) {
        if (!isKeeper(event.getRightClicked())) return;
        event.setCancelled(true); // no villager trade window, no name tags, no feeding
        if (event.getHand() != EquipmentSlot.HAND) return;
        Optional<ShopPoints.Shop> shop = shops.byEntity(event.getRightClicked().getUniqueId().toString());
        String kind = shop.map(ShopPoints.Shop::kind).orElseGet(() -> {
            String tag = event.getRightClicked().getPersistentDataContainer().get(key, PersistentDataType.STRING);
            return tag != null && tag.endsWith(":shop") ? "shop" : "market";
        });
        opener.accept(event.getPlayer(), kind);
    }

    @EventHandler(priority = EventPriority.HIGH)
    public void damage(EntityDamageEvent event) {
        if (isKeeper(event.getEntity())) event.setCancelled(true);
    }

    @EventHandler(priority = EventPriority.HIGH)
    public void leash(PlayerLeashEntityEvent event) {
        if (isKeeper(event.getEntity())) event.setCancelled(true);
    }
}
