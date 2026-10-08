package net.scopenet.client;

import com.google.gson.*;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.components.*;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.input.MouseButtonEvent;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.Identifier;
import net.minecraft.world.item.ItemStack;
import java.util.*;

/** Native screens share a layout, narration, keyboard focus and explicit financial confirmations. */
public final class CompanionScreen extends Screen {
    private static final List<String> PAGES = List.of("hub", "quests", "guild", "market", "orders", "contracts", "map", "travel", "vaults", "friends", "notifications");
    private static final List<String> TITLES = List.of("Overview", "Quests", "Guild", "Marketplace", "Orders", "Contracts", "World Map", "Travel", "Vaults & Kits", "Friends", "Inbox");
    private static final int ROW = 46, NAV = 108;

    private record Act(String label, Runnable run) {}
    private record Row(String title, String sub, String extra, ItemStack icon, String skin, double progress, int tint, List<Act> acts) {}

    private final List<Row> rows = new ArrayList<>();
    private final Map<String, EditBox> boxes = new HashMap<>();
    private final Map<String, String> texts = new HashMap<>();
    private final Skins skins = new Skins();
    private final MapPanel map = new MapPanel();
    private String page, tab = "", notice = "", confirmation = "";
    private boolean noticeBad, loading, rebuild;
    private Runnable confirmed;
    private JsonElement data = JsonNull.INSTANCE;
    private long loadAfter, refreshAt;
    private int offset, left, top, panelW, panelH, bx, by, bw, visible, sort, kind;
    private String stateSnapshot = "";

    public CompanionScreen(String page) { super(Component.literal("SCOPENET")); this.page = PAGES.contains(page) ? page : "hub"; }

    @Override protected void init() {
        panelW = Math.min(680, width - 12); panelH = Math.min(440, height - 12);
        left = (width - panelW) / 2; top = (height - panelH) / 2;
        bx = left + NAV + 14; by = top + 82; bw = panelW - NAV - 22;
        visible = Math.max(1, (panelH - 82 - 48) / ROW);
        widgets(); load();
    }

    // ---- small helpers --------------------------------------------------------------------------------------------

    private Button button(String text, int x, int y, int w, Runnable run) {
        Button b = Button.builder(Component.literal(text), ignored -> run.run()).bounds(x, y, w, 20).build(); addRenderableWidget(b); return b;
    }
    private int fit(String text) { return font.width(text) + 14; }
    private String text(String key) { return texts.getOrDefault(key, ""); }
    private EditBox box(String key, int x, int y, int w, String hint, int max) {
        EditBox e = new EditBox(font, x, y, w, 18, Component.literal(hint));
        e.setMaxLength(max); e.setValue(text(key)); e.setHint(Component.literal(hint));
        e.setResponder(s -> { texts.put(key, s); if (key.equals("search")) { offset = 0; rebuild = true; } });
        addRenderableWidget(e); boxes.put(key, e); return e;
    }
    private boolean enabled(String target) {
        return switch (target) {
            case "market", "orders", "contracts" -> Model.flag(Companion.model.features, "economy");
            case "guild" -> Model.flag(Companion.model.features, "guilds");
            case "friends" -> Model.flag(Companion.model.features, "social");
            default -> true;
        };
    }
    private String[] tabs() {
        return switch (page) {
            case "quests" -> new String[]{"daily", "weekly", "achievements"};
            case "guild" -> new String[]{"overview", "directory", "treasury", "requests"};
            case "market" -> new String[]{"browse", "listings", "mailbox", "history"};
            case "orders" -> new String[]{"board", "mine", "history"};
            case "contracts" -> new String[]{"active", "recent"};
            case "vaults" -> new String[]{"vaults", "kits", "perks"};
            case "travel" -> new String[]{"places", "requests"};
            default -> new String[0];
        };
    }
    private String tabLabel(String t) { return t.equals("listings") ? "My Listings" : Words.title(t); }
    private String searchHint() {
        return switch (page) {
            case "market" -> "Search listings";
            case "travel", "friends", "guild", "orders" -> "Search, or type a player or place name";
            default -> "Search";
        };
    }
    private void navigate(String target) {
        page = target; tab = ""; offset = 0; sort = 0; kind = 0; data = JsonNull.INSTANCE; notice = ""; texts.put("search", ""); widgets(); load();
    }
    private void say(String message, boolean bad) { notice = message; noticeBad = bad; }

    // ---- widgets ----------------------------------------------------------------------------------------------------

    private void widgets() {
        String focused = ""; int cursor = 0;
        for (var entry : boxes.entrySet()) if (entry.getValue().isFocused()) { focused = entry.getKey(); cursor = entry.getValue().getCursorPosition(); }
        clearWidgets(); rows.clear(); boxes.clear();
        int step = Math.min(24, (panelH - 76) / (PAGES.size() + 1));
        for (int i = 0; i < PAGES.size(); i++) {
            String target = PAGES.get(i);
            Button b = button(TITLES.get(i), left + 8, top + 34 + i * step, NAV, () -> navigate(target));
            b.setHeight(Math.max(14, step - 2)); b.active = !page.equals(target) && enabled(target);
        }
        Button hud = button("HUD Manager", left + 8, top + 34 + PAGES.size() * step + 6, NAV, Companion::openHud);
        hud.setHeight(Math.max(14, step - 2));
        button("Done", left + panelW - 56, top + 8, 48, this::onClose);
        String[] tabs = tabs(); if (tab.isEmpty() && tabs.length > 0) tab = tabs[0];
        if (page.equals("map")) mapToolbar();
        else {
            int searchW = Math.max(80, bw - 112);
            box("search", bx, top + 32, searchW, searchHint(), 80);
            button("<", bx + searchW + 4, top + 31, 20, () -> { offset = Math.max(0, offset - visible); rebuild = true; });
            button(">", bx + searchW + 26, top + 31, 20, () -> { offset = Math.min(Math.max(0, rows.size() - visible), offset + visible); rebuild = true; });
            button("Refresh", bx + searchW + 48, top + 31, bw - searchW - 48, this::load);
            int tx = bx;
            for (String t : tabs) { int w = Math.max(48, fit(tabLabel(t))); Button b = button(tabLabel(t), tx, top + 56, w, () -> { tab = t; offset = 0; data = JsonNull.INSTANCE; texts.put("search", ""); widgets(); load(); }); b.active = !tab.equals(t); tx += w + 2; }
            buildRows(); footer(); rowActions();
        }
        if (!confirmation.isEmpty()) {
            clearWidgets(); boxes.clear();
            button("Confirm", width / 2 - 90, height / 2 + 12, 84, () -> { Runnable r = confirmed; confirmation = ""; confirmed = null; if (r != null) r.run(); rebuild = true; });
            button("Cancel", width / 2 + 6, height / 2 + 12, 84, () -> { confirmation = ""; confirmed = null; rebuild = true; });
        } else if (!focused.isEmpty() && boxes.containsKey(focused)) {
            EditBox e = boxes.get(focused); setFocused(e); e.setCursorPosition(cursor); e.setHighlightPos(cursor);
        }
    }

    private void mapToolbar() {
        int x = bx, y = top + 31;
        x += button("Find Me", x, y, fit("Find Me"), () -> { if (minecraft.player != null) map.findMe(minecraft.player.getX(), minecraft.player.getZ(), MapScene.slug(minecraft.level.dimension().identifier().toString())); }).getWidth() + 3;
        String dim = Words.world(map.slug()); String dimLabel = map.slug().isEmpty() ? "Dimension" : dim;
        x += button(dimLabel, x, y, Math.max(70, fit(dimLabel)), map::cycleDimension).getWidth() + 3;
        x += button("+", x, y, 20, () -> map.zoom(1.6)).getWidth() + 2;
        x += button("-", x, y, 20, () -> map.zoom(1 / 1.6)).getWidth() + 6;
        String[] layers = {"Claims", "Pins", "Players", "Skins"};
        boolean[] on = {map.claims, map.pins, map.players, map.skins};
        for (int i = 0; i < layers.length; i++) {
            int index = i;
            String label = layers[i] + (on[i] ? " ✓" : "");
            if (x + fit(label) > bx + bw) break;
            Button b = button(label, x, y, fit(label), () -> { switch (index) { case 0 -> map.claims = !map.claims; case 1 -> map.pins = !map.pins; case 2 -> map.players = !map.players; default -> map.skins = !map.skins; } rebuild = true; });
            x += b.getWidth() + 2;
        }
    }

    private void footer() {
        int y = top + panelH - 28;
        switch (page) {
            case "market" -> {
                if (tab.equals("browse")) {
                    String[] sorts = {"Newest", "Price: Low", "Price: High", "Ending Soon"}, kinds = {"All", "Buy Now", "Auctions"};
                    int x = bx;
                    x += button("Sort: " + sorts[sort], x, y, fit("Sort: Price: High"), () -> { sort = (sort + 1) % sorts.length; offset = 0; rebuild = true; }).getWidth() + 3;
                    button("Show: " + kinds[kind], x, y, fit("Show: Buy Now"), () -> { kind = (kind + 1) % kinds.length; offset = 0; rebuild = true; });
                }
                int w = fit("List Held Item");
                box("price", bx + bw - w - 4 - 76, y + 1, 76, "Price", 10);
                button("List Held Item", bx + bw - w, y, w, () -> { double n = number("price", "Enter the price to list your held item for."); if (n > 0) confirm("List your held stack for " + Companion.model.money(n) + "?", () -> cmd("market sell " + n)); });
            }
            case "orders" -> {
                int w = fit("Request Held Item");
                box("qty", bx, y + 1, 64, "Amount", 5);
                box("price", bx + 68, y + 1, 84, "Total price", 10);
                button("Request Held Item", bx + 156, y, w, () -> {
                    double n = number("price", "Enter the total price you will pay."), q = number("qty", "Enter how many items you want.");
                    if (n > 0 && q > 0) confirm("Request " + (int) q + " of your held item for " + Companion.model.money(n) + "? The money is held in escrow.", () -> cmd("orders request " + (int) q + " " + n));
                });
            }
            case "guild" -> { if (tab.equals("treasury")) box("price", bx + bw - 90, y + 1, 90, "Amount", 10); }
            case "friends" -> {
                box("message", bx, y + 1, bw - 70, "Message the player named above", 180);
                button("Send", bx + bw - 66, y, 66, () -> { if (name().matches("[A-Za-z0-9_]{1,32}") && !text("message").isBlank()) { cmd("msg " + name() + " " + text("message").trim()); texts.put("message", ""); rebuild = true; } else say("Type a player name in the search box and a message.", true); });
            }
            case "notifications" -> button("Mark All Read", bx + bw - fit("Mark All Read"), y, fit("Mark All Read"), () -> {
                JsonArray ids = new JsonArray(); for (JsonElement e : Model.array(object(), "items")) if (e.isJsonObject() && !Model.flag(e.getAsJsonObject(), "read")) ids.add((long) Model.number(e.getAsJsonObject(), "id"));
                if (ids.isEmpty()) { say("Nothing unread.", false); return; }
                JsonObject a = new JsonObject(); a.add("ids", ids); fetch("notifications_read", a, false);
            });
            default -> {}
        }
    }

    private void rowActions() {
        int y = by;
        for (Row row : rows.subList(Math.min(offset, rows.size()), Math.min(offset + visible, rows.size()))) {
            int n = row.acts.size(), w = n > 1 ? 52 : 62, x = bx + bw - 12 - n * w - (n - 1) * 2;
            for (Act act : row.acts) {
                Button b = button(act.label, x, y + (ROW - 4 - 20) / 2, w, () -> { act.run.run(); rebuild = true; });
                b.active = !loading; b.setTooltip(Tooltip.create(Component.literal(row.title + " · " + act.label)));
                x += w + 2;
            }
            y += ROW;
        }
    }

    // ---- data -------------------------------------------------------------------------------------------------------

    @Override public void tick() {
        if (page.equals("hub") && !stateSnapshot.equals(Companion.model.state.toString())) { stateSnapshot = Companion.model.state.toString(); rebuild = true; }
        if (refreshAt > 0 && System.currentTimeMillis() >= refreshAt && !loading && confirmation.isEmpty() && Companion.connection.ready() && (page.equals("map") || page.equals("quests") || page.equals("contracts"))) { refreshAt = 0; load(); }
        if (loadAfter > 0 && System.currentTimeMillis() >= loadAfter && !loading) { loadAfter = 0; load(); }
        if (rebuild) { rebuild = false; widgets(); }
    }

    private void load() {
        if (!Companion.model.connected) { say("Connect to a SCOPENET server to use these features.", true); return; }
        if (!enabled(page)) { say("This feature is disabled on this server.", true); return; }
        if (page.equals("hub")) { rebuild = true; return; }
        if (Companion.model.modern && !Companion.connection.ready()) { loadAfter = System.currentTimeMillis() + 500; return; }
        String op = switch (page) {
            case "quests" -> tab.equals("achievements") ? "achievements" : "quests";
            case "guild" -> switch (tab) { case "directory" -> "guilds"; case "treasury" -> "guild_bank"; case "requests" -> "guild_requests"; default -> "guild"; };
            case "market" -> tab.equals("history") ? "transactions" : "market";
            case "vaults" -> "storage";
            case "notifications" -> "notifications";
            default -> page;
        };
        fetch(op, new JsonObject(), true);
    }

    private void fetch(String op, JsonObject args, boolean replace) {
        if (loading) return;
        loading = true; say("Loading...", false);
        String expectedPage = page, expectedTab = tab;
        Companion.connection.request(op, args, result -> {
            loading = false;
            if (!page.equals(expectedPage) || !tab.equals(expectedTab)) { loadAfter = System.currentTimeMillis() + 500; rebuild = true; return; }
            if (replace) {
                data = result;
                if (page.equals("map") && result.isJsonObject() && minecraft.player != null)
                    map.setData(result.getAsJsonObject(), minecraft.player.getX(), minecraft.player.getZ(), MapScene.slug(minecraft.level.dimension().identifier().toString()));
                refreshAt = System.currentTimeMillis() + (page.equals("map") ? 8000 : 30000);
            }
            say(replace ? "" : "Done.", false);
            if (!replace && result.isJsonObject()) {
                JsonObject r = result.getAsJsonObject();
                if (r.has("balance")) Companion.model.state.add("balance", r.get("balance"));
                if (Set.of("quest_claim", "notifications_read", "friend_request").contains(op)) loadAfter = System.currentTimeMillis() + 650;
            }
            rebuild = true;
        }, error -> { loading = false; if (!page.equals(expectedPage) || !tab.equals(expectedTab)) loadAfter = System.currentTimeMillis() + 500; else say(error, true); rebuild = true; });
    }

    private JsonObject object() { return data.isJsonObject() ? data.getAsJsonObject() : new JsonObject(); }
    private JsonArray list() { return data.isJsonArray() ? data.getAsJsonArray() : new JsonArray(); }
    private static JsonObject args(String key, String value) { JsonObject a = new JsonObject(); a.addProperty(key, value); return a; }
    private double number(String key, String problem) {
        try { double n = Double.parseDouble(text(key).trim()); if (!Double.isFinite(n) || n <= 0) throw new IllegalArgumentException(); return n; }
        catch (Exception e) { say(problem, true); return -1; }
    }
    private void confirm(String text, Runnable run) { confirmation = text; confirmed = run; rebuild = true; }
    private void cmd(String text) { Connection.command(text); say("Command sent. Check the server response in chat.", false); loadAfter = System.currentTimeMillis() + 1500; }
    private void container(String text) { onClose(); Connection.command(text); }
    private String name() { return text("search").trim().split(" ")[0]; }
    private void named(String command) { if (!name().matches("[A-Za-z0-9_]{1,32}")) { say("Type a player or place name in the search box.", true); return; } cmd(command + " " + name()); }
    private String selfId() { return minecraft != null && minecraft.player != null ? minecraft.player.getUUID().toString() : ""; }

    // ---- rows -------------------------------------------------------------------------------------------------------

    private static final class Builder {
        final String title; String sub = "", extra = "", skin = null; ItemStack icon = ItemStack.EMPTY; double progress = -1; int tint = 0; final List<Act> acts = new ArrayList<>();
        Builder(String title) { this.title = title; }
        Builder sub(String s) { sub = s == null ? "" : s; return this; }
        Builder extra(String s) { extra = s == null ? "" : s; return this; }
        Builder icon(String id) { icon = Presentation.icon(id); return this; }
        Builder stack(ItemStack s) { icon = s; return this; }
        Builder skin(String s) { skin = s == null || s.isBlank() ? null : s; return this; }
        Builder bar(double p) { progress = p; return this; }
        Builder tint(int rgb) { tint = 0xff000000 | rgb; return this; }
        Builder act(String label, Runnable run) { acts.add(new Act(label, run)); return this; }
    }
    private Builder row(String title) { return new Builder(title); }
    private void add(Builder b) { add(b, true); }
    private void add(Builder b, boolean filter) {
        String query = text("search").trim().toLowerCase(Locale.ROOT);
        if (filter && !query.isEmpty() && !(b.title + " " + b.sub + " " + b.extra).toLowerCase(Locale.ROOT).contains(query)) return;
        rows.add(new Row(b.title, b.sub, b.extra, b.icon, b.skin, b.progress, b.tint, List.copyOf(b.acts)));
    }
    private void form(Builder b) { add(b, false); }

    private void buildRows() {
        Model m = Companion.model;
        switch (page) {
            case "hub" -> hubRows();
            case "quests" -> questRows();
            case "guild" -> guildRows();
            case "market" -> marketRows();
            case "orders" -> orderRows();
            case "contracts" -> contractRows();
            case "travel" -> travelRows();
            case "vaults" -> vaultRows();
            case "friends" -> {
                for (JsonElement e : list()) {
                    JsonObject f = e.getAsJsonObject(); String status = Model.text(f, "status"), user = Model.text(f, "username");
                    boolean online = Model.flag(f, "online"), accepted = status.equals("accepted");
                    Builder b = row(user).skin(Model.text(f, "uuid").isBlank() ? user : Model.text(f, "uuid")).icon("minecraft:player_head")
                        .sub(accepted ? (online ? "Online" + (Model.text(f, "playing_on").isBlank() ? "" : " · Playing on " + Model.text(f, "playing_on")) : "Offline") : Words.title(status.replace("pending_", "Pending ")))
                        .tint(accepted ? (online ? 0x55ff55 : 0x707070) : 0xffaa00);
                    int unread = (int) Model.number(f, "unread"); if (unread > 0) b.extra(unread + " unread message" + (unread == 1 ? "" : "s"));
                    if (accepted) { b.act("TPA", () -> cmd("tpa " + user)); b.act("Message", () -> { texts.put("search", user); rebuild = true; }); }
                    else if (status.equals("pending_incoming")) b.act("Accept", () -> fetch("friend_request", args("username", user), false));
                    add(b);
                }
                form(row("Add a Friend").icon("minecraft:paper").sub("Type a player name in the search box, then press Request.").act("Request", () -> fetch("friend_request", args("username", name()), false)));
            }
            case "notifications" -> {
                for (JsonElement e : Model.array(object(), "items")) {
                    JsonObject n = e.getAsJsonObject(); boolean read = Model.flag(n, "read");
                    Builder b = row(Words.sentence(Model.text(n, "title"))).sub(Model.text(n, "body")).extra(Model.text(n, "created_at")).icon("minecraft:paper").tint(read ? 0x404448 : 0xffaa00);
                    if (!read) b.act("Read", () -> { JsonObject a = new JsonObject(); JsonArray ids = new JsonArray(); ids.add((long) Model.number(n, "id")); a.add("ids", ids); fetch("notifications_read", a, false); });
                    add(b);
                }
            }
            case "map" -> {}
            default -> {}
        }
        if (offset > Math.max(0, rows.size() - visible)) offset = Math.max(0, rows.size() - visible);
    }

    private void hubRows() {
        Model m = Companion.model;
        double xp = Model.number(m.state, "xpPct");
        add(row("Welcome, " + Model.text(m.state, "name")).skin(selfId()).icon("minecraft:player_head").sub(m.server)
            .extra("Level " + (int) Model.number(m.state, "level") + " · " + Math.round(xp) + "% to next level · Server level " + (int) Model.number(m.state, "serverLevel")).bar(xp / 100).tint(0x55ff55));
        if (Model.flag(m.features, "economy")) add(row("Wallet").icon("minecraft:gold_ingot").sub(m.money(Model.number(m.state, "balance"))).act("Market", () -> navigate("market")));
        JsonObject pinned = Companion.pinnedQuest;
        if (pinned != null) {
            JsonObject def = Model.object(pinned, "quest"); double target = Math.max(1, Model.number(def, "target_count")), done = Model.number(pinned, "progress");
            add(row("Tracked Quest: " + Model.text(def, "title")).icon("minecraft:book").sub(Model.text(def, "description")).extra((int) done + " / " + (int) target + " · " + (int) Model.number(def, "xp_reward") + " XP").bar(done / target).act("Quests", () -> navigate("quests")));
        } else {
            JsonObject q = Model.object(m.state, "daily");
            add(row("Daily Quests").icon("minecraft:book").sub((int) Model.number(q, "done") + " of " + (int) Model.number(q, "total") + " done today").bar(Model.number(q, "done") / Math.max(1, Model.number(q, "total"))).act("Quests", () -> navigate("quests")));
        }
        if (Model.flag(m.features, "economy")) {
            add(row("Contracts").icon("minecraft:writable_book").sub("Daily jobs that pay: gather items or hunt mobs.").act("Open", () -> navigate("contracts")));
            add(row("Buy Orders").icon("minecraft:chest").sub("Ask for items, or fill what others want.").act("Open", () -> navigate("orders")));
        }
        if (Model.flag(m.features, "guilds")) add(row("Guild").icon("minecraft:shield").sub(Model.text(Model.object(m.state, "guild"), "name").isBlank() ? "You are not in a guild yet." : Model.text(Model.object(m.state, "guild"), "name")).act("Open", () -> navigate("guild")));
        add(row("World Map").icon("minecraft:filled_map").sub("Terrain, players, claims and places, live.").act("Map", () -> navigate("map")));
    }

    private void questRows() {
        List<JsonObject> source = new ArrayList<>();
        for (JsonElement e : list()) if (e.isJsonObject()) source.add(e.getAsJsonObject());
        if (tab.equals("achievements")) {
            for (JsonObject q : source) {
                boolean got = Model.flag(q, "unlocked");
                add(row(Model.text(q, "title")).icon(Model.text(q, "icon_item").isBlank() ? "minecraft:nether_star" : Model.text(q, "icon_item")).sub(Model.text(q, "description")).extra(got ? "Unlocked" : "Locked").tint(got ? 0x55ff55 : 0x404448));
            }
            return;
        }
        List<JsonObject> mine = new ArrayList<>();
        for (JsonObject q : source) if (Model.text(Model.object(q, "quest"), "period").equals(tab)) mine.add(q);
        // Ready to claim first, then in progress, then claimed.
        mine.sort(Comparator.comparingInt(q -> Model.flag(q, "claimed") ? 2 : Model.flag(q, "completed") ? 0 : 1));
        for (JsonObject q : mine) {
            JsonObject def = Model.object(q, "quest"); String id = Model.text(def, "id");
            boolean claimed = Model.flag(q, "claimed"), claim = Model.flag(q, "completed") && !claimed;
            boolean tracked = Companion.pinnedQuest != null && Model.text(Model.object(Companion.pinnedQuest, "quest"), "id").equals(id);
            double target = Math.max(1, Model.number(def, "target_count")), done = Math.min(target, Model.number(q, "progress"));
            if (tracked) Companion.pinnedQuest = q;
            Builder b = row(Model.text(def, "title")).icon(claim ? "minecraft:experience_bottle" : "minecraft:book").sub(Model.text(def, "description"))
                .extra((int) done + " / " + (int) target + " · " + (int) Model.number(def, "xp_reward") + " XP" + (claimed ? " · Claimed" : tracked ? " · Tracked on HUD" : "")).bar(done / target)
                .tint(claim ? 0x55ff55 : claimed ? 0x404448 : tracked ? 0xffaa00 : 0);
            if (claim) b.act("Claim", () -> fetch("quest_claim", args("id", id), false));
            else if (!claimed) b.act(tracked ? "Untrack" : "Track", () -> { Companion.pinnedQuest = tracked ? null : q; say(tracked ? "Quest removed from your HUD." : "Quest pinned to your HUD.", false); });
            add(b);
        }
    }

    private void guildRows() {
        JsonObject g = object(); Model m = Companion.model;
        if (tab.equals("requests")) {
            for (JsonElement e : Model.array(g, "requests")) {
                JsonObject r = e.getAsJsonObject(); String who = Model.text(r, "name");
                add(row(who).skin(Model.text(r, "uuid")).icon("minecraft:player_head").sub(Model.text(r, "message")).extra(Model.text(r, "created_at"))
                    .act("Approve", () -> confirm("Approve " + who + "?", () -> cmd("guild approve " + who))).act("Reject", () -> confirm("Reject " + who + "?", () -> cmd("guild reject " + who))));
            }
            return;
        }
        if (tab.equals("directory")) {
            for (JsonElement e : list()) { JsonObject r = e.getAsJsonObject(); add(row("[" + Model.text(r, "tag") + "] " + Model.text(r, "name")).icon("minecraft:shield").sub(Model.text(r, "description")).extra((int) Model.number(r, "member_count") > 0 ? (int) Model.number(r, "member_count") + " members" : "").act("Request", () -> cmd("guild join " + Model.text(r, "tag")))); }
            return;
        }
        add(row(Model.text(g, "name").isBlank() ? "Your Guild" : Model.text(g, "name")).icon("minecraft:shield").sub(Model.text(g, "description")).tint(0xfb7185).act("Home", () -> container("guild home")));
        if (tab.equals("treasury")) {
            add(row("Treasury Balance").icon("minecraft:gold_block").sub(m.money(Model.number(g, "balance"))).tint(0xffaa00));
            form(row("Deposit").icon("minecraft:gold_ingot").sub("Enter an amount below. The server checks your permissions.").act("Deposit", () -> { double n = number("price", "Enter an amount to deposit."); if (n > 0) confirm("Deposit " + m.money(n) + "?", () -> cmd("guild bank deposit " + n)); }));
            form(row("Withdraw").icon("minecraft:gold_nugget").sub("Needs the guild spending permission.").act("Withdraw", () -> { double n = number("price", "Enter an amount to withdraw."); if (n > 0) confirm("Withdraw " + m.money(n) + "?", () -> cmd("guild bank withdraw " + n)); }));
            for (JsonElement e : Model.array(g, "transactions")) { JsonObject r = e.getAsJsonObject(); add(row(Words.title(Model.text(r, "kind"))).icon("minecraft:paper").sub(m.money(Model.number(r, "amount"))).extra(Model.text(r, "note"))); }
            return;
        }
        if (!Model.text(g, "motd").isBlank()) add(row("Message of the Day").icon("minecraft:oak_sign").sub(Model.text(g, "motd")));
        for (JsonElement e : Model.array(g, "members")) { JsonObject r = e.getAsJsonObject(); add(row(Model.text(r, "name")).skin(Model.text(r, "uuid")).icon("minecraft:player_head").sub(Words.title(Model.text(r, "role")))); }
        for (JsonElement e : Model.array(g, "posts")) { JsonObject r = e.getAsJsonObject(); add(row(Model.text(r, "title")).icon("minecraft:writable_book").sub(Model.text(r, "content"))); }
        form(row("Claim This Chunk").icon("minecraft:map").sub("Land rules and costs apply.").act("Claim", () -> confirm("Claim your current chunk?", () -> cmd("guild claim"))));
        form(row("Invite a Player").icon("minecraft:player_head").sub("Type a player name in the search box.").act("Invite", () -> named("guild invite")));
    }

    private void marketRows() {
        Model m = Companion.model;
        if (tab.equals("history")) {
            for (JsonElement e : list()) { JsonObject t = e.getAsJsonObject(); add(row(Words.sentence(Model.text(t, "description"))).icon("minecraft:gold_ingot").sub(m.money(Model.number(t, "amount"))).extra(Model.text(t, "created_at"))); }
            return;
        }
        if (tab.equals("mailbox")) { form(row("Collect Your Items").icon("minecraft:chest").sub("Auction wins and returned listings are delivered here.").act("Collect", () -> container("market claim"))); return; }
        List<JsonObject> listings = new ArrayList<>();
        for (JsonElement e : list()) {
            if (!e.isJsonObject()) continue;
            JsonObject l = e.getAsJsonObject(); boolean mine = Model.text(l, "seller_uuid").equalsIgnoreCase(selfId()), auction = Model.text(l, "kind").equals("auction");
            if (tab.equals("listings") ? !mine : kind == 1 && auction || kind == 2 && !auction) continue;
            listings.add(l);
        }
        Comparator<JsonObject> byPrice = Comparator.comparingDouble(l -> Model.text(l, "kind").equals("auction") ? Model.number(l, "min_next_bid") : Model.number(l, "price"));
        switch (sort) {
            case 1 -> listings.sort(byPrice);
            case 2 -> listings.sort(byPrice.reversed());
            case 3 -> listings.sort(Comparator.comparingLong(l -> { long s = Words.secondsUntil(Model.text(l, "ends_at"), System.currentTimeMillis()); return s < 0 ? Long.MAX_VALUE : s; }));
            default -> {}
        }
        for (JsonObject l : listings) {
            long id = (long) Model.number(l, "id"); boolean auction = Model.text(l, "kind").equals("auction"), cancel = tab.equals("listings");
            double price = auction ? Model.number(l, "min_next_bid") : Model.number(l, "price");
            int amount = (int) Model.number(l, "amount"); String item = Model.text(l, "item_name").isBlank() ? Words.item(Model.text(l, "item_id")) : Presentation.plain(Model.text(l, "item_name"));
            String sub = auction ? "Bid from " + m.money(price) + (Model.number(l, "bid_count") > 0 ? " · " + (int) Model.number(l, "bid_count") + " bids" : " · No bids yet") : m.money(price) + (amount > 1 ? " · " + m.money(price / amount) + " each" : "");
            long left = auction ? Words.secondsUntil(Model.text(l, "ends_at"), System.currentTimeMillis()) : -1;
            String extra = "Seller: " + Model.text(l, "seller_name") + (Model.text(l, "seller_guild").isBlank() ? "" : " [" + Model.text(l, "seller_guild") + "]") + (auction ? " · Ends in " + (left < 0 ? "?" : Words.duration(left)) : "");
            Builder b = row((amount > 1 ? amount + "× " : "") + item).icon(Model.text(l, "item_id")).skin(Model.text(l, "seller_name")).sub(sub).extra(extra).tint(auction ? 0xffaa00 : 0x55ff55);
            String verb = cancel ? "Cancel" : auction ? "Bid" : "Buy";
            b.act(verb, () -> confirm((cancel ? "Cancel your listing of " + item + "?" : auction ? "Bid " + m.money(price) + " on " + item + "?" : "Buy " + (amount > 1 ? amount + "× " : "") + item + " for " + m.money(price) + "?"), () -> cmd("market " + (cancel ? "cancel " : auction ? "bid " : "buy ") + id)));
            add(b);
        }
    }

    private void orderRows() {
        Model m = Companion.model; JsonObject o = object();
        if (!Model.flag(o, "enabled") && !o.isEmpty()) { form(row("Buy Orders Are Off").icon("minecraft:barrier").sub("An admin has switched this feature off.")); return; }
        JsonObject rules = Model.object(o, "rules");
        if (tab.equals("board") && !rules.isEmpty())
            form(row("How Buy Orders Work").icon("minecraft:writable_book").sub("Request items and set a price. A player picks it up and hands the items in; you get them in your vault and they get paid.")
                .extra((int) Model.number(rules, "fee_percent") + "% fee · Pick-ups last " + (int) Model.number(rules, "claim_minutes") + " min · Up to " + (int) Model.number(rules, "max_open") + " open orders"));
        JsonArray source = tab.equals("history") ? Model.array(o, "history") : Model.array(o, "orders");
        for (JsonElement e : source) {
            if (!e.isJsonObject()) continue;
            JsonObject r = e.getAsJsonObject(); boolean mine = Model.flag(r, "mine"), byMe = Model.flag(r, "claimed_by_me"), taken = Model.text(r, "status").equals("claimed");
            if (tab.equals("mine") && !mine && !byMe) continue;
            long id = (long) Model.number(r, "id"); int amount = (int) Model.number(r, "amount");
            String item = Model.text(r, "item_name").isBlank() ? Words.item(Model.text(r, "item_id")) : Presentation.plain(Model.text(r, "item_name"));
            String state = tab.equals("history") ? Words.title(Model.text(r, "resolved")) : mine ? (taken ? "Yours · Picked up by " + Model.text(r, "claimer_name") : "Yours · Waiting for a player") : byMe ? "Picked up by you" : taken ? "Picked up by " + Model.text(r, "claimer_name") : "Open";
            Builder b = row(amount + "× " + item).icon(Model.text(r, "item_id")).skin(Model.text(r, "buyer_name")).sub(m.money(Model.number(r, "total")) + " total · " + m.money(Model.number(r, "each")) + " each")
                .extra("From " + Model.text(r, "buyer_name") + " · " + state).tint(mine ? 0x55aaff : byMe ? 0x55ff55 : taken ? 0x404448 : 0xffaa00);
            if (!tab.equals("history")) {
                if (mine) b.act("Cancel", () -> confirm("Cancel your order for " + amount + "× " + item + "? Your escrowed money comes back.", () -> cmd("orders cancel " + id)));
                else if (byMe) { b.act("Fill", () -> container("orders fill " + id)); b.act("Drop", () -> cmd("orders drop " + id)); }
                else if (!taken) b.act("Pick Up", () -> confirm("Pick up the order for " + amount + "× " + item + "? You then have " + (int) Model.number(rules, "claim_minutes") + " minutes to hand the items in.", () -> cmd("orders pickup " + id)));
            }
            add(b);
        }
    }

    private void contractRows() {
        Model m = Companion.model; JsonObject o = object();
        if (!o.isEmpty() && !Model.flag(o, "enabled")) { form(row("Contracts Are Off").icon("minecraft:barrier").sub("An admin has switched this feature off.")); return; }
        JsonObject stats = Model.object(o, "stats");
        if (tab.equals("active") && !stats.isEmpty())
            form(row("Today").icon("minecraft:clock").sub((int) Model.number(stats, "done_today") + (Model.number(stats, "daily_limit") > 0 ? " of " + (int) Model.number(stats, "daily_limit") : "") + " completed · " + (int) Model.number(stats, "rerolls_left") + " rerolls left")
                .extra("All time: " + (int) Model.number(stats, "completed") + " contracts · " + m.money(Model.number(stats, "earned")) + " earned").tint(0x55ff55));
        for (JsonElement e : Model.array(o, tab.equals("recent") ? "recent" : "contracts")) {
            if (!e.isJsonObject()) continue;
            JsonObject c = e.getAsJsonObject(); long id = (long) Model.number(c, "id"); boolean gather = Model.text(c, "kind").equals("gather");
            double required = Math.max(1, Model.number(c, "required")), progress = Math.min(required, Model.number(c, "progress"));
            long left = Words.secondsUntil(Model.text(c, "expires_at"), System.currentTimeMillis());
            String reward = m.money(Model.number(c, "reward")) + (Model.number(c, "bonus") > 0 ? " + " + m.money(Model.number(c, "bonus")) + " bonus" : "");
            Builder b = row(Words.sentence(Model.text(c, "title"))).icon(gather ? Model.text(c, "target") : "minecraft:iron_sword")
                .sub((int) progress + " / " + (int) required + (gather ? " submitted" : " done") + " · Reward " + reward)
                .extra(tab.equals("recent") ? "Completed" : "Expires in " + (left < 0 ? "?" : Words.duration(left)) + (gather ? " · Hold the items and press Submit" : " · Counts automatically")).bar(progress / required)
                .tint(tab.equals("recent") ? 0x404448 : gather ? 0xffaa00 : 0xff5555);
            if (!tab.equals("recent")) {
                if (gather) b.act("Submit", () -> container("contracts submit " + id));
                b.act("Drop", () -> confirm("Drop this contract? It counts against your daily re-rolls.", () -> cmd("contracts abandon " + id)));
            }
            add(b);
        }
    }

    private void travelRows() {
        if (tab.equals("requests")) {
            form(row("Teleport Request").icon("minecraft:ender_pearl").sub("Type a player name in the search box.").act("Send", () -> named("tpa")));
            form(row("Accept Request").icon("minecraft:lime_dye").sub("Accept the teleport request someone sent you.").act("Accept", () -> cmd("tpaccept")));
            form(row("Trade").icon("minecraft:emerald").sub("Type a player name in the search box. The server opens the trade window.").act("Trade", () -> { onClose(); named("trade"); }));
            return;
        }
        form(row("Spawn").icon("minecraft:compass").sub("Server permissions and cooldowns apply.").act("Go", () -> container("spawn")));
        form(row("Back").icon("minecraft:recovery_compass").sub("Return to where you were before you last teleported.").act("Go", () -> container("back")));
        form(row("Random Teleport").icon("minecraft:ender_eye").sub("A random safe spot in the wild.").act("Go", () -> container("rtp")));
        double px = minecraft.player == null ? 0 : minecraft.player.getX(), pz = minecraft.player == null ? 0 : minecraft.player.getZ();
        String here = minecraft.level == null ? "" : minecraft.level.dimension().identifier().toString();
        List<JsonObject> places = new ArrayList<>();
        for (JsonElement e : Model.array(object(), "places")) if (e.isJsonObject()) places.add(e.getAsJsonObject());
        places.sort(Comparator.comparing((JsonObject p) -> Model.text(p, "kind")).thenComparingDouble(p -> Math.hypot(Model.number(p, "x") - px, Model.number(p, "z") - pz)));
        for (JsonObject p : places) {
            String kind = Model.text(p, "kind"), name = Model.text(p, "name"), world = Model.text(p, "world");
            boolean near = MapScene.slug(world).equals(MapScene.slug(here)) || Words.world(world).equals(Words.world(here));
            double dist = Math.hypot(Model.number(p, "x") - px, Model.number(p, "z") - pz);
            add(row(Words.title(name)).icon(kind.equals("home") ? "minecraft:red_bed" : "minecraft:ender_pearl").sub(Words.title(kind) + " · " + Words.world(world))
                .extra("X " + (int) Model.number(p, "x") + "  Z " + (int) Model.number(p, "z") + (near ? " · " + Words.number(Math.round(dist)) + " blocks away" : "")).tint(kind.equals("home") ? 0xa78bfa : 0x38bdf8)
                .act("Go", () -> container(kind + " " + name)));
        }
        form(row("Set Home Here").icon("minecraft:white_bed").sub("Type a name in the search box, then press Save.").act("Save", () -> named("sethome")));
    }

    private void vaultRows() {
        if (tab.equals("kits")) {
            for (JsonElement e : Model.array(object(), "kits")) { JsonObject k = e.getAsJsonObject(); String kit = Model.text(k, "name"); add(row(Words.title(kit)).icon("minecraft:bundle").sub(Model.text(k, "description")).act("Claim", () -> confirm("Claim the " + Words.title(kit) + " kit?", () -> cmd("kit " + Model.text(k, "id"))))); }
        } else if (tab.equals("perks")) {
            String[][] perks = {{"heal", "Restore your health.", "minecraft:golden_apple"}, {"feed", "Fill your hunger bar.", "minecraft:cooked_beef"}, {"fly", "Toggle flight.", "minecraft:elytra"}, {"repair", "Repair the item in your hand.", "minecraft:anvil"}, {"craft", "Open a crafting table anywhere.", "minecraft:crafting_table"}};
            for (String[] p : perks) add(row(Words.title(p[0])).icon(p[2]).sub(p[1] + " Permissions and cooldowns apply.").act("Use", () -> container(p[0])));
        } else {
            for (JsonElement e : Model.array(object(), "vaults")) { int n = e.getAsInt(); add(row("Vault " + n).icon("minecraft:chest").sub("Server-owned storage that follows you.").act("Open", () -> container("vault " + n))); }
            if (Model.flag(object(), "echest")) add(row("Ender Chest").icon("minecraft:ender_chest").sub("Your personal ender storage.").act("Open", () -> container("echest")));
        }
    }

    // ---- input ------------------------------------------------------------------------------------------------------

    @Override public boolean mouseScrolled(double x, double y, double horizontal, double vertical) {
        if (!confirmation.isEmpty()) return true;
        if (page.equals("map") && map.mouseScrolled(x, y, vertical)) return true;
        offset = Math.clamp(offset - (int) Math.signum(vertical), 0, Math.max(0, rows.size() - visible)); rebuild = true; return true;
    }
    @Override public boolean mouseClicked(MouseButtonEvent e, boolean doubled) {
        if (confirmation.isEmpty() && page.equals("map")) return super.mouseClicked(e, doubled) || map.mouseClicked(e.x(), e.y(), doubled);
        return super.mouseClicked(e, doubled);
    }
    @Override public boolean mouseDragged(MouseButtonEvent e, double dx, double dy) { return page.equals("map") && map.mouseDragged(dx, dy) || super.mouseDragged(e, dx, dy); }
    @Override public boolean mouseReleased(MouseButtonEvent e) { return page.equals("map") && map.mouseReleased(e.x(), e.y()) || super.mouseReleased(e); }

    // ---- drawing ----------------------------------------------------------------------------------------------------

    @Override public void extractRenderState(GuiGraphicsExtractor g, int mx, int my, float delta) {
        int accent = Companion.preferences.accentColor();
        g.fill(0, 0, width, height, 0x99000000);
        Look.panel(g, left, top, panelW, panelH, Companion.preferences.opacity);
        g.fill(left + NAV + 12, top + 30, left + NAV + 13, top + panelH - 8, 0xff2a2e33);
        g.text(font, font.plainSubstrByWidth(Companion.preferences.brand, NAV), left + 10, top + 12, accent);
        String title = TITLES.get(PAGES.indexOf(page));
        g.text(font, title, bx, top + 12, Look.TEXT);
        if (Companion.model.connected) { String s = Companion.model.server; g.text(font, font.plainSubstrByWidth(s, 160), left + panelW - 64 - Math.min(160, font.width(s)), top + 12, Look.DIM); }
        if (page.equals("map")) {
            int mapTop = top + 56, mapH = panelH - 56 - 34;
            map.render(g, font, bx, mapTop, bw, mapH, mx, my, selfId());
            g.text(font, "Drag to pan · Scroll to zoom · Double-click to zoom in · Click a player, pin or claim for details", bx, top + panelH - 24, Look.DIM);
        } else {
            Look.well(g, bx, by, bw, visible * ROW);
            int y = by;
            for (int i = offset; i < Math.min(rows.size(), offset + visible); i++) { row(g, rows.get(i), y, mx, my); y += ROW; }
            if (rows.isEmpty()) g.text(font, loading ? "Loading..." : "Nothing to show yet.", bx + 10, by + 14, Look.MUTED);
            if (rows.size() > visible) scrollbar(g);
            if (rows.size() > 0) { String count = (offset + 1) + "–" + Math.min(rows.size(), offset + visible) + " of " + rows.size(); g.text(font, count, bx + bw - font.width(count), top + 62, Look.DIM); }
        }
        if (!notice.isEmpty() && confirmation.isEmpty()) g.text(font, font.plainSubstrByWidth(notice, bw), bx, top + panelH - 40, noticeBad ? Look.BAD : accent);
        if (!confirmation.isEmpty()) {
            g.fill(0, 0, width, height, 0xcc000000);
            int w = Math.min(width - 20, 330); List<net.minecraft.util.FormattedCharSequence> lines = font.split(Component.literal(confirmation), w - 28);
            int h = 62 + lines.size() * 11;
            Look.panel(g, width / 2 - w / 2, height / 2 - 40, w, h, 1f);
            for (int i = 0; i < lines.size(); i++) g.text(font, lines.get(i), width / 2 - w / 2 + 14, height / 2 - 30 + i * 11, Look.TEXT);
        }
        super.extractRenderState(g, mx, my, delta);
    }

    private void scrollbar(GuiGraphicsExtractor g) {
        int trackH = visible * ROW, x = bx + bw - 5;
        g.fill(x, by, x + 3, by + trackH, 0xff1a1d21);
        int thumb = Math.max(16, trackH * visible / rows.size()), at = (trackH - thumb) * offset / Math.max(1, rows.size() - visible);
        g.fill(x, by + at, x + 3, by + at + thumb, 0xff6b7076);
    }

    private void row(GuiGraphicsExtractor g, Row row, int y, int mx, int my) {
        int accent = row.tint != 0 ? row.tint : Companion.preferences.accentColor();
        boolean hover = mx >= bx && mx < bx + bw - 8 && my >= y && my < y + ROW - 2;
        g.fill(bx, y, bx + bw, y + ROW - 2, hover ? Look.ROW_HOVER : Look.ROW);
        g.fill(bx, y, bx + 2, y + ROW - 2, accent);
        Identifier head = row.skin == null ? null : skins.head(Companion.model.panel, row.skin);
        // A person's row (friends, members) shows their head large; a thing's row (a listing, an order) shows the item large and the person small.
        boolean personRow = row.skin != null && (row.icon.isEmpty() || row.icon.is(net.minecraft.world.item.Items.PLAYER_HEAD));
        int tx = bx + 34;
        if (personRow) {
            if (head != null) { Look.blit(g, head, bx + 8, y + 7, 24, 24, 0, 0, 1, 1); g.outline(bx + 8, y + 7, 24, 24, 0xff000000); }
            else g.fill(bx + 8, y + 7, bx + 32, y + 31, 0xff2a2f35);
        } else if (!row.icon.isEmpty()) {
            g.item(row.icon, bx + 12, y + 11);
            if (mx >= bx + 8 && mx < bx + 32 && my >= y && my < y + ROW - 2) g.setTooltipForNextFrame(font, row.icon, mx, my);
        }
        int extraX = tx;
        if (!personRow && head != null) { Look.blit(g, head, tx, y + 27, 10, 10, 0, 0, 1, 1); extraX = tx + 13; }
        int actions = 0; for (int i = 0; i < row.acts.size(); i++) actions += (row.acts.size() > 1 ? 54 : 64);
        int tw = Math.max(40, bw - (tx - bx) - actions - 20);
        g.text(font, font.plainSubstrByWidth(row.title, tw), tx, y + 6, Look.TEXT);
        if (!row.sub.isEmpty()) g.text(font, font.plainSubstrByWidth(row.sub, tw), tx, y + 17, Look.MUTED);
        if (!row.extra.isEmpty()) g.text(font, font.plainSubstrByWidth(row.extra, tw - (extraX - tx)), extraX, y + 28, Look.DIM);
        if (row.progress >= 0) Look.bar(g, tx, y + ROW - 6, tw, 2, row.progress, Look.alpha(accent, 1));
    }

    @Override public void removed() { skins.close(); map.close(); }
    @Override public boolean isPauseScreen() { return false; }
}
