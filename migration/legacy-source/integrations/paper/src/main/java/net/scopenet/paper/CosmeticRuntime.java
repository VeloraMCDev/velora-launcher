package net.scopenet.paper;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import net.scopenet.core.ItemSpec;
import net.scopenet.core.PlayerCache;
import org.bukkit.Bukkit;
import org.bukkit.Location;
import org.bukkit.entity.ItemDisplay;
import org.bukkit.entity.Player;
import org.bukkit.inventory.ItemStack;
import org.bukkit.scheduler.BukkitTask;
import org.bukkit.util.Transformation;
import org.joml.AxisAngle4f;
import org.joml.Vector3f;

import java.util.HashMap;
import java.util.Iterator;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import java.util.function.BiFunction;

/**
 * Shows the modelled cosmetic a player has equipped (Cosmetics Studio) on their character.
 *
 * <p>The cosmetic is an {@link ItemDisplay} riding the player, wearing the cosmetic's model through the server resource pack. It is plain
 * vanilla, so every client that has the pack sees it (including the Fabric companion), and nothing is saved into the world. What a player
 * wears comes from the panel, with the rest of their profile ({@code cosmetics.equipped}); {@link #apply} is called whenever that is refreshed.
 *
 * <p>Everything here runs on the server thread.
 */
final class CosmeticRuntime {
    /** What is worn: the model, where and how big. Equal looks do not respawn anything. */
    private record Look(String key, String item, int model, String slot, float scale, float offsetY) {}

    private static final class Worn {
        final Look look;
        ItemDisplay display;
        /** The cosmetic's base item does not exist on this server; do not keep retrying. */
        boolean unusable;
        Worn(Look look) { this.look = look; }
    }

    private final ScopenetPlugin plugin;
    private final BiFunction<ItemSpec, Integer, ItemStack> stacks;
    private final Map<UUID, Worn> worn = new HashMap<>();
    private BukkitTask task;

    CosmeticRuntime(ScopenetPlugin plugin, BiFunction<ItemSpec, Integer, ItemStack> stacks) {
        this.plugin = plugin;
        this.stacks = stacks;
    }

    void start() { task = Bukkit.getScheduler().runTaskTimer(plugin, this::follow, 20, 2); }

    void stop() {
        if (task != null) task.cancel();
        task = null;
        for (Worn w : worn.values()) discard(w);
        worn.clear();
    }

    /** The player left. */
    void remove(Player player) {
        Worn w = worn.remove(player.getUniqueId());
        if (w != null) discard(w);
    }

    /** Put on (or take off, or swap) whatever {@code info}, the player's panel profile, says is equipped. */
    void apply(Player player, JsonObject info) {
        Look look = equipped(info);
        Worn current = worn.get(player.getUniqueId());
        if (look == null) { remove(player); return; }
        if (current != null && current.look.equals(look) && current.display != null && current.display.isValid()) return;
        if (current != null) discard(current);
        Worn next = new Worn(look);
        worn.put(player.getUniqueId(), next);
        spawn(player, next);
    }

    /** The modelled cosmetic in {@code info.cosmetics.equipped}, if any. */
    private static Look equipped(JsonObject info) {
        JsonObject cosmetics = PlayerCache.obj(info, "cosmetics");
        for (JsonElement e : PlayerCache.arr(cosmetics, "equipped")) {
            if (!e.isJsonObject()) continue;
            JsonObject entry = e.getAsJsonObject();
            if (!"cosmetic".equals(PlayerCache.str(entry, "type", ""))) continue;
            JsonObject meta = PlayerCache.obj(entry, "metadata");
            JsonObject look = PlayerCache.obj(meta, "look");
            int model = (int) PlayerCache.num(look, "custom_model_data", 0);
            if (look == null || model <= 0) continue;
            float scale = (float) Math.max(0.25, Math.min(3.0, PlayerCache.num(meta, "scale", 1.0)));
            float offsetY = (float) Math.max(-1.0, Math.min(1.0, PlayerCache.num(meta, "offset_y", 0.0)));
            return new Look(PlayerCache.str(entry, "key", ""), PlayerCache.str(look, "item", "minecraft:stick"), model,
                    PlayerCache.str(meta, "slot", "head"), scale, offsetY);
        }
        return null;
    }

    private void spawn(Player player, Worn w) {
        Look look = w.look;
        ItemStack stack = stacks.apply(new ItemSpec(look.item(), 1, "", List.of(), Map.of(), false, false, false, look.model(), List.of()), 1);
        if (stack == null) { w.unusable = true; return; }
        Location at = player.getLocation();
        ItemDisplay d = at.getWorld().spawn(at, ItemDisplay.class);
        d.setItemStack(stack);
        d.setItemDisplayTransform(ItemDisplay.ItemDisplayTransform.NONE);
        d.setTransformation(transformation(look));
        d.setPersistent(false);
        w.display = d;
        if (!player.addPassenger(d)) { discard(w); return; }
        // Nobody wants a hat in the middle of their own view.
        if (look.slot().equals("head")) player.hideEntity(plugin, d);
    }

    /**
     * Where the model sits relative to the rider's attachment point (the top of the player's head). These are starting values: the
     * cosmetic's own "height" setting in the Cosmetics Studio moves it up or down.
     */
    private static Transformation transformation(Look look) {
        float s = look.scale();
        float x = 0f, y, z = 0f;
        switch (look.slot()) {
            case "back" -> { y = -0.6f + look.offsetY(); z = -(0.25f + 0.25f * s); }
            case "hand" -> { x = -0.5f; y = -0.9f + look.offsetY(); z = 0.25f; }
            case "aura" -> y = -0.9f + look.offsetY();
            default -> y = 0.5f * s + look.offsetY();
        }
        return new Transformation(new Vector3f(x, y, z), new AxisAngle4f(), new Vector3f(s, s, s), new AxisAngle4f());
    }

    private static void discard(Worn w) {
        if (w.display != null) { w.display.remove(); w.display = null; }
    }

    /** Keep every display on its player: turn it with them, and put it back after a teleport or a respawn. */
    private void follow() {
        Iterator<Map.Entry<UUID, Worn>> it = worn.entrySet().iterator();
        while (it.hasNext()) {
            Map.Entry<UUID, Worn> entry = it.next();
            Player player = Bukkit.getPlayer(entry.getKey());
            Worn w = entry.getValue();
            if (player == null || !player.isOnline()) { discard(w); it.remove(); continue; }
            if (w.unusable) continue;
            ItemDisplay d = w.display;
            if (d == null || !d.isValid() || d.getWorld() != player.getWorld()) { discard(w); spawn(player, w); continue; }
            if (!player.getPassengers().contains(d)) {
                d.teleport(player.getLocation());
                if (!player.addPassenger(d)) { discard(w); spawn(player, w); continue; }
            }
            d.setRotation(player.getLocation().getYaw(), 0f);
        }
    }
}
