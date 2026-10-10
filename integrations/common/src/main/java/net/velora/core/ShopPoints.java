package net.velora.core;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.google.gson.reflect.TypeToken;

import java.io.IOException;
import java.io.Reader;
import java.io.Writer;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.util.*;
import java.util.concurrent.ConcurrentHashMap;

/**
 * Physical shops: a block in the world that opens the market or the server shop when a player right-clicks it, and
 * shows up as a pin on the map. The GUI itself is the same as typing the command, so it works on Paper (chest window)
 * and with the client mod (shop and market windows). Set up by staff with {@code /market point add|remove|list}.
 */
public final class ShopPoints {
    /** A shop point: a block, or (when {@code entity} is set) an NPC the market opens from. */
    public record Shop(String name, String kind, Pos pos, String entity) {
        public Shop(String name, String kind, Pos pos) { this(name, kind, pos, ""); }
        public boolean isNpc() { return entity != null && !entity.isBlank(); }
    }

    public static final String NODE = "velora.command.market.point";
    private static final Gson GSON = new GsonBuilder().setPrettyPrinting().create();

    private final Path file;
    private final Map<String, Shop> shops = new ConcurrentHashMap<>();

    public ShopPoints(Path file) {
        this.file = file;
        if (Files.exists(file)) {
            try (Reader in = Files.newBufferedReader(file)) {
                List<Shop> loaded = GSON.fromJson(in, new TypeToken<List<Shop>>() {}.getType());
                if (loaded != null) loaded.forEach(s -> shops.put(s.name().toLowerCase(Locale.ROOT), s));
            } catch (IOException | RuntimeException e) {
                throw new IllegalStateException("Could not read " + file + ": " + e.getMessage(), e);
            }
        }
    }

    public Collection<Shop> all() { return new ArrayList<>(new TreeMap<>(shops).values()); }

    /** The shop at this block, if any. */
    public Optional<Shop> at(String dimension, int x, int y, int z) {
        for (Shop s : shops.values()) {
            Pos p = s.pos();
            if (p.world().equals(dimension) && p.blockX() == x && p.blockY() == y && p.blockZ() == z) return Optional.of(s);
        }
        return Optional.empty();
    }

    /** The shopkeeper NPC linked to this entity, if any. */
    public Optional<Shop> byEntity(String entityUuid) {
        for (Shop s : shops.values()) if (s.isNpc() && s.entity().equalsIgnoreCase(entityUuid)) return Optional.of(s);
        return Optional.empty();
    }

    public Optional<Shop> named(String name) { return Optional.ofNullable(shops.get(name.toLowerCase(Locale.ROOT))); }

    /** Registers an NPC. Returns an error message, or null on success. */
    public String addNpc(String name, String kind, Pos pos, String entityUuid) {
        if (!name.matches("[A-Za-z0-9_-]{1,24}")) return "Names are 1-24 letters, digits, - or _.";
        if (!kind.equals("market") && !kind.equals("shop")) return "The kind is market or shop.";
        if (shops.containsKey(name.toLowerCase(Locale.ROOT))) return "There is already a shop point or shopkeeper called '" + name + "'.";
        if (byEntity(entityUuid).isPresent()) return "That mob is already a shopkeeper.";
        shops.put(name.toLowerCase(Locale.ROOT), new Shop(name, kind, pos, entityUuid));
        save();
        return null;
    }

    /** Moves a shopkeeper's pin when its mob has been moved. */
    public void moveNpc(String name, Pos pos) {
        Shop old = shops.get(name.toLowerCase(Locale.ROOT));
        if (old == null || !old.isNpc()) return;
        shops.put(name.toLowerCase(Locale.ROOT), new Shop(old.name(), old.kind(), pos, old.entity()));
        save();
    }

    public Optional<Shop> removeNamed(String name) {
        Shop gone = shops.remove(name.toLowerCase(Locale.ROOT));
        if (gone != null) save();
        return Optional.ofNullable(gone);
    }

    private void save() {
        try {
            if (file.getParent() != null) Files.createDirectories(file.getParent());
            Path tmp = file.resolveSibling(file.getFileName() + ".tmp");
            try (Writer w = Files.newBufferedWriter(tmp)) { GSON.toJson(all(), w); }
            Files.move(tmp, file, StandardCopyOption.REPLACE_EXISTING);
        } catch (IOException e) {
            throw new IllegalStateException("Could not save " + file + ": " + e.getMessage(), e);
        }
    }

    /**
     * {@code /market point add <name> [market|shop]} marks the block the player is looking at... or, where the platform can't
     * tell, the block they stand on. {@code target} is that block. {@code remove <name>} and {@code list} as expected.
     */
    public void command(CorePlayer p, String[] args, Pos target) {
        if (!p.hasPermission(NODE)) { p.send(Format.RED + "You do not have permission to manage shop points."); return; }
        String sub = args.length == 0 ? "list" : args[0].toLowerCase(Locale.ROOT);
        switch (sub) {
            case "add" -> {
                if (args.length < 2) { p.send(Format.RED + "Usage: /market point add <name> [market|shop]"); return; }
                String name = args[1];
                if (!name.matches("[A-Za-z0-9_-]{1,24}")) { p.send(Format.RED + "Names are 1-24 letters, digits, - or _."); return; }
                String kind = args.length > 2 ? args[2].toLowerCase(Locale.ROOT) : "market";
                if (!kind.equals("market") && !kind.equals("shop")) { p.send(Format.RED + "The kind is market or shop."); return; }
                if (at(target.world(), target.blockX(), target.blockY(), target.blockZ()).isPresent()) { p.send(Format.RED + "There is already a shop point on that block."); return; }
                Pos block = new Pos(target.world(), target.blockX() + 0.5, target.blockY(), target.blockZ() + 0.5, 0, 0);
                shops.put(name.toLowerCase(Locale.ROOT), new Shop(name, kind, block));
                save();
                p.send(Format.GREEN + "Shop point " + Format.YELLOW + name + Format.GREEN + " set (" + kind + "). Right-click that block to open the "
                        + (kind.equals("market") ? "market" : "shop") + ".");
            }
            case "remove", "delete" -> {
                if (args.length < 2) { p.send(Format.RED + "Usage: /market point remove <name>"); return; }
                if (shops.remove(args[1].toLowerCase(Locale.ROOT)) == null) { p.send(Format.RED + "No shop point called '" + args[1] + "'."); return; }
                save();
                p.send(Format.GREEN + "Shop point " + Format.YELLOW + args[1] + Format.GREEN + " removed.");
            }
            case "list" -> {
                p.send(Format.GOLD + "=== Shop points ===");
                if (shops.isEmpty()) p.send(Format.GRAY + "None yet. Look at a block and use /market point add <name>.");
                for (Shop s : all()) p.send(Format.YELLOW + s.name() + Format.GRAY + " (" + s.kind() + ") " + s.pos().blockX() + ", " + s.pos().blockY() + ", " + s.pos().blockZ());
            }
            default -> p.send(Format.RED + "Usage: /market point add <name> [market|shop] | remove <name> | list");
        }
    }
}
