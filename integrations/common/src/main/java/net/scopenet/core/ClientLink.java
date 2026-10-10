package net.scopenet.core;

import com.google.gson.*;
import net.scopenet.integration.ChunkCheckResult;

import java.util.*;
import java.util.concurrent.ConcurrentHashMap;
import java.util.function.BiConsumer;

/**
 * Talks to the optional Velora client mod over the {@link Wire} channels. The client says hello; from then on this
 * pushes that player's state (level, balance, guild, quests), the claims around them, and answers market and shop
 * requests. Player actions use an allowlisted panel bridge or ordinary server commands; the server owns every result.
 */
public final class ClientLink {
    private static final int RADIUS = 4;

    private final Env env;
    private final PlayerCache cache;
    private final BiConsumer<CorePlayer, byte[]> out;
    private final String serverName;
    private final Map<UUID, String> lastState = new ConcurrentHashMap<>();
    private final Map<UUID, String> lastClaimsKey = new ConcurrentHashMap<>();
    private final Set<UUID> clients = ConcurrentHashMap.newKeySet();
    private final Map<UUID, Long> lastRequest = new ConcurrentHashMap<>();
    private final Set<UUID> requests = ConcurrentHashMap.newKeySet();
    private final Map<UUID, String> requestIds = new ConcurrentHashMap<>();
    private final Map<UUID, Long> sessions = new ConcurrentHashMap<>();
    private long nextSession;
    private final Set<UUID> nativeClients = ConcurrentHashMap.newKeySet();
    private BiConsumer<CorePlayer, JsonObject> localRequest;
    private String panelUrl = "";
    public void panelUrl(String url) { panelUrl = url; }
    public void localRequests(BiConsumer<CorePlayer, JsonObject> handler) { localRequest = handler; }
    /** One request per player at most this often; a modded client can't turn the link into a panel flood. */
    static final long MIN_REQUEST_GAP_MS = 400;

    public ClientLink(Env env, PlayerCache cache, String serverName, BiConsumer<CorePlayer, byte[]> out) {
        this.env = env; this.cache = cache; this.serverName = serverName; this.out = out;
    }

    public boolean hasClient(UUID uuid) { return clients.contains(uuid); }

    public void disconnected(UUID uuid) {
        clients.remove(uuid); nativeClients.remove(uuid); sessions.remove(uuid); requests.remove(uuid); requestIds.remove(uuid); lastState.remove(uuid); lastClaimsKey.remove(uuid); lastRequest.remove(uuid); cache.forget(uuid);
    }

    private void send(CorePlayer p, JsonObject message) { out.accept(p, Wire.encode(message.toString())); }

    /** A packet from a player's client. Unknown or malformed messages are ignored. */
    public void receive(CorePlayer p, byte[] payload) {
        JsonObject msg;
        try { msg = JsonParser.parseString(Wire.decode(payload)).getAsJsonObject(); }
        catch (RuntimeException e) { return; }
        String type = PlayerCache.str(msg, "t", "");
        if (!env.features.clientLink()) return;
        long now = env.clock.getAsLong();
        Long previous = lastRequest.put(p.uuid(), now);
        if (previous != null && now - previous < MIN_REQUEST_GAP_MS) {
            String id = PlayerCache.str(msg, "id", "");
            if (type.equals("request") && id.matches("[a-zA-Z0-9-]{1,64}")) result(p, id, null, "Please wait a moment before another request.");
            return;
        }
        switch (type) {
            case "hello" -> {
                if (PlayerCache.num(msg, "protocol", 1) == 2) nativeClients.add(p.uuid());
                else nativeClients.remove(p.uuid());
                sessions.putIfAbsent(p.uuid(), ++nextSession);
                hello(p);
            }
            case "request" -> { if (nativeClients.contains(p.uuid())) request(p, msg); }
            case "refresh" -> { if (clients.contains(p.uuid())) { lastState.remove(p.uuid()); cache.refresh(p, info -> pushState(p)); } }
            case "claims" -> { if (clients.contains(p.uuid())) { lastClaimsKey.remove(p.uuid()); pushClaims(p); } }
            case "market" -> { if (clients.contains(p.uuid())) market(p); }
            case "shop" -> { if (clients.contains(p.uuid())) shop(p); }
            default -> { }
        }
    }

    private void hello(CorePlayer p) {
        clients.add(p.uuid());
        Features f = env.features;
        JsonObject m = new JsonObject();
        m.addProperty("t", "hello");
        m.addProperty("v", Wire.VERSION);
        m.addProperty("companion", 2);
        if (nativeClients.contains(p.uuid())) m.addProperty("panel", panelUrl);
        m.addProperty("server", serverName);
        m.addProperty("currency", f.currencySymbol());
        JsonObject features = new JsonObject();
        features.addProperty("essentials", f.essentials());
        features.addProperty("economy", f.economy());
        features.addProperty("guilds", f.guilds());
        features.addProperty("claims", f.guilds() && f.landClaiming());
        features.addProperty("social", f.social());
        m.add("features", features);
        m.add("modules", env.modules.get().toJson());
        send(p, m);
        cache.refresh(p, info -> pushState(p));
        if (cache.info(p.uuid()) != null) pushState(p);
        pushClaims(p);
    }

    private static final Set<String> OPERATIONS = Set.of("market", "quests", "quest_claim", "achievements", "levels", "transactions", "friends", "friend_request",
            "notifications", "notifications_read", "guild", "guilds", "guild_bank", "guild_requests", "map", "orders", "contracts", "darknet", "darknet_buy",
            "casino", "casino_history", "casino_slots", "casino_wheel", "casino_plinko", "casino_dice", "casino_coinflip", "casino_roulette",
            "casino_crash_start", "casino_crash_status", "casino_crash_cashout", "casino_blackjack_start", "casino_blackjack_hit", "casino_blackjack_stand", "casino_blackjack_double",
            "casino_mines_start", "casino_mines_reveal", "casino_mines_cashout", "casino_burst_start", "casino_burst_advance", "casino_burst_cashout");

    private void request(CorePlayer p, JsonObject msg) {
        String id = PlayerCache.str(msg, "id", ""), op = PlayerCache.str(msg, "operation", "");
        if (!id.matches("[a-zA-Z0-9-]{1,64}")) return;
        if (op.equals("claim_edit")) { editClaim(p, id, PlayerCache.obj(msg, "args")); return; }
        if (op.equals("travel") || op.equals("storage") || op.equals("physical_shop")) {
            if (localRequest != null) localRequest.accept(p, msg); else result(p, id, null, "This server does not expose that screen.");
            return;
        }
        if (!OPERATIONS.contains(op)) { result(p, id, null, "Unsupported companion operation."); return; }
        if ((op.equals("market") || op.equals("transactions") || op.equals("orders") || op.equals("contracts") || op.startsWith("darknet")) && !env.features.economy()
                || op.startsWith("casino") && (!env.modules.get().enabled("casino") || !env.features.economy())
                || op.equals("map") && !env.modules.get().enabled("map")
                || op.startsWith("guild") && !env.features.guilds()
                || (op.equals("friends") || op.equals("friend_request")) && !env.features.social()) {
            result(p, id, null, "This feature is disabled."); return;
        }
        if ((op.startsWith("casino")&&!p.hasPermission("scopenet.command.casino")) || (op.startsWith("darknet")&&!p.hasPermission("scopenet.command.darknet"))) {
            result(p,id,null,"You do not have permission to use this screen.");return;
        }
        if (op.startsWith("guild") && !p.hasPermission("scopenet.command.guild") || op.equals("guild_bank") && !p.hasPermission("scopenet.command.guild.bank")) {
            result(p, id, null, "You do not have permission to use this screen."); return;
        }
        if (!requests.add(p.uuid())) { result(p, id, null, "A request is already running."); return; }
        if (id.equals(requestIds.put(p.uuid(), id))) { requests.remove(p.uuid()); result(p, id, null, "This request was already submitted. Refresh before trying again."); return; }
        JsonObject args = PlayerCache.obj(msg, "args");
        if (args == null) args = new JsonObject();
        if (args.toString().length() > 4096) { requests.remove(p.uuid()); result(p, id, null, "Request too large."); return; }
        JsonObject body = new JsonObject();
        body.addProperty("request_id", id);
        body.addProperty("uuid", p.uuid().toString()); body.addProperty("operation", op); body.add("args", args);
        Long session = sessions.get(p.uuid());
        env.io(() -> env.panel.call("companion", body), data -> {
            if (!Objects.equals(session, sessions.get(p.uuid()))) return;
            requests.remove(p.uuid()); result(p, id, data, null);
            lastState.remove(p.uuid()); cache.refresh(p, info -> pushState(p));
        }, error -> {
            if (!Objects.equals(session, sessions.get(p.uuid()))) return;
            requests.remove(p.uuid()); result(p, id, null, error);
        });
    }

    private void editClaim(CorePlayer p, String id, JsonObject args) {
        if (!env.modules.get().enabled("map") || !env.modules.get().enabled("factions") || !env.features.landClaiming()) {
            result(p,id,null,"Map claim editing is disabled."); return;
        }
        try {
            if (args == null || args.toString().length()>256) throw new IllegalArgumentException();
            String action=args.get("action").getAsString();
            if (!Set.of("claim","unclaim").contains(action)) throw new IllegalArgumentException();
            int x=args.get("x").getAsBigDecimal().intValueExact(), z=args.get("z").getAsBigDecimal().intValueExact();
            if (Math.abs((long)x)>1_875_000 || Math.abs((long)z)>1_875_000) throw new IllegalArgumentException();
            if (!p.hasPermission("scopenet.command.guild."+action)) { result(p,id,null,"You do not have permission to edit claims."); return; }
            if (!requests.add(p.uuid())) { result(p,id,null,"A request is already running."); return; }
            JsonObject body=new JsonObject(); body.addProperty("uuid",p.uuid().toString());
            body.addProperty("dimension",p.pos().world()); body.addProperty("chunk_x",x); body.addProperty("chunk_z",z);
            Long session=sessions.get(p.uuid());
            env.io(()->env.panel.call("guilds/"+action,body),data->{
                if (!Objects.equals(session,sessions.get(p.uuid()))) return;
                requests.remove(p.uuid()); env.panel.claimsChanged(); lastClaimsKey.remove(p.uuid()); result(p,id,data,null);
            },error->{if (Objects.equals(session,sessions.get(p.uuid()))) {requests.remove(p.uuid());result(p,id,null,error);}});
        } catch (RuntimeException invalid) { result(p,id,null,"Choose valid whole-number chunk coordinates and an action."); }
    }

    /** Bounded fragments keep each payload below Paper's messaging limit. */
    public void result(CorePlayer p, String id, JsonElement data, String error) {
        JsonObject response = new JsonObject();
        response.addProperty("ok", error == null);
        if (error != null) response.addProperty("error", error); else response.add("data", data == null ? JsonNull.INSTANCE : data);
        String json = response.toString();
        if (json.length() > 480_000) json = "{\"ok\":false,\"error\":\"Too much data; narrow your selection in the launcher.\"}";
        List<String> chunks = new ArrayList<>();
        for (int start = 0; start < json.length();) {
            int end = Math.min(json.length(), start + 4000);
            if (end < json.length() && Character.isHighSurrogate(json.charAt(end - 1))) end--;
            chunks.add(json.substring(start, end)); start = end;
        }
        for (int at = 0; at < chunks.size(); at++) {
            JsonObject part = new JsonObject(); part.addProperty("t", "response"); part.addProperty("id", id);
            part.addProperty("part", at); part.addProperty("parts", chunks.size());
            part.addProperty("json", chunks.get(at)); send(p, part);
        }
    }

    /** Push the state if it changed since the last push. */
    public void pushState(CorePlayer p) {
        if (!clients.contains(p.uuid())) return;
        JsonObject i = cache.info(p.uuid());
        if (i == null) return;
        JsonObject global = PlayerCache.obj(i, "global"), server = PlayerCache.obj(i, "server"), guild = PlayerCache.obj(i, "guild");
        JsonObject quests = PlayerCache.obj(i, "quests");
        JsonObject m = new JsonObject();
        m.addProperty("t", "state");
        m.add("modules", env.modules.get().toJson());
        m.addProperty("name", p.name());
        m.addProperty("level", (long) PlayerCache.num(global, "level", 1));
        m.addProperty("xp", (long) PlayerCache.num(global, "xp", 0));
        m.addProperty("xpNow", (long) PlayerCache.num(global, "current_level_xp", 0));
        m.addProperty("xpNext", (long) PlayerCache.num(global, "next_level_xp", 0));
        m.addProperty("xpPct", PlayerCache.num(global, "progress_pct", 0));
        m.addProperty("title", PlayerCache.str(global, "title", ""));
        m.addProperty("serverLevel", (long) PlayerCache.num(server, "level", 1));
        m.addProperty("serverXpPct", PlayerCache.num(server, "progress_pct", 0));
        if (guild != null) {
            JsonObject g = new JsonObject();
            g.addProperty("name", PlayerCache.str(guild, "name", ""));
            g.addProperty("tag", PlayerCache.str(guild, "tag", ""));
            g.addProperty("role", PlayerCache.str(guild, "role", ""));
            g.addProperty("claims", (long) PlayerCache.num(guild, "claims", 0));
            m.add("guild", g);
        }
        m.addProperty("balance", PlayerCache.num(i, "balance", 0));
        m.addProperty("playtime", (long) PlayerCache.num(i, "playtime_secs", 0));
        for (String period : List.of("daily", "weekly")) {
            JsonObject q = PlayerCache.obj(quests, period);
            JsonObject o = new JsonObject();
            o.addProperty("done", (long) PlayerCache.num(q, "completed", 0));
            o.addProperty("total", (long) PlayerCache.num(q, "total", 0));
            m.add(period, o);
        }
        JsonObject rank = PlayerCache.obj(i, "rank");
        if (rank != null) m.addProperty("rank", PlayerCache.str(rank, "display", PlayerCache.str(rank, "group", "")));
        m.addProperty("kills", (long) PlayerCache.num(i, "kills", 0));
        m.addProperty("deaths", (long) PlayerCache.num(i, "deaths", 0));
        JsonObject identity = PlayerCache.obj(i, "identity");
        JsonObject cosmetics = PlayerCache.obj(i, "cosmetics");
        if (identity != null) m.add("identity", identity.deepCopy());
        if (cosmetics != null) m.add("cosmetics", cosmetics.deepCopy());
        String text = m.toString();
        if (text.equals(lastState.put(p.uuid(), text))) return;
        send(p, m);
    }

    /** Claims around the player; sent when they change chunk or the claim map changes. */
    public void pushClaims(CorePlayer p) {
        if (!clients.contains(p.uuid()) || !env.features.guilds() || !env.features.landClaiming()) return;
        Pos pos = p.pos();
        int cx = pos.blockX() >> 4, cz = pos.blockZ() >> 4;
        StringBuilder cells = new StringBuilder();
        for (int dz = -RADIUS; dz <= RADIUS; dz++) {
            for (int dx = -RADIUS; dx <= RADIUS; dx++) {
                ChunkCheckResult r = env.panel.check(pos.world(), cx + dx, cz + dz, p.uuid());
                cells.append(!r.claimed() ? '0' : r.allowed() ? '1' : '2');
            }
        }
        ChunkCheckResult here = env.panel.check(pos.world(), cx, cz, p.uuid());
        String key = pos.world() + ":" + cx + ":" + cz + ":" + cells;
        if (key.equals(lastClaimsKey.put(p.uuid(), key))) return;
        JsonObject m = new JsonObject();
        m.addProperty("t", "claims");
        m.addProperty("dim", pos.world());
        m.addProperty("cx", cx);
        m.addProperty("cz", cz);
        m.addProperty("r", RADIUS);
        m.addProperty("cells", cells.toString());
        if (here.claimed()) {
            JsonObject owner = new JsonObject();
            owner.addProperty("name", here.guildName() == null ? "" : here.guildName());
            owner.addProperty("tag", here.guildTag() == null ? "" : here.guildTag());
            owner.addProperty("mine", here.allowed());
            m.add("owner", owner);
        }
        send(p, m);
    }

    private void market(CorePlayer p) {
        if (!env.features.economy() || !env.modules.get().enabled("economy")) return;
        env.io(() -> env.panel.call("economy/market", new JsonObject()), el -> {
            JsonObject m = new JsonObject();
            m.addProperty("t", "market");
            m.add("listings", el.isJsonArray() ? el.getAsJsonArray() : new JsonArray());
            m.addProperty("balance", PlayerCache.num(cache.info(p.uuid()), "balance", 0));
            send(p, m);
        }, e -> notify(p, "error", "Market", "Could not load listings: " + e));
    }

    private void shop(CorePlayer p) {
        if (!env.features.economy() || !env.modules.get().enabled("economy")) return;
        JsonObject m = new JsonObject();
        m.addProperty("t", "shop");
        JsonArray items = new JsonArray();
        ItemValues.all().forEach((id, price) -> {
            JsonObject o = new JsonObject();
            o.addProperty("id", id);
            o.addProperty("name", id.toLowerCase(Locale.ROOT).replace('_', ' '));
            o.addProperty("price", price);
            items.add(o);
        });
        m.add("items", items);
        p.heldItem().ifPresent(h -> {
            JsonObject held = new JsonObject();
            held.addProperty("id", h.id());
            held.addProperty("name", h.name());
            held.addProperty("count", h.count());
            Double unit = ItemValues.unitPrice(h.id());
            if (unit != null) held.addProperty("unit", unit);
            m.add("held", held);
        });
        send(p, m);
    }

    /** Ask the client mod to open one of its windows ("market" or "shop"). Returns false if the player has no client mod. */
    public boolean open(CorePlayer p, String screen) {
        if (!clients.contains(p.uuid())) return false;
        JsonObject m = new JsonObject();
        m.addProperty("t", "open");
        m.addProperty("screen", screen);
        send(p, m);
        return true;
    }

    /** A toast for the client (level-up, achievement, friend request…). */
    public void notify(CorePlayer p, String kind, String title, String text) {
        if (!clients.contains(p.uuid())) return;
        JsonObject m = new JsonObject();
        m.addProperty("t", "notify");
        m.addProperty("kind", kind);
        m.addProperty("title", title);
        m.addProperty("text", text);
        send(p, m);
    }

    /** Presentation only. NPC commands and rewards continue to execute on Paper. */
    public void dialogue(CorePlayer p, String title, List<String> lines) {
        if (!nativeClients.contains(p.uuid()) || !env.features.clientLink()) return;
        JsonObject m = new JsonObject(); m.addProperty("t", "dialogue");
        m.addProperty("title", title.substring(0, Math.min(80, title.length())));
        JsonArray text = new JsonArray();
        lines.stream().limit(12).forEach(line -> text.add(line.substring(0, Math.min(200, line.length()))));
        m.add("lines", text); send(p, m);
    }

    /**
     * Events the panel queued for this server (level-ups, achievements, guild changes). Safe to call from any thread:
     * each becomes a toast on the player's client and forces a fresh state.
     */
    public void onPanelEvents(JsonArray events) {
        for (JsonElement element : events) {
            if (!element.isJsonObject()) continue;
            JsonObject n = element.getAsJsonObject();
            String kind = PlayerCache.str(n, "kind", "");
            String uuidText = PlayerCache.str(n, "uuid", "");
            JsonObject d = PlayerCache.obj(n, "data");
            UUID uuid;
            try { uuid = UUID.fromString(uuidText); } catch (IllegalArgumentException e) { continue; }
            String title, text;
            switch (kind) {
                case "level_up" -> { title = "Level up!"; text = ("server".equals(PlayerCache.str(d, "scope", "")) ? "Server level " : "Level ") + (long) PlayerCache.num(d, "level", 0); }
                case "achievement" -> { title = "Achievement unlocked"; text = PlayerCache.str(d, "title", ""); }
                case "guild_join" -> { title = "Guild"; text = "Joined " + PlayerCache.str(d, "guild", "a guild"); }
                case "guild_leave" -> { title = "Guild"; text = "Left " + PlayerCache.str(d, "guild", "your guild"); }
                default -> { continue; }
            }
            final String t = title, x = text, k = kind;
            env.platform.runMain(() -> env.platform.player(uuid).ifPresent(p -> {
                notify(p, k, t, x);
                lastState.remove(uuid);
                cache.refresh(p, info -> pushState(p));
            }));
        }
    }

    /** About once a second: new chunk, new claims, or new state for every client. */
    public void tick() {
        for (UUID id : new ArrayList<>(clients)) {
            Optional<CorePlayer> p = env.platform.player(id);
            if (p.isEmpty()) { disconnected(id); continue; }
            pushClaims(p.get());
            pushState(p.get());
        }
    }
}
