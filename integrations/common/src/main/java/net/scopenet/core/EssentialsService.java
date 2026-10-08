package net.scopenet.core;

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
import java.util.function.LongSupplier;

/**
 * Homes, warps, /back, TPA and RTP, the same on every platform. Behaviour and wording match the Paper
 * plugin. Call from the server thread; state is saved to one JSON file after every change.
 */
public final class EssentialsService {
    private static final String GREEN = "§a", RED = "§c", YELLOW = "§e", GRAY = "§7";
    private static final Gson GSON = new GsonBuilder().setPrettyPrinting().create();

    private record Data(Map<String, Map<String, Pos>> homes, Map<String, Pos> warps) {}

    private final Platform platform;
    private final Path file;
    private final LongSupplier clock;
    private final Random rng;
    private volatile EssentialsConfig config;

    private final Map<UUID, Map<String, Pos>> homes = new ConcurrentHashMap<>();
    private final Map<String, Pos> warps = new ConcurrentHashMap<>();
    private final Map<UUID, Pos> lastLocations = new ConcurrentHashMap<>();
    /** target -> (requester, sent at) */
    private final Map<UUID, Map.Entry<UUID, Long>> pendingTpa = new ConcurrentHashMap<>();

    public EssentialsService(Platform platform, Path file, EssentialsConfig config, LongSupplier clock, Random rng) {
        this.platform = platform;
        this.file = file;
        this.config = config;
        this.clock = clock;
        this.rng = rng;
        load();
    }

    public void configure(EssentialsConfig next) { this.config = next; }

    public int cooldownSeconds(String command) { return config.cooldown(command); }

    // ---- persistence -------------------------------------------------------

    private void load() {
        if (!Files.exists(file)) return;
        try (Reader in = Files.newBufferedReader(file)) {
            Data data = GSON.fromJson(in, new TypeToken<Data>() {}.getType());
            if (data == null) return;
            if (data.homes() != null) data.homes().forEach((id, h) -> {
                try { homes.put(UUID.fromString(id), new HashMap<>(h)); } catch (IllegalArgumentException ignored) { }
            });
            if (data.warps() != null) warps.putAll(data.warps());
        } catch (IOException | RuntimeException e) {
            throw new IllegalStateException("Could not read " + file + ": " + e.getMessage(), e);
        }
    }

    private void save() {
        Map<String, Map<String, Pos>> out = new TreeMap<>();
        homes.forEach((id, h) -> { if (!h.isEmpty()) out.put(id.toString(), new TreeMap<>(h)); });
        try {
            Path tmp = file.resolveSibling(file.getFileName() + ".tmp");
            if (file.getParent() != null) Files.createDirectories(file.getParent());
            try (Writer w = Files.newBufferedWriter(tmp)) { GSON.toJson(new Data(out, new TreeMap<>(warps)), w); }
            Files.move(tmp, file, StandardCopyOption.REPLACE_EXISTING);
        } catch (IOException e) {
            throw new IllegalStateException("Could not save " + file + ": " + e.getMessage(), e);
        }
    }

    // ---- location memory for /back ----------------------------------------

    /** Platforms call this before a teleport and on death. */
    public void remember(UUID player, Pos where) { lastLocations.put(player, where); }

    // ---- homes -------------------------------------------------------------

    public List<String> homeNames(UUID player) {
        return new ArrayList<>(new TreeSet<>(homes.getOrDefault(player, Map.of()).keySet()));
    }

    public Optional<Pos> homeAt(UUID player, String name) {
        return Optional.ofNullable(homes.getOrDefault(player, Map.of()).get(name.toLowerCase(Locale.ROOT)));
    }

    /** {@code /home [name]}. Returns the homes to choose from when several exist and no name was given. */
    public List<String> home(CorePlayer p, String[] args) {
        Map<String, Pos> mine = homes.getOrDefault(p.uuid(), Map.of());
        if (args.length == 0) {
            if (mine.isEmpty()) { p.send(RED + "You have no homes set! Set one with " + YELLOW + "/sethome [name]"); return List.of(); }
            if (mine.size() == 1) { go(p, mine.values().iterator().next()); p.send(GREEN + "Teleported to home."); return List.of(); }
            return homeNames(p.uuid());
        }
        String name = args[0].toLowerCase(Locale.ROOT);
        Pos pos = mine.get(name);
        if (pos == null) {
            p.send(RED + "Home '" + name + "' not found. Your homes: " + YELLOW + String.join(", ", homeNames(p.uuid())));
            return List.of();
        }
        go(p, pos);
        p.send(GREEN + "Teleported to home " + YELLOW + name + GREEN + ".");
        return List.of();
    }

    public void setHome(CorePlayer p, String[] args) {
        String name = args.length > 0 ? args[0].toLowerCase(Locale.ROOT) : "home";
        Map<String, Pos> mine = homes.computeIfAbsent(p.uuid(), k -> new HashMap<>());
        if (name.length() > config.homeNameMax()) { p.send(RED + "Home names can be at most " + config.homeNameMax() + " characters."); return; }
        int limit = Limits.tier(p::hasPermission, "scopenet.homes.", config.maxHomes(), EssentialsConfig.TIER_CAP);
        if (!mine.containsKey(name) && mine.size() >= limit) {
            p.send(RED + "You have reached your limit of " + Limits.show(limit) + " homes.");
            return;
        }
        mine.put(name, p.pos());
        save();
        p.send(GREEN + "Home " + YELLOW + name + GREEN + " set successfully.");
    }

    public void delHome(CorePlayer p, String[] args) {
        String name = args.length > 0 ? args[0].toLowerCase(Locale.ROOT) : "home";
        Map<String, Pos> mine = homes.get(p.uuid());
        if (mine == null || mine.remove(name) == null) { p.send(RED + "Home '" + name + "' does not exist."); return; }
        save();
        p.send(GREEN + "Home " + YELLOW + name + GREEN + " deleted.");
    }

    // ---- warps -------------------------------------------------------------

    /** Every warp, for the map. A copy. */
    public Map<String, Pos> allWarps() { return new TreeMap<>(warps); }

    /** Every player's homes, for the map (only shown when the admin turns that on). A copy. */
    public Map<UUID, Map<String, Pos>> allHomes() {
        Map<UUID, Map<String, Pos>> out = new HashMap<>();
        homes.forEach((id, h) -> out.put(id, new TreeMap<>(h)));
        return out;
    }

    public List<String> warpNames() { return new ArrayList<>(new TreeSet<>(warps.keySet())); }

    public Optional<Pos> warpAt(String name) { return Optional.ofNullable(warps.get(name.toLowerCase(Locale.ROOT))); }

    /** {@code /warp [name]}. With no name, returns the list for a picker (seeding "spawn" if empty). */
    public List<String> warp(CorePlayer p, String[] args) {
        if (args.length == 0) {
            if (warps.isEmpty()) { warps.put("spawn", platform.worldSpawn(p.pos().world())); save(); }
            return warpNames();
        }
        String name = args[0].toLowerCase(Locale.ROOT);
        Pos pos = warps.get(name);
        if (pos == null) { p.send(RED + "Warp '" + name + "' does not exist."); return List.of(); }
        go(p, pos);
        p.send(GREEN + "Teleported to warp " + YELLOW + name + GREEN + ".");
        return List.of();
    }

    public void setWarp(CorePlayer p, String name) {
        if (config.maxWarps() > 0 && !warps.containsKey(name.toLowerCase(Locale.ROOT)) && warps.size() >= config.maxWarps()) {
            p.send(RED + "The server already has " + config.maxWarps() + " warps. Delete one first.");
            return;
        }
        warps.put(name.toLowerCase(Locale.ROOT), p.pos());
        save();
        p.send(GREEN + "Warp " + YELLOW + name.toLowerCase(Locale.ROOT) + GREEN + " set.");
    }

    public void delWarp(CorePlayer p, String name) {
        if (warps.remove(name.toLowerCase(Locale.ROOT)) == null) { p.send(RED + "Warp '" + name + "' does not exist."); return; }
        save();
        p.send(GREEN + "Warp " + YELLOW + name.toLowerCase(Locale.ROOT) + GREEN + " deleted.");
    }

    // ---- back --------------------------------------------------------------

    public void back(CorePlayer p) {
        Pos last = lastLocations.get(p.uuid());
        if (last == null) { p.send(RED + "No previous location found to return to."); return; }
        go(p, last);
        p.send(GREEN + "Teleported back to your previous location.");
    }

    // ---- tpa ---------------------------------------------------------------

    public void tpa(CorePlayer p, String[] args) {
        if (args.length < 1) { p.send(RED + "Usage: /tpa <player>"); return; }
        Optional<CorePlayer> found = platform.playerByName(args[0]);
        if (found.isEmpty()) { p.send(RED + "Player '" + args[0] + "' is not online."); return; }
        CorePlayer target = found.get();
        if (target.uuid().equals(p.uuid())) { p.send(RED + "You cannot teleport to yourself."); return; }
        pendingTpa.put(target.uuid(), Map.entry(p.uuid(), clock.getAsLong()));
        lastAction.put(p.uuid(), clock.getAsLong());
        p.send(GREEN + "Teleport request sent to " + YELLOW + target.name() + GREEN + ".");
        target.send(YELLOW + p.name() + GREEN + " has sent you a teleport request.");
        target.send(GRAY + "Type " + YELLOW + "/tpaccept" + GRAY + " to accept or " + RED + "/tpdeny" + GRAY
                + " to deny (expires in " + config.tpaTimeoutMs() / 1000 + "s).");
    }

    public void tpAccept(CorePlayer p) {
        Map.Entry<UUID, Long> req = pendingTpa.get(p.uuid());
        if (req == null || clock.getAsLong() - req.getValue() > config.tpaTimeoutMs()) {
            pendingTpa.remove(p.uuid());
            p.send(RED + "You have no active teleport requests.");
            return;
        }
        pendingTpa.remove(p.uuid());
        Optional<CorePlayer> requester = platform.player(req.getKey());
        if (requester.isEmpty()) { p.send(RED + "The player who requested to teleport is no longer online."); return; }
        go(requester.get(), p.pos());
        requester.get().send(GREEN + "Teleport request accepted by " + YELLOW + p.name() + GREEN + "!");
        p.send(GREEN + "Accepted teleport request from " + YELLOW + requester.get().name() + GREEN + ".");
    }

    public void tpDeny(CorePlayer p) {
        Map.Entry<UUID, Long> req = pendingTpa.remove(p.uuid());
        if (req == null) { p.send(RED + "You have no active teleport requests."); return; }
        platform.player(req.getKey()).ifPresent(r -> r.send(RED + p.name() + " denied your teleport request."));
        p.send(YELLOW + "Teleport request denied.");
    }

    // ---- rtp ---------------------------------------------------------------

    /** Must run on the server thread, because it reads chunks. */
    public void rtp(CorePlayer p) {
        int radius = config.rtpRadius();
        int min = Math.min(config.rtpMinRadius(), radius - 1);
        String world = p.pos().world();
        p.send(GRAY + "Searching for a safe wilderness destination...");
        for (int i = 0; i < config.rtpAttempts(); i++) {
            int x = rng.nextInt(radius * 2) - radius;
            int z = rng.nextInt(radius * 2) - radius;
            if (Math.max(Math.abs(x), Math.abs(z)) < min) continue; // too close to the centre
            Optional<Pos> spot = platform.safeSurface(world, x, z);
            if (spot.isPresent()) {
                Pos at = spot.get();
                go(p, new Pos(world, x + 0.5, at.y(), z + 0.5, p.pos().yaw(), p.pos().pitch()));
                p.send(GREEN + "Wilderness RTP: Teleported to " + YELLOW + x + ", " + at.blockY() + ", " + z + GREEN + "!");
                return;
            }
        }
        p.send(RED + "Could not find a safe RTP spot. Please try again.");
    }

    /** Teleport and remember where they were for /back. */
    public void teleportTo(CorePlayer p, Pos destination) { go(p, destination); }

    /** When a player last teleported or sent a teleport request (for cooldowns). */
    public long lastAction(UUID id) { return lastAction.getOrDefault(id, -1L); }

    private final Map<UUID, Long> lastAction = new ConcurrentHashMap<>();

    private void go(CorePlayer p, Pos destination) {
        lastAction.put(p.uuid(), clock.getAsLong());
        remember(p.uuid(), p.pos());
        p.teleport(destination);
    }
}
