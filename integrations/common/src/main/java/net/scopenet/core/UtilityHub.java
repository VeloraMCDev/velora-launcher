package net.scopenet.core;

import com.google.gson.JsonObject;
import net.scopenet.integration.ChunkCheckResult;
import net.scopenet.integration.ClaimIndex;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;
import java.util.function.Supplier;

/**
 * The player utilities the panel configures: /heal, /feed, /fly, /vault, /echest, /kit, /customitem, /adminclaim, plus the
 * "you are entering…" banner. One implementation serves Paper and Fabric; each platform supplies the {@link Platform} hooks.
 */
public final class UtilityHub {
    private final Env env;
    private final Supplier<Utilities> settings;
    private final UtilityStore store;
    private final VaultDeliveries vaultDeliveries;
    /** Called after a player puts on or takes off a cosmetic with /cosmetic, so the platform can show the change at once. */
    public volatile java.util.function.Consumer<CorePlayer> cosmeticsChanged = p -> { };
    /** Players who switched flight on with /fly. */
    private final Set<UUID> flying = ConcurrentHashMap.newKeySet();
    /** What each player last stood in, so the banner shows once per change. */
    private final Map<UUID, String> standingIn = new HashMap<>();

    public UtilityHub(Env env, Supplier<Utilities> settings, UtilityStore store) {
        this.env = env;
        this.settings = settings;
        this.store = store;
        this.vaultDeliveries = new VaultDeliveries(env, settings, store);
    }

    // ---- commands ----------------------------------------------------------------------------------------

    public List<CoreCommand> commands() {
        List<CoreCommand> out = new ArrayList<>();
        out.add(new Cmd("heal", List.of(), "scopenet.command.heal", (p, a) -> restore(p, a, true), (p, a) -> names(p, a)));
        out.add(new Cmd("feed", List.of(), "scopenet.command.feed", (p, a) -> restore(p, a, false), (p, a) -> a.length == 1 && p.hasPermission("scopenet.command.feed.others") ? onlineNames() : List.of()));
        out.add(new Cmd("fly", List.of(), "", (p, a) -> fly(p, a), (p, a) -> a.length == 1 ? List.of("on", "off") : List.of()));
        out.add(new Cmd("vault", List.of("pv"), "scopenet.command.vault", this::vault, (p, a) -> a.length == 1 ? java.util.stream.IntStream.rangeClosed(1, settings.get().vault.count()).filter(n -> mayOpenVault(p, n, settings.get().vault)).mapToObj(Integer::toString).toList() : List.of()));
        out.add(new Cmd("echest", List.of("ec", "enderchest"), "scopenet.command.echest", this::echest, (p, a) -> List.of()));
        out.add(new Cmd("kit", List.of("kits"), "scopenet.command.kit", this::kit, (p, a) -> {
            if (a.length != 1) return List.of();
            List<String> choices = usableKits(p);
            if (p.hasPermission("scopenet.admin.kits")) choices.add("create");
            return choices;
        }));
        out.add(new Cmd("quests", List.of("quest", "dailies"), "scopenet.command.quests", this::quests, (p, a) -> a.length == 1 ? onlineNames() : List.of()));
        out.addAll(new CosmeticCommands(env, p -> cosmeticsChanged.accept(p)).build());
        out.add(new Cmd("customitem", List.of("citem"), "scopenet.admin.customitem", this::customItem,
                (p, a) -> a.length == 1 ? List.of("give", "list") : a.length == 2 && a[0].equalsIgnoreCase("give") ? onlineNames()
                        : a.length == 3 && a[0].equalsIgnoreCase("give") ? java.util.stream.Stream.concat(settings.get().customItems.keySet().stream(), settings.get().content.keySet().stream()).sorted().toList() : List.of()));
        out.add(new Cmd("adminclaim", List.of("aclaim"), "scopenet.admin.claims", this::adminClaim,
                (p, a) -> a.length == 1 ? List.of("create", "add", "remove", "delete", "rename", "describe", "color", "list", "info")
                        : a.length == 2 && Set.of("add", "remove", "delete", "rename", "describe", "color").contains(a[0].toLowerCase(java.util.Locale.ROOT)) ? env.panel.claimNames(true) : List.of()));
        return out;
    }

    private List<String> onlineNames() {
        List<String> n = new ArrayList<>();
        for (CorePlayer p : env.platform.online()) n.add(p.name());
        return n;
    }

    private List<String> names(CorePlayer who, String[] a) { return a.length == 1 && who.hasPermission("scopenet.command.heal.others") ? onlineNames() : List.of(); }

    private static String waitText(long ms) {
        long s = Math.max(1, (ms + 999) / 1000);
        if (s >= 86_400) return (s / 86_400) + "d " + ((s % 86_400) / 3600) + "h";
        if (s >= 3600) return (s / 3600) + "h " + ((s % 3600) / 60) + "m";
        if (s >= 60) return (s / 60) + "m " + (s % 60) + "s";
        return s + "s";
    }

    /** Checks and starts a cooldown. False (with a message) if the player has to wait. */
    private boolean cooled(CorePlayer p, String what, long seconds) {
        if (seconds <= 0 || p.hasPermission("scopenet.cooldown.bypass")) return true;
        long left = store.remainingMs(p.uuid(), what, seconds * 1000, env.clock.getAsLong());
        if (left > 0) { p.send(Format.RED + "You can use /" + what + " again in " + waitText(left) + "."); return false; }
        return true;
    }

    // ---- /heal and /feed ---------------------------------------------------------------------------------

    private void restore(CorePlayer p, String[] args, boolean heal) {
        String name = heal ? "heal" : "feed";
        Utilities u = settings.get();
        if (!(heal ? u.heal.enabled() : u.feed.enabled())) { p.send(Format.RED + "/" + name + " is switched off on this server."); return; }
        CorePlayer target = p;
        if (args.length > 0 && !args[0].equalsIgnoreCase(p.name())) {
            if (!p.hasPermission("scopenet.command." + name + ".others")) { p.send(Format.RED + "You can only /" + name + " yourself."); return; }
            target = env.platform.playerByName(args[0]).orElse(null);
            if (target == null) { p.send(Format.RED + "No player called " + args[0] + " is online."); return; }
        }
        boolean self = target == p;
        // Healing someone else is a staff favour: no cooldown for it.
        if (self && !cooled(p, name, heal ? u.heal.cooldownSecs() : u.feed.cooldownSecs())) return;
        if (heal) env.platform.heal(target.uuid(), u.heal.amount()); else env.platform.feed(target.uuid(), u.feed.amount());
        if (self) store.stamp(p.uuid(), name, env.clock.getAsLong());
        String what = heal ? (u.heal.amount() <= 0 ? "to full health" : "by " + trim(u.heal.amount() / 2) + " hearts") : "fed";
        target.send(Format.GREEN + (heal ? "You were healed " : "You were ") + what + ".");
        if (!self) p.send(Format.GREEN + target.name() + " was " + (heal ? "healed." : "fed."));
    }

    private static String trim(double v) { return v == Math.rint(v) ? Long.toString((long) v) : String.format(java.util.Locale.ROOT, "%.1f", v); }

    // ---- /fly --------------------------------------------------------------------------------------------

    private boolean inOwnGuildLand(CorePlayer p) {
        Pos pos = p.pos();
        ChunkCheckResult r = env.panel.check(pos.world(), Math.floorDiv(pos.blockX(), 16), Math.floorDiv(pos.blockZ(), 16), p.uuid());
        ClaimIndex.ClaimInfo info = env.panel.info(pos.world(), Math.floorDiv(pos.blockX(), 16), Math.floorDiv(pos.blockZ(), 16));
        return r.claimed() && r.allowed() && (info == null || !info.admin());
    }

    private void fly(CorePlayer p, String[] args) {
        if (!settings.get().fly) { p.send(Format.RED + "/fly is switched off on this server."); return; }
        boolean free = p.hasPermission("free.fly"), guild = p.hasPermission("guild.fly");
        if (!free && !guild) { p.send(Format.RED + "You don't have permission to fly."); return; }
        boolean want = args.length > 0 ? args[0].equalsIgnoreCase("on") : !flying.contains(p.uuid());
        if (args.length > 0 && !want && !args[0].equalsIgnoreCase("off")) { p.send(Format.YELLOW + "Usage: /fly [on|off]"); return; }
        if (want && !free && !inOwnGuildLand(p)) {
            p.send(Format.RED + "You can only fly inside your own guild's land.");
            return;
        }
        if (want) flying.add(p.uuid()); else flying.remove(p.uuid());
        env.platform.setFlight(p.uuid(), want);
        p.send(want ? Format.GREEN + "Flight enabled." + (free ? "" : Format.GRAY + " It switches off when you leave your guild's land.") : Format.YELLOW + "Flight disabled.");
    }

    // ---- /vault and /echest ------------------------------------------------------------------------------

    private boolean mayOpenVault(CorePlayer p, int n, Utilities.Vault v) { return n <= v.freeCount() || p.hasPermission("scopenet.vault." + n) || p.hasPermission("scopenet.vault.*"); }

    private void vault(CorePlayer p, String[] args) {
        Utilities.Vault v = settings.get().vault;
        if (!v.enabled()) { p.send(Format.RED + "Vaults are switched off on this server."); return; }
        if (args.length == 0 && v.count() > 1) {
            p.send(Format.GOLD + "Your vaults " + Format.GRAY + "(" + v.rows() * 9 + " slots each)");
            for (int n = 1; n <= v.count(); n++) {
                p.send(" " + Format.YELLOW + "/vault " + n + (mayOpenVault(p, n, v) ? "" : Format.DARK_GRAY + "  locked"));
            }
            return;
        }
        int n = 1;
        if (args.length > 0) {
            try { n = Integer.parseInt(args[0]); } catch (NumberFormatException e) { p.send(Format.RED + "Usage: /vault <1-" + v.count() + ">"); return; }
        }
        if (n < 1 || n > v.count()) { p.send(Format.RED + "Pick a vault from 1 to " + v.count() + "."); return; }
        if (!mayOpenVault(p, n, v)) { p.send(Format.RED + "Vault " + n + " is locked for your rank."); return; }
        env.platform.openVault(p.uuid(), n, v.rows());
    }

    private void echest(CorePlayer p, String[] args) {
        if (!settings.get().echest) { p.send(Format.RED + "/echest is switched off on this server."); return; }
        env.platform.openEnderChest(p.uuid());
    }

    // ---- /kit --------------------------------------------------------------------------------------------

    private boolean mayUse(CorePlayer p, Utilities.Kit k) {
        if (k.groups().isEmpty() || p.hasPermission("scopenet.kit." + k.id())) return true;
        for (String g : k.groups()) if (p.hasPermission("group." + g)) return true;
        return false;
    }

    private List<String> usableKits(CorePlayer p) {
        List<String> out = new ArrayList<>();
        for (Utilities.Kit k : settings.get().kits) if (mayUse(p, k)) out.add(k.id());
        return out;
    }

    private void kit(CorePlayer p, String[] args) {
        if (args.length > 0 && args[0].equalsIgnoreCase("create")) {
            if (!p.hasPermission("scopenet.admin.kits")) { p.send(Format.RED + "Only administrators can create kits."); return; }
            if (args.length != 2 || !args[1].matches("[a-zA-Z0-9_-]{1,24}")) { p.send(Format.YELLOW + "Usage: /kit create <name> (1-24 letters, numbers, - or _)"); return; }
            var items = env.platform.inventorySnapshot(p.uuid());
            if (items.isEmpty()) { p.send(Format.RED + "Fill your inventory before creating a kit."); return; }
            JsonObject body = new JsonObject();
            body.addProperty("id", args[1].toLowerCase(java.util.Locale.ROOT));
            body.add("items", items);
            env.io(() -> env.panel.call("kits/create", body), r -> p.send(Format.GREEN + "Kit created. You can edit it in the admin panel; servers receive it on their next sync."), e -> p.send(Format.RED + e));
            return;
        }
        Utilities u = settings.get();
        if (args.length == 0) {
            List<Utilities.Kit> mine = new ArrayList<>();
            for (Utilities.Kit k : u.kits) if (mayUse(p, k)) mine.add(k);
            if (mine.isEmpty()) { p.send(Format.GRAY + "There are no kits available to you."); return; }
            p.send(Format.GOLD + "Kits you can claim:");
            long now = env.clock.getAsLong();
            for (Utilities.Kit k : mine) {
                String state;
                if (k.oneTime() && store.last(p.uuid(), "kit:" + k.id()) >= 0) state = Format.DARK_GRAY + "claimed";
                else {
                    long left = k.oneTime() || p.hasPermission("scopenet.cooldown.bypass") ? 0 : store.remainingMs(p.uuid(), "kit:" + k.id(), k.cooldownSecs() * 1000, now);
                    state = left > 0 ? Format.RED + "ready in " + waitText(left) : Format.GREEN + "ready";
                }
                p.send(" " + Format.YELLOW + "/kit " + k.id() + Format.GRAY + " - " + k.name() + " " + state
                        + (k.description().isBlank() ? "" : Format.DARK_GRAY + " · " + k.description()));
            }
            return;
        }
        Utilities.Kit k = u.kit(args[0]);
        if (k == null || !mayUse(p, k)) { p.send(Format.RED + "There's no kit called \"" + args[0] + "\" for you."); return; }
        long now = env.clock.getAsLong();
        if (k.oneTime() && store.last(p.uuid(), "kit:" + k.id()) >= 0) { p.send(Format.RED + "You've already claimed the " + k.name() + " kit."); return; }
        if (!k.oneTime() && !p.hasPermission("scopenet.cooldown.bypass")) {
            long left = store.remainingMs(p.uuid(), "kit:" + k.id(), k.cooldownSecs() * 1000, now);
            if (left > 0) { p.send(Format.RED + "The " + k.name() + " kit is ready again in " + waitText(left) + "."); return; }
        }
        int given = 0, missing = 0;
        for (Utilities.KitItem it : k.items()) {
            ItemSpec spec = it.custom() != null ? u.customItems.get(it.custom()) : it.spec();
            if (spec == null) { missing++; continue; }
            if (env.platform.giveSpec(p.uuid(), spec.withAmount(Math.max(1, it.amount())))) given++; else missing++;
        }
        for (String command : k.commands()) {
            if (!env.platform.console(command.replace("{player}", p.name()))) env.log.warning("kit " + k.id() + ": command not found: " + command);
        }
        store.stamp(p.uuid(), "kit:" + k.id(), now);
        p.send(Format.GREEN + "You claimed the " + k.name() + " kit" + Format.GRAY + " (" + given + " item" + (given == 1 ? "" : "s") + ")" + Format.GREEN + "!");
        if (missing > 0) p.send(Format.YELLOW + missing + " item" + (missing == 1 ? " in this kit isn't" : "s in this kit aren't") + " available on this server.");
    }

    // ---- /quests -----------------------------------------------------------------------------------------

    private static String bar(long progress, long target) {
        int filled = target <= 0 ? 0 : (int) Math.min(10, Math.round(10.0 * progress / target));
        return Format.GREEN + "▮".repeat(filled) + Format.DARK_GRAY + "▮".repeat(10 - filled);
    }

    private void quests(CorePlayer p, String[] args) {
        JsonObject body = new JsonObject();
        if (args.length > 0) body.addProperty("player", args[0]); else body.addProperty("uuid", p.uuid().toString());
        env.io(() -> env.panel.call("quests/player", body), r -> {
            JsonObject o = r.getAsJsonObject();
            boolean self = o.get("uuid").getAsString().equalsIgnoreCase(p.uuid().toString());
            p.send(Format.GOLD + "=== " + (self ? "Your" : o.get("player").getAsString() + "'s") + " quests ===");
            for (String[] section : new String[][] {{"daily", "Daily", "daily_resets"}, {"weekly", "Weekly", "weekly_resets"}}) {
                var list = o.has(section[0]) ? o.getAsJsonArray(section[0]) : new com.google.gson.JsonArray();
                String resets = "";
                try {
                    long secs = java.time.Duration.between(java.time.Instant.now(), java.time.Instant.parse(o.get(section[2]).getAsString())).getSeconds();
                    resets = Format.DARK_GRAY + " (resets in " + waitText(secs * 1000) + ")";
                } catch (RuntimeException ignored) { /* no reset time */ }
                p.send(Format.AQUA + section[1] + " quests" + resets);
                if (list.size() == 0) p.send(Format.GRAY + "  none right now");
                for (var e : list) {
                    JsonObject q = e.getAsJsonObject();
                    long progress = q.get("progress").getAsLong(), target = q.get("target").getAsLong();
                    boolean done = q.get("completed").getAsBoolean();
                    String state = q.get("claimed").getAsBoolean() ? Format.DARK_GRAY + "claimed" : done ? Format.GREEN + "complete" + (self ? " - claim it in the launcher" : "") : Format.GRAY + progress + "/" + target;
                    p.send("  " + bar(progress, target) + " " + Format.YELLOW + q.get("title").getAsString() + Format.GRAY + " " + state + Format.DARK_GRAY + " +" + q.get("xp").getAsLong() + " XP");
                }
            }
        }, e -> p.send(Format.RED + e));
    }

    // ---- /customitem -------------------------------------------------------------------------------------

    private void customItem(CorePlayer p, String[] args) {
        Utilities u = settings.get();
        if (args.length == 0 || args[0].equalsIgnoreCase("list")) {
            if (u.customItems.isEmpty() && u.content.isEmpty()) { p.send(Format.GRAY + "Nothing custom yet. Add some in the admin panel under Content Studio."); return; }
            if (!u.customItems.isEmpty()) p.send(Format.GOLD + "Custom items: " + Format.YELLOW + String.join(Format.GRAY + ", " + Format.YELLOW, u.customItems.keySet().stream().sorted().toList()));
            if (!u.content.isEmpty()) p.send(Format.GOLD + "Placeable (blocks, NPCs, decorations…): " + Format.YELLOW + String.join(Format.GRAY + ", " + Format.YELLOW, u.content.keySet().stream().sorted().toList()));
            p.send(Format.GRAY + "Usage: /customitem give <player> <id> [amount]");
            return;
        }
        if (!args[0].equalsIgnoreCase("give") || args.length < 3) { p.send(Format.YELLOW + "Usage: /customitem give <player> <id> [amount]"); return; }
        CorePlayer target = env.platform.playerByName(args[1]).orElse(null);
        if (target == null) { p.send(Format.RED + "No player called " + args[1] + " is online."); return; }
        ItemSpec spec = u.customItems.get(args[2].toLowerCase());
        if (spec == null && u.content.containsKey(args[2].toLowerCase())) spec = u.content.get(args[2].toLowerCase()).toItemSpec();
        if (spec == null) { p.send(Format.RED + "No custom item called \"" + args[2] + "\". Try /customitem list."); return; }
        int amount = spec.amount();
        if (args.length > 3) {
            try { amount = Math.max(1, Math.min(64, Integer.parseInt(args[3]))); } catch (NumberFormatException e) { p.send(Format.RED + "Amount must be a number."); return; }
        }
        if (!env.platform.giveSpec(target.uuid(), spec.withAmount(amount))) { p.send(Format.RED + "That item can't be created on this server."); return; }
        p.send(Format.GREEN + "Gave " + amount + "x " + args[2].toLowerCase() + " to " + target.name() + ".");
        if (target != p) target.send(Format.GREEN + "You received a special item!");
    }

    // ---- /adminclaim -------------------------------------------------------------------------------------

    private void adminClaim(CorePlayer p, String[] args) {
        if (args.length == 0) {
            p.send(Format.GOLD + "Admin claims " + Format.GRAY + "(protected land that belongs to the server)");
            for (String line : new String[] {
                    "create <name> [description…] [radius] - claim the chunk you stand in (radius adds chunks around you)",
                    "add <name> [radius]    - add your chunk to an existing claim",
                    "remove [name] [radius] - release your chunk",
                    "rename <name> <new name>",
                    "describe <name> <text…>",
                    "color <name> <#rrggbb>",
                    "flag <name> [flag] [on|off] - show or change what is allowed inside (pvp, build, fly, mob_spawning…)",
                    "delete <name>", "list", "info"}) p.send(" " + Format.YELLOW + "/adminclaim " + Format.GRAY + line);
            return;
        }
        Pos pos = p.pos();
        JsonObject body = new JsonObject();
        body.addProperty("dimension", pos.world());
        body.addProperty("chunk_x", Math.floorDiv(pos.blockX(), 16));
        body.addProperty("chunk_z", Math.floorDiv(pos.blockZ(), 16));
        String sub = args[0].toLowerCase();
        switch (sub) {
            case "info" -> {
                ClaimIndex.ClaimInfo info = env.panel.info(pos.world(), Math.floorDiv(pos.blockX(), 16), Math.floorDiv(pos.blockZ(), 16));
                if (info == null) p.send(Format.GRAY + "You're standing in the wilderness.");
                else p.send(Format.GOLD + (info.admin() ? "Admin claim " : "Guild claim ") + Format.YELLOW + info.name()
                        + (info.description().isBlank() ? "" : Format.GRAY + " - " + info.description()));
                return;
            }
            case "list" -> body.addProperty("action", "list");
            case "create" -> {
                if (args.length < 2) { p.send(Format.RED + "Usage: /adminclaim create <name> [description…]"); return; }
                int radius = 0;
                int end = args.length;
                if (end > 2 && args[end - 1].matches("\\d{1,2}")) { radius = Integer.parseInt(args[end - 1]); end--; }
                body.addProperty("action", "create");
                body.addProperty("name", args[1]);
                body.addProperty("value", String.join(" ", java.util.Arrays.copyOfRange(args, 2, end)));
                body.addProperty("radius", radius);
            }
            case "add", "remove" -> {
                body.addProperty("action", sub);
                int radius = 0;
                int end = args.length;
                if (end > 1 && args[end - 1].matches("\\d{1,2}")) { radius = Integer.parseInt(args[end - 1]); end--; }
                if (sub.equals("add") && end < 2) { p.send(Format.RED + "Usage: /adminclaim add <name> [radius]"); return; }
                body.addProperty("name", end > 1 ? args[1] : "");
                body.addProperty("radius", radius);
            }
            case "delete" -> {
                if (args.length < 2) { p.send(Format.RED + "Usage: /adminclaim delete <name>"); return; }
                body.addProperty("action", "delete");
                body.addProperty("name", args[1]);
            }
            case "rename", "describe", "color" -> {
                if (args.length < 3) { p.send(Format.RED + "Usage: /adminclaim " + sub + " <name> <" + (sub.equals("rename") ? "new name" : sub.equals("color") ? "#rrggbb" : "text") + ">"); return; }
                body.addProperty("action", sub);
                body.addProperty("name", args[1]);
                body.addProperty("value", sub.equals("describe") ? String.join(" ", java.util.Arrays.copyOfRange(args, 2, args.length)) : args[2]);
            }
            case "flag", "flags" -> {
                if (args.length < 2) { p.send(Format.RED + "Usage: /adminclaim flag <name> [flag] [on|off]"); return; }
                body.addProperty("action", "flag");
                body.addProperty("name", args[1]);
                body.addProperty("value", args.length >= 4 ? args[2] + " " + args[3] : "");
            }
            default -> { p.send(Format.RED + "Unknown sub-command. Try /adminclaim for help."); return; }
        }
        env.io(() -> env.panel.call("admin-claims", body), result -> {
            env.panel.claimsChanged();
            JsonObject o = result.isJsonObject() ? result.getAsJsonObject() : new JsonObject();
            switch (sub) {
                case "list" -> {
                    var claims = o.has("claims") ? o.getAsJsonArray("claims") : new com.google.gson.JsonArray();
                    if (claims.size() == 0) { p.send(Format.GRAY + "No admin claims yet."); return; }
                    p.send(Format.GOLD + "Admin claims:");
                    claims.forEach(e -> {
                        JsonObject c = e.getAsJsonObject();
                        p.send(" " + Format.YELLOW + c.get("name").getAsString() + Format.GRAY + " - " + c.get("chunks").getAsInt() + " chunks");
                    });
                }
                case "create", "add" -> p.send(Format.GREEN + "Done - " + (o.has("added") ? o.get("added").getAsInt() : 0) + " chunk(s) added"
                        + (o.has("skipped") && o.get("skipped").getAsInt() > 0 ? Format.GRAY + " (" + o.get("skipped").getAsInt() + " already taken)" : "") + ".");
                case "flag", "flags" -> {
                    if (o.has("flags") && o.get("flags").isJsonObject()) {
                        p.send(Format.GOLD + "Flags for " + args[1] + Format.GRAY + " (green = allowed):");
                        o.getAsJsonObject("flags").entrySet().forEach(f -> p.send(" " + (f.getValue().getAsBoolean() ? Format.GREEN : Format.RED) + f.getKey()));
                    } else p.send(Format.GREEN + "Flag updated.");
                }
                case "remove" -> p.send(Format.GREEN + "Released " + (o.has("removed") ? o.get("removed").getAsInt() : 0) + " chunk(s).");
                default -> p.send(Format.GREEN + "Done.");
            }
        }, err -> p.send(Format.RED + err));
    }

    // ---- once a second: banner and flight --------------------------------------------------------------

    /** Call about once a second from the server thread. */
    public void tick() {
        vaultDeliveries.tick();
        Utilities u = settings.get();
        for (CorePlayer p : env.platform.online()) {
            Pos pos = p.pos();
            int cx = Math.floorDiv(pos.blockX(), 16), cz = Math.floorDiv(pos.blockZ(), 16);
            ClaimIndex.ClaimInfo info = env.panel.info(pos.world(), cx, cz);
            String id = info == null ? "" : info.id();
            String before = standingIn.put(p.uuid(), id);
            if (before != null && !before.equals(id)) banner(p, info);
            // An admin claim can ground everyone but staff, whatever else lets them fly.
            if (flying.contains(p.uuid()) && !p.hasPermission("scopenet.claims.bypass") && !env.panel.adminAllows(pos.world(), cx, cz, "fly")) {
                flying.remove(p.uuid());
                env.platform.setFlight(p.uuid(), false);
                p.send(Format.YELLOW + "Flight disabled - flying isn't allowed here.");
            }
            if (flying.contains(p.uuid()) && !p.hasPermission("free.fly") && (!u.fly || !p.hasPermission("guild.fly") || !inOwnGuildLand(p))) {
                flying.remove(p.uuid());
                env.platform.setFlight(p.uuid(), false);
                p.send(Format.YELLOW + "Flight disabled - you left your guild's land.");
            }
        }
    }

    private void banner(CorePlayer p, ClaimIndex.ClaimInfo info) {
        if (info == null) { env.platform.actionbar(p.uuid(), Format.GRAY + "Wilderness"); return; }
        String desc = info.description() == null ? "" : info.description().strip();
        if (desc.length() > 90) desc = desc.substring(0, 89) + "…";
        String text = info.admin()
                ? Format.GREEN + "⚑ " + Format.YELLOW + info.name() + (desc.isEmpty() ? "" : Format.GRAY + " · " + Format.WHITE + desc)
                : Format.GOLD + "⚑ " + Format.YELLOW + "[" + info.tag() + "] " + info.name() + Format.GRAY + " territory";
        env.platform.actionbar(p.uuid(), text);
    }

    /** A player disconnected. */
    public void left(UUID player) {
        standingIn.remove(player);
        flying.remove(player);
    }
}
