package net.scopenet.worldmap;

import com.google.gson.*;
import net.scopenet.integration.PanelClient;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.ByteBuffer;
import java.nio.file.*;
import java.util.*;
import java.util.concurrent.*;
import java.util.function.Consumer;

/**
 * SCOPENET Map, game-server side. Renders the world's region files to map tiles on a background thread (low priority, one
 * region at a time) and uploads only the tiles that changed. Player positions and the claims/pins overlay ride along.
 * Nothing here touches the server thread except {@link #offerPlayers} and {@link #offerOverlay}.
 */
public final class WorldMapSync implements AutoCloseable {
    public record Player(String uuid, String name, String dimension, double x, double y, double z, float yaw, float pitch) {}

    private record Dim(String id, String dir, ChunkSampler.Kind kind) {}

    private static final Dim[] DIMS = {
            new Dim("minecraft:overworld", "overworld", ChunkSampler.Kind.OVERWORLD),
            new Dim("minecraft:the_nether", "the_nether", ChunkSampler.Kind.NETHER),
            new Dim("minecraft:the_end", "the_end", ChunkSampler.Kind.END)};
    /** Bump when the look of the tiles changes: caches from an older renderer are redrawn. 3 = biome tints, relief shading, see-through water. */
    private static final int RENDERER_VERSION = 3;
    private static final int BATCH_BYTES = 3 * 1024 * 1024;
    private static final long RERENDER_GAP_MS = 90_000;
    private static final long COARSE_UPLOAD_GAP_MS = 60_000;

    private static final class State {
        long epoch = -1;
        /** The renderer that drew the cached tiles (see {@link #RENDERER_VERSION}). */
        int version;
        /** "dim/x.z" -> file mtime (ms) at the time it was rendered. */
        Map<String, Long> regions = new HashMap<>();
        /** Tiles waiting to be uploaded, as {@link TileStore.Key} strings. */
        Set<String> dirty = new HashSet<>();
    }

    private final PanelClient client;
    private final Path worldRoot;
    private final Path cacheDir;
    private final Consumer<String> log;
    private final Gson gson = new Gson();
    private final TileStore store;
    private final Object wake = new Object();
    private final Thread thread;
    private final ScheduledExecutorService live = Executors.newSingleThreadScheduledExecutor(r -> {
        Thread t = new Thread(r, "scopenet-map-live");
        t.setDaemon(true);
        return t;
    });

    private State state = new State();
    private final Map<String, Long> renderedAt = new HashMap<>();
    private volatile boolean closed;
    private volatile boolean enabled;
    private volatile boolean wakeRequested;
    private volatile int maxZoom = 6;
    private volatile long playerIntervalMs = 2000;
    private volatile List<Player> players = List.of();
    private volatile java.util.function.Supplier<String> overlay;
    private long configAt;
    private long reconcileAt;
    private long coarseUploadAt;
    private boolean stateDirty;
    private boolean warnedMissing;
    private boolean reportedEmpty;
    private String lastOverlaySent;
    private long overlaySentAt;
    private String lastPlayersSent = "";
    private long playersSentAt;
    private volatile String stage = "starting";
    private volatile String lastError = "";
    /** Why the world produced nothing to draw (kept until a region does draw). */
    private volatile String lastEmpty = "";
    private volatile String lastRegion = "";
    private volatile long regionsRendered;
    private volatile long tilesUploaded;
    private volatile long lastUploadAt;
    private volatile int regionFiles;
    private volatile int queuedTiles;

    public WorldMapSync(PanelClient client, Path worldRoot, Path cacheDir, Consumer<String> log) {
        this.client = client;
        this.worldRoot = worldRoot;
        this.cacheDir = cacheDir;
        this.log = log;
        this.store = new TileStore(cacheDir.resolve("tiles"));
        thread = new Thread(this::run, "scopenet-map");
        thread.setDaemon(true);
        thread.setPriority(Thread.MIN_PRIORITY);
    }

    public void start() {
        loadState();
        thread.start();
        live.scheduleWithFixedDelay(this::sendLive, 4, 1, TimeUnit.SECONDS);
    }

    public void offerPlayers(List<Player> next) {
        players = List.copyOf(next);
    }

    /** Safe to read on the game thread for /map and /scopenet map. */
    public String status() {
        return "Map " + (enabled ? stage : "waiting for panel/map setting") + "; world=" + worldRoot
                + "; region files=" + regionFiles + "; rendered=" + regionsRendered
                + "; queued tiles=" + queuedTiles + "; uploaded=" + tilesUploaded
                + (lastRegion.isEmpty() ? "" : "; last region=" + lastRegion)
                + (lastError.isEmpty() ? "" : "; error=" + lastError);
    }

    /** Where the claims and pins come from ({@code {"claims":[...],"pins":[...]}}); asked every few seconds, sent when it changes. */
    public void setOverlay(java.util.function.Supplier<String> source) {
        overlay = source;
    }

    /** The world was saved: look for changed regions soon. */
    public void requestScan() {
        synchronized (wake) {
            wakeRequested = true;
            wake.notifyAll();
        }
    }

    @Override public void close() {
        closed = true;
        live.shutdownNow();
        thread.interrupt();
        saveState();
    }

    // ---- main loop ----

    private void run() {
        while (!closed) {
            long pause = 10_000;
            try {
                if (!checkConfig()) pause = 60_000;
                else {
                    boolean worked = cycle();
                    pause = worked ? 150 : 10_000;
                }
            } catch (InterruptedException e) {
                return;
            } catch (PanelClient.HttpFailure e) {
                if (e.status == 403 || e.status == 404) enabled = false;
                else log.accept("SCOPENET Map: the panel refused an upload (" + e.getMessage() + ")");
                lastError = "panel HTTP " + e.status + ": " + e.getMessage();
                stage = "upload failed";
                pause = 30_000;
            } catch (Exception e) {
                log.accept("SCOPENET Map failed: " + e.getClass().getSimpleName() + ": " + e.getMessage());
                lastError = e.getClass().getSimpleName() + ": " + e.getMessage();
                stage = "failed";
                pause = 60_000;
            }
            if (stateDirty) saveState();
            queuedTiles = state.dirty.size();
            try {
                synchronized (wake) {
                    if (!wakeRequested) wake.wait(pause);
                    wakeRequested = false;
                }
            } catch (InterruptedException e) {
                return;
            }
        }
    }

    private boolean checkConfig() throws IOException, InterruptedException {
        long now = System.currentTimeMillis();
        if (now < configAt) return enabled;
        configAt = now + 60_000;
        try {
            JsonObject config = client.get("map/config");
            boolean on = config.has("enabled") && config.get("enabled").getAsBoolean();
            if (config.has("max_zoom")) maxZoom = Math.max(0, Math.min(6, config.get("max_zoom").getAsInt()));
            if (config.has("player_interval_ms")) playerIntervalMs = Math.max(500, config.get("player_interval_ms").getAsLong());
            long epoch = config.has("epoch") ? config.get("epoch").getAsLong() : 0;
            if (on && epoch != state.epoch) {
                if (state.epoch >= 0) log.accept("SCOPENET Map was reset on the panel: rendering the world again.");
                store.clear();
                state = new State();
                state.epoch = epoch;
                renderedAt.clear();
                stateDirty = true;
            }
            if (on) reconcile(config);
            if (on && !enabled) log.accept("SCOPENET Map is on: rendering this world and sending it to the panel.");
            enabled = on;
            stage = on ? "scanning regions" : "disabled on panel";
            if (on && lastEmpty.isEmpty()) lastError = "";
        } catch (PanelClient.HttpFailure e) {
            enabled = false; // an older panel without the map
            stage = "panel configuration failed";
            lastError = "panel HTTP " + e.status + ": " + e.getMessage();
        }
        return enabled;
    }

    /**
     * Cached tiles the panel doesn't have are sent again. A cache that says "done" is only true if the panel kept what it was sent:
     * a wiped data folder, a restored backup or an older build would otherwise leave the map empty for good.
     */
    private void reconcile(JsonObject config) {
        long now = System.currentTimeMillis();
        if (now < reconcileAt || !state.dirty.isEmpty() || !config.has("have") || !config.get("have").isJsonObject()) return;
        reconcileAt = now + 10 * 60_000;
        JsonObject have = config.getAsJsonObject("have");
        int queued = 0;
        for (Dim dim : DIMS) {
            int local = store.keys(dim.dir(), 0).size();
            int remote = have.has(dim.dir()) ? have.get(dim.dir()).getAsInt() : 0;
            if (local <= remote) continue;
            for (int z = 0; z <= maxZoom; z++) {
                for (TileStore.Key k : store.keys(dim.dir(), z)) {
                    state.dirty.add(k.toString());
                    queued++;
                }
            }
            log.accept("SCOPENET Map: the panel has " + remote + " " + dim.dir() + " tiles but " + local + " are cached here; sending them again.");
        }
        if (queued > 0) stateDirty = true;
    }

    /**
     * Tiles drawn by an older renderer are thrown away so the whole world is drawn again in the current style (and any tile the
     * panel never received is sent again). Runs once per cache; a fresh cache just records the version.
     */
    private void upgradeRenderer() throws IOException {
        if (!state.regions.isEmpty() || !state.dirty.isEmpty()) log.accept("SCOPENET Map: new map style, drawing the world again.");
        store.clear();
        state.regions.clear();
        state.dirty.clear();
        renderedAt.clear();
        state.version = RENDERER_VERSION;
        stateDirty = true;
    }

    /** One unit of work: render the most useful stale region, then upload what changed. True if anything happened. */
    private boolean cycle() throws IOException, InterruptedException {
        if (!Files.isDirectory(worldRoot)) {
            stage = "world folder missing";
            if (!warnedMissing) {
                warnedMissing = true;
                log.accept("SCOPENET Map: world folder " + worldRoot + " not found");
            }
            return false;
        }
        if (state.version < RENDERER_VERSION) upgradeRenderer();
        long now = System.currentTimeMillis();
        record Stale(Dim dim, Path file, String key, long mtime, int rx, int rz, double distance) {}
        List<Stale> stale = new ArrayList<>();
        int filesFound = 0;
        int completeFiles = 0;
        List<Player> online = players;
        for (Dim dim : DIMS) {
            Path dir = regionDir(dim);
            if (dir == null) continue;
            try (DirectoryStream<Path> stream = Files.newDirectoryStream(dir, "r.*.*.mca")) {
                for (Path file : stream) {
                    filesFound++;
                    String[] parts = file.getFileName().toString().split("\\.");
                    int rx, rz;
                    try {
                        rx = Integer.parseInt(parts[1]);
                        rz = Integer.parseInt(parts[2]);
                    } catch (RuntimeException e) {
                        continue;
                    }
                    if (Files.size(file) <= 8192) continue;
                    completeFiles++;
                    long mtime = Files.getLastModifiedTime(file).toMillis();
                    String key = dim.dir() + "/" + rx + "." + rz;
                    Long done = state.regions.get(key);
                    if (done != null && mtime <= done) continue;
                    Long last = renderedAt.get(key);
                    if (last != null && now - last < RERENDER_GAP_MS) continue;
                    double distance = Double.MAX_VALUE;
                    for (Player p : online) {
                        if (!dim.id().equals(p.dimension())) continue;
                        double dx = p.x() - (rx * 512 + 256), dz = p.z() - (rz * 512 + 256);
                        distance = Math.min(distance, Math.sqrt(dx * dx + dz * dz));
                    }
                    stale.add(new Stale(dim, file, key, mtime, rx, rz, distance));
                }
            }
        }
        regionFiles = filesFound;
        if (filesFound == 0) stage = "no region files found";
        else if (completeFiles == 0) stage = "region files have no saved chunks yet";
        boolean worked = false;
        if (!stale.isEmpty()) {
            stale.sort(Comparator.comparingDouble(Stale::distance).thenComparing(Stale::key));
            Stale next = stale.get(0);
            worked = renderRegion(next.dim(), next.file(), next.key(), next.mtime(), next.rx(), next.rz());
        }
        if (flushUploads(stale.size() <= 1)) worked = true;
        if (filesFound > 0 && stale.isEmpty() && state.dirty.isEmpty() && state.regions.size() >= filesFound) {
            stage = tilesUploaded == 0 && regionsRendered > 0 ? "rendered " + regionsRendered + " regions but produced no tiles" : "caught up";
        }
        return worked;
    }

    private boolean renderRegion(Dim dim, Path file, String key, long mtime, int rx, int rz) throws IOException {
        stage = "rendering " + key;
        lastRegion = key;
        int[][] tiles;
        try {
            tiles = RegionRenderer.render(file, dim.kind());
        } catch (IOException | RuntimeException e) {
            // A region being written right now; try again after the gap.
            renderedAt.put(key, System.currentTimeMillis());
            lastError = "render " + key + ": " + e.getMessage();
            return false;
        }
        renderedAt.put(key, System.currentTimeMillis());
        if (tiles == null) {
            // A region may still be generating or use a format we cannot read. Never mark it complete.
            String why = RegionRenderer.lastEmptyReport;
            lastError = "No readable full chunks in " + key + (why == null ? "" : ": " + why) + "; will retry";
            if (why != null && !reportedEmpty) {
                reportedEmpty = true;
                log.accept("SCOPENET Map: " + lastError);
            }
            stage = "waiting for readable chunks";
            return false;
        }
        lastError = "";
        regionsRendered++;
        String empty = RegionRenderer.lastEmptyReport;
        if (empty == null) lastEmpty = "";
        else {
            // Readable chunks, nothing to draw: say so loudly, once, with what the chunks looked like.
            lastEmpty = "Nothing drawable in " + key + ": " + empty;
            lastError = lastEmpty;
            stage = "read the world but found no blocks to draw";
            if (!reportedEmpty) {
                reportedEmpty = true;
                log.accept("SCOPENET Map: " + lastError);
            }
        }
        List<TileStore.Key> changed = new ArrayList<>();
        if (tiles != null) {
            for (int i = 0; i < 4; i++) {
                TileStore.Key k = new TileStore.Key(dim.dir(), 0, rx * 2 + (i & 1), rz * 2 + (i >> 1));
                if (store.update(k, tiles[i])) {
                    changed.add(k);
                    state.dirty.add(k.toString());
                }
            }
        }
        // Rebuild the zoom pyramid above whatever changed.
        List<TileStore.Key> level = changed;
        for (int z = 0; z < maxZoom && !level.isEmpty(); z++) {
            Set<Long> seen = new HashSet<>();
            List<TileStore.Key> next = new ArrayList<>();
            for (TileStore.Key child : level) {
                long id = ((long) (child.x() >> 1) << 32) ^ ((child.y() >> 1) & 0xFFFFFFFFL);
                if (!seen.add(id)) continue;
                TileStore.Key parent = store.rebuildParent(child);
                if (parent != null) {
                    next.add(parent);
                    state.dirty.add(parent.toString());
                }
            }
            level = next;
        }
        state.regions.put(key, mtime);
        stateDirty = true;
        return true;
    }

    /** Uploads dirty tiles in batches. Coarse zoom levels wait until the world is mostly done so they aren't re-sent constantly. */
    private boolean flushUploads(boolean caughtUp) throws IOException, InterruptedException {
        if (state.dirty.isEmpty()) return false;
        long now = System.currentTimeMillis();
        boolean coarseNow = caughtUp || now - coarseUploadAt >= COARSE_UPLOAD_GAP_MS;
        List<TileStore.Key> ready = new ArrayList<>();
        for (String s : state.dirty) {
            TileStore.Key k = TileStore.Key.parse(s);
            if (k.zoom() == 0 || coarseNow) ready.add(k);
        }
        if (ready.isEmpty()) return false;
        ready.sort(Comparator.comparing(TileStore.Key::dim).thenComparing(Comparator.comparingInt(TileStore.Key::zoom).reversed()));
        if (coarseNow) coarseUploadAt = now;
        boolean sentAny = false;
        int batches = 0;
        int i = 0;
        while (i < ready.size() && batches < 8 && !closed) {
            String dimension = ready.get(i).dim();
            ByteArrayOutputStream body = new ByteArrayOutputStream();
            List<TileStore.Key> inBatch = new ArrayList<>();
            while (i < ready.size() && ready.get(i).dim().equals(dimension) && body.size() < BATCH_BYTES) {
                TileStore.Key k = ready.get(i++);
                int[] pixels = store.load(k);
                if (pixels == null) {
                    state.dirty.remove(k.toString());
                    continue;
                }
                byte[] png = Png.encode(pixels, RegionRenderer.TILE, RegionRenderer.TILE);
                body.writeBytes(ByteBuffer.allocate(13).put((byte) k.zoom()).putInt(k.x()).putInt(k.y()).putInt(png.length).array());
                body.writeBytes(png);
                inBatch.add(k);
            }
            if (inBatch.isEmpty()) continue;
            client.postBytes("map/tiles?dim=" + idOf(dimension), body.toByteArray());
            tilesUploaded += inBatch.size();
            lastUploadAt = System.currentTimeMillis();
            stage = "uploading tiles";
            for (TileStore.Key k : inBatch) state.dirty.remove(k.toString());
            stateDirty = true;
            sentAny = true;
            batches++;
        }
        return sentAny;
    }

    private static String idOf(String dir) {
        for (Dim d : DIMS) if (d.dir().equals(dir)) return d.id();
        return "minecraft:" + dir;
    }

    // ---- live data ----

    private void sendLive() {
        if (closed || !enabled) return;
        try {
            long now = System.currentTimeMillis();
            List<Player> snapshot = players;
            JsonArray list = new JsonArray();
            for (Player p : snapshot) {
                JsonObject o = new JsonObject();
                o.addProperty("uuid", p.uuid());
                o.addProperty("name", p.name());
                o.addProperty("dimension", p.dimension());
                o.addProperty("x", Math.round(p.x() * 10) / 10.0);
                o.addProperty("y", Math.round(p.y() * 10) / 10.0);
                o.addProperty("z", Math.round(p.z() * 10) / 10.0);
                o.addProperty("yaw", Math.round(p.yaw()));
                list.add(o);
            }
            String text = list.toString();
            if ((!text.equals(lastPlayersSent) && now - playersSentAt >= playerIntervalMs) || now - playersSentAt >= 10_000) {
                JsonObject body = new JsonObject();
                body.add("players", list);
                client.post("map/players", body);
                lastPlayersSent = text;
                playersSentAt = now;
            }
            java.util.function.Supplier<String> source = overlay;
            String ov = source == null || now - overlaySentAt < 10_000 ? null : source.get();
            if (ov != null && (!ov.equals(lastOverlaySent) || now - overlaySentAt >= 5 * 60_000) && now - overlaySentAt >= 10_000) {
                client.post("map/overlay", JsonParser.parseString(ov).getAsJsonObject());
                lastOverlaySent = ov;
                overlaySentAt = now;
            }
            if (now - diagnosticsSentAt >= 10_000) {
                JsonObject diagnostic = new JsonObject();
                diagnostic.addProperty("stage", stage);
                diagnostic.addProperty("error", lastError.length() > 1800 ? lastError.substring(0, 1800) : lastError);
                diagnostic.addProperty("world_root", worldRoot.toString());
                diagnostic.addProperty("region_files", regionFiles);
                diagnostic.addProperty("regions_rendered", regionsRendered);
                diagnostic.addProperty("queued_tiles", queuedTiles);
                diagnostic.addProperty("tiles_uploaded", tilesUploaded);
                diagnostic.addProperty("last_region", lastRegion);
                diagnostic.addProperty("last_upload_ms", lastUploadAt);
                try {
                    client.post("map/diagnostics", diagnostic);
                    diagnosticsSentAt = now;
                } catch (PanelClient.HttpFailure e) {
                    // Diagnostics are optional while an older panel is being upgraded.
                    diagnosticsSentAt = now;
                    if (e.status != 404) lastError = "diagnostics HTTP " + e.status + ": " + e.getMessage();
                }
            }
        } catch (PanelClient.HttpFailure e) {
            if (e.status == 403 || e.status == 404) enabled = false;
            lastError = "live upload HTTP " + e.status + ": " + e.getMessage();
        } catch (Exception e) {
            lastError = "live upload: " + e.getMessage();
        }
    }
    private long diagnosticsSentAt;

    // ---- world layout ----

    private Path regionDir(Dim dim) {
        String name = worldRoot.getFileName() == null ? "world" : worldRoot.getFileName().toString();
        Path parent = worldRoot.toAbsolutePath().getParent();
        List<Path> candidates = switch (dim.dir()) {
            case "the_nether" -> Arrays.asList(worldRoot.resolve("DIM-1/region"), sibling(parent, name + "_nether", "DIM-1/region"),
                    worldRoot.resolve("dimensions/minecraft/the_nether/region"));
            case "the_end" -> Arrays.asList(worldRoot.resolve("DIM1/region"), sibling(parent, name + "_the_end", "DIM1/region"),
                    worldRoot.resolve("dimensions/minecraft/the_end/region"));
            default -> List.of(worldRoot.resolve("region"), worldRoot.resolve("dimensions/minecraft/overworld/region"));
        };
        for (Path p : candidates) if (p != null && Files.isDirectory(p)) return p;
        return null;
    }

    private static Path sibling(Path parent, String name, String rel) {
        return parent == null ? null : parent.resolve(name).resolve(rel);
    }

    // ---- state ----

    private void loadState() {
        try {
            Path f = cacheDir.resolve("state.json");
            if (Files.isRegularFile(f)) {
                State s = gson.fromJson(Files.readString(f), State.class);
                if (s != null && s.regions != null && s.dirty != null) {
                    state = s;
                    // Older builds could mark an unreadable region complete without producing a tile.
                    // Clear those entries on upgrade so a live server can recover without an admin reset.
                    state.regions.keySet().removeIf(key -> {
                        String[] parts = key.split("[/ .]");
                        if (parts.length != 3) return true;
                        try {
                            int rx = Integer.parseInt(parts[1]), rz = Integer.parseInt(parts[2]);
                            for (int i = 0; i < 4; i++) {
                                if (store.exists(new TileStore.Key(parts[0], 0, rx * 2 + (i & 1), rz * 2 + (i >> 1)))) return false;
                            }
                        } catch (NumberFormatException e) { return true; }
                        return true;
                    });
                    stateDirty = true;
                }
            }
        } catch (IOException | RuntimeException e) {
            state = new State();
        }
    }

    private void saveState() {
        try {
            Files.createDirectories(cacheDir);
            Path f = cacheDir.resolve("state.json");
            Path tmp = f.resolveSibling("state.json.tmp");
            Files.writeString(tmp, gson.toJson(state));
            Files.move(tmp, f, StandardCopyOption.REPLACE_EXISTING);
            stateDirty = false;
        } catch (IOException e) {
            // the cache is only an optimisation
        }
    }
}
