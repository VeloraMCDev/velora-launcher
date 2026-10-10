package net.velora.core;

import com.google.gson.*;
import net.scopenet.integration.ChunkCheckResult;

import java.io.IOException;
import java.io.Reader;
import java.io.Writer;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.util.*;
import java.util.concurrent.ConcurrentHashMap;
import java.util.stream.Stream;

/** /faction (and its subcommands), /claim, /unclaim: same behaviour and permission nodes as the Paper plugin. */
final class GuildCommands {
    private static final Gson GSON = new Gson();
    private static final Set<String> GATED = Set.of("create", "leave", "rename", "disband", "claim", "unclaim", "map", "chat", "sethome", "home", "members", "bank", "sell", "market", "pay", "invite", "accept", "decline");

    private final Env env;
    private final Jobs jobs;
    private final EconomyCommands economy;
    private final GuildManage manage;
    private final EssentialsService ess;
    private final Path homesFile;
    private final Map<String, Pos> guildHomes = new ConcurrentHashMap<>();
    /** Faction vaults; set once the utility hub exists. */
    CloudVaults vaults;

    GuildCommands(Env env, Jobs jobs, EconomyCommands economy, EssentialsService ess, Path homesFile) {
        this.manage = new GuildManage(env);
        this.env = env; this.jobs = jobs; this.economy = economy; this.ess = ess; this.homesFile = homesFile;
        if (Files.exists(homesFile)) {
            try (Reader in = Files.newBufferedReader(homesFile)) {
                Map<String, Pos> loaded = GSON.fromJson(in, new com.google.gson.reflect.TypeToken<Map<String, Pos>>() {}.getType());
                if (loaded != null) guildHomes.putAll(loaded);
            } catch (IOException | RuntimeException e) {
                throw new IllegalStateException("Could not read " + homesFile + ": " + e.getMessage(), e);
            }
        }
    }

    List<CoreCommand> build() {
        List<CoreCommand> out = new ArrayList<>();
        out.add(new Cmd("faction", List.of("guild", "g", "clan", "f"), "velora.command.faction", this::guild,
                (p, a) -> CommandSuggestions.choices("faction", a, EssentialsCommands.names(env, p), p::hasPermission)));
        out.add(Cmd.of("claim", List.of(), (p, a) -> claim(p)));
        out.add(Cmd.of("unclaim", List.of(), (p, a) -> unclaim(p)));
        return out;
    }

    private void guild(CorePlayer p, String[] args) {
        if (!env.features.guilds()) { p.send(Format.RED + "Factions are disabled."); return; }
        if (args.length == 0 || (args.length == 1 && (args[0].equalsIgnoreCase("info") || args[0].equalsIgnoreCase("gui")))) {
            if (!p.hasPermission("velora.command.faction.info")) { p.send(Format.RED + "You do not have permission to use /faction info."); return; }
            info(p);
            return;
        }
        String sub = args[0].toLowerCase(Locale.ROOT);
        String node = sub.equals("c") ? "chat" : sub;
        if (GATED.contains(node) && !p.hasPermission("velora.command.faction." + node)) {
            p.send(Format.RED + "You do not have permission to use /faction " + node + ".");
            return;
        }
        String detail = args.length > 1 ? args[1].toLowerCase(Locale.ROOT) : "";
        String fine = sub.equals("sell") && detail.equals("hand") ? "sell.hand"
                : sub.equals("market") && (detail.equals("sell") || detail.equals("buy")) ? "market." + detail : null;
        if (fine != null && !p.hasPermission("velora.command.faction." + fine)) {
            p.send(Format.RED + "You do not have permission to use /faction " + sub + " " + detail + ".");
            return;
        }
        String[] rest = Arrays.copyOfRange(args, 1, args.length);
        switch (sub) {
            case "create" -> create(p, args);
            case "leave" -> leave(p);
            case "rename" -> rename(p, args);
            case "disband" -> disband(p, args);
            case "claim" -> claim(p);
            case "unclaim" -> unclaim(p);
            case "map" -> map(p);
            case "chat", "c" -> chat(p, args);
            case "sethome" -> setHome(p);
            case "home" -> home(p);
            case "members" -> members(p);
            case "bank" -> bank(p, rest);
            case "pay" -> pay(p, rest);
            case "invite" -> invite(p, rest);
            case "accept" -> answer(p, rest, true);
            case "decline", "deny" -> answer(p, rest, false);
            case "sell" -> economy.sell(p, rest, true);
            case "market" -> economy.market(p, rest, true);
            case "upgrade", "upgrades" -> upgrade(p, rest);
            case "outpost" -> outpost(p, rest);
            case "vault", "storage" -> factionVault(p, rest);
            default -> { if (GuildManage.handles(sub, rest.length)) manage.run(p, sub, rest); else help(p); }
        }
    }

    private void help(CorePlayer p) {
        p.send(Format.GOLD + "=== Velora Faction Commands ===");
        String[][] rows = {
            {"/faction", "Your faction overview"}, {"/faction create <name> <tag>", "Create a new faction"}, {"/faction leave", "Leave your current faction"}, {"/faction rename <name> [tag]", "Rename your faction (leader)"}, {"/faction disband", "Disband your faction (leader)"},
            {"/claim", "Claim current chunk (also /faction claim)"}, {"/unclaim", "Unclaim current chunk (also /faction unclaim)"},
            {"/faction map", "Show nearby land claims"}, {"/faction chat <msg>", "Send message to faction members"},
            {"/faction sethome / /faction home", "Faction waypoint base"}, {"/faction bank [deposit|withdraw <amount>]", "Faction money"},
            {"/faction sell [hand]", "Sell items to the shop for the faction bank"}, {"/faction market sell <price> | buy <#id>", "Trade on the market as a faction"},
            {"/faction pay <tag> <amount>", "Pay another faction"},
            {"/faction invite <player>", "Invite a player (leaders and officers)"}, {"/faction accept [tag] | decline [tag]", "Answer a faction invitation"},
            {"/faction list / info <name>", "Browse the factions on this server"}, {"/faction join <name> [message]", "Ask to join a faction"},
            {"/faction requests / approve / reject <player>", "Review join requests (leaders and officers)"},
            {"/faction kick <player>", "Remove a member"}, {"/faction promote / demote <player>", "Make someone an officer, or a member again (leader)"},
            {"/faction roles / role <player> <role>", "See and hand out roles (leader)"}, {"/faction transfer <player> confirm", "Hand over leadership"},
            {"/faction motd <text> / desc <text>", "Set the message of the day or the description"},
            {"/faction flags [<rule> <on|off>]", "See or change what happens on your faction's land"},
            {"/faction post <title> | <text> / posts", "Announcements for your faction"},
            {"/faction upgrade [claims|members|outposts|vault] [tier]", "Buy faction upgrades from the bank (leaders and officers)"},
            {"/faction vault [page]", "Open your faction's shared vault"},
            {"/faction outpost place", "Place the held Outpost Flag; claim its chunk first, then up to 12 nearby chunks"},
        };
        for (String[] r : rows) p.send(Format.YELLOW + r[0] + Format.GRAY + " - " + r[1]);
    }

    private JsonObject uuidBody(CorePlayer p) {
        JsonObject o = new JsonObject();
        o.addProperty("uuid", p.uuid().toString());
        return o;
    }

    private JsonObject chunkBody(CorePlayer p) {
        Pos pos = p.pos();
        JsonObject o = uuidBody(p);
        o.addProperty("dimension", pos.world());
        o.addProperty("chunk_x", pos.blockX() >> 4);
        o.addProperty("chunk_z", pos.blockZ() >> 4);
        return o;
    }

    void claim(CorePlayer p) {
        if (!env.features.guilds()) { p.send(Format.RED + "Factions are disabled."); return; }
        JsonObject body = chunkBody(p);
        p.send(Format.GRAY + "Claiming chunk [" + body.get("chunk_x").getAsInt() + ", " + body.get("chunk_z").getAsInt() + "]...");
        env.io(() -> { JsonObject r = env.panel.call("guilds/claim", body).getAsJsonObject(); env.panel.claimsChanged(); return r; }, r -> {
            String name = r.has("guild_name") ? r.get("guild_name").getAsString() : "your faction";
            p.send(Format.GREEN + "Chunk claimed successfully for " + Format.YELLOW + name + Format.GREEN + "!");
        }, e -> p.send(Format.RED + "Claim failed: " + e));
    }

    void unclaim(CorePlayer p) {
        if (!env.features.guilds()) { p.send(Format.RED + "Factions are disabled."); return; }
        JsonObject body = chunkBody(p);
        int cx = body.get("chunk_x").getAsInt(), cz = body.get("chunk_z").getAsInt();
        p.send(Format.GRAY + "Unclaiming chunk [" + cx + ", " + cz + "]...");
        env.io(() -> { JsonObject r = env.panel.call("guilds/unclaim", body).getAsJsonObject(); env.panel.claimsChanged(); return r; },
                r -> p.send(Format.GREEN + "Chunk [" + cx + ", " + cz + "] has been unclaimed."), e -> p.send(Format.RED + "Unclaim failed: " + e));
    }

    private void create(CorePlayer p, String[] args) {
        if (args.length < 3) { p.send(Format.RED + "Usage: /faction create <name> <tag>"); return; }
        String name = args[1], tag = args[2];
        p.send(Format.GRAY + "Founding faction " + name + " [" + tag + "]...");
        JsonObject body = uuidBody(p);
        body.addProperty("username", p.name());
        body.addProperty("name", name);
        body.addProperty("tag", tag);
        env.io(() -> { JsonObject r = env.panel.call("guilds/create", body).getAsJsonObject(); env.panel.claimsChanged(); return r; },
                r -> p.send(Format.GREEN + "Faction " + Format.YELLOW + name + Format.GOLD + " [" + tag + "]" + Format.GREEN + " founded successfully! You are the Faction Leader."),
                e -> p.send(Format.RED + "Failed to create faction: " + e));
    }

    private void rename(CorePlayer p, String[] args) {
        if (args.length < 2) { p.send(Format.RED + "Usage: /faction rename <name> [tag]"); return; }
        JsonObject body = uuidBody(p);
        body.addProperty("name", args[1]);
        if (args.length > 2) body.addProperty("tag", args[2]);
        p.send(Format.GRAY + "Renaming faction...");
        env.io(() -> { JsonObject r = env.panel.call("guilds/rename", body).getAsJsonObject(); env.panel.claimsChanged(); return r; },
                r -> p.send(Format.GREEN + "Your faction is now " + Format.YELLOW + r.get("name").getAsString() + Format.GOLD + " [" + r.get("tag").getAsString() + "]" + Format.GREEN + "."),
                e -> p.send(Format.RED + "Could not rename the faction: " + e));
    }

    private void disband(CorePlayer p, String[] args) {
        if (args.length < 2 || !args[1].equalsIgnoreCase("confirm")) {
            p.send(Format.RED + "This permanently deletes your faction, frees all its land and pays the treasury to you.");
            p.send(Format.YELLOW + "Type " + Format.GOLD + "/faction disband confirm" + Format.YELLOW + " to go ahead.");
            return;
        }
        env.io(() -> { JsonObject r = env.panel.call("guilds/disband", uuidBody(p)).getAsJsonObject(); env.panel.claimsChanged(); return r; }, r -> {
            double refunded = r.has("refunded") ? r.get("refunded").getAsDouble() : 0;
            p.send(Format.YELLOW + "Your faction was disbanded." + (refunded > 0 ? Format.GREEN + " " + refunded + " from the treasury was paid to you." : ""));
        }, e -> p.send(Format.RED + "Could not disband the faction: " + e));
    }

    private void leave(CorePlayer p) {
        p.send(Format.GRAY + "Leaving faction...");
        env.io(() -> { JsonObject r = env.panel.call("guilds/leave", uuidBody(p)).getAsJsonObject(); env.panel.claimsChanged(); return r; }, r -> {
            boolean disbanded = r.has("disbanded") && r.get("disbanded").getAsBoolean();
            p.send(disbanded ? Format.YELLOW + "You were the last member. The faction was disbanded." : Format.GREEN + "You left your faction.");
        }, e -> p.send(Format.RED + "Error leaving faction: " + e));
    }

    /** The guild the player is in, or an error message sent to them. Runs {@code then} on the server thread. */
    private void withGuild(CorePlayer p, String notInGuild, java.util.function.Consumer<JsonObject> then) {
        env.io(() -> env.panel.call("guilds/player", uuidBody(p)).getAsJsonObject(), r -> {
            if (!r.has("in_guild") || !r.get("in_guild").getAsBoolean()) { p.send(Format.RED + notInGuild); return; }
            then.accept(r.getAsJsonObject("guild"));
        }, e -> p.send(Format.RED + "Error: " + e));
    }

    private void info(CorePlayer p) {
        p.send(Format.GRAY + "Loading faction info...");
        env.io(() -> env.panel.call("guilds/player", uuidBody(p)).getAsJsonObject(), r -> {
            if (!r.has("in_guild") || !r.get("in_guild").getAsBoolean()) {
                p.send(Format.YELLOW + "You are not in a faction! Create one with " + Format.GOLD + "/faction create <name> <tag>");
                return;
            }
            JsonObject g = r.getAsJsonObject("guild");
            int members = g.has("members") ? g.getAsJsonArray("members").size() : 0;
            p.send(Format.GOLD + "=== Faction: " + Format.YELLOW + g.get("name").getAsString() + " " + Format.GOLD + "[" + g.get("tag").getAsString() + "] ===");
            p.send(Format.GRAY + "Your role: " + Format.GREEN + g.get("role").getAsString().toUpperCase(Locale.ROOT) + Format.GRAY + "   Level: " + Format.GOLD
                    + (g.has("level") ? g.get("level").getAsInt() : 1) + Format.GRAY + "   Members: " + Format.WHITE + members);
            p.send(Format.GRAY + "Claimed chunks: " + Format.YELLOW + (g.has("claims_count") ? g.get("claims_count").getAsLong() : 0)
                    + Format.GRAY + " / " + Format.GREEN + (g.has("max_claims") ? g.get("max_claims").getAsLong() : 16));
            p.send(Format.GRAY + "/claim, /unclaim, /faction map, /faction members, /faction bank");
        }, e -> p.send(Format.RED + "Failed to load faction: " + e));
    }

    private void map(CorePlayer p) {
        Pos pos = p.pos();
        int cx = pos.blockX() >> 4, cz = pos.blockZ() >> 4;
        p.send(Format.GOLD + "========= " + Format.YELLOW + "Territory Map (" + cx + ", " + cz + ")" + Format.GOLD + " =========");
        p.send(Format.GRAY + "   N (-Z)");
        for (int dz = -3; dz <= 3; dz++) {
            StringBuilder row = new StringBuilder(" ");
            for (int dx = -4; dx <= 4; dx++) {
                if (dx == 0 && dz == 0) { row.append(Format.GOLD).append("[P]"); continue; }
                ChunkCheckResult res = env.panel.check(pos.world(), cx + dx, cz + dz, p.uuid());
                if (res.claimed() && res.allowed()) row.append(Format.GREEN).append(" + ");
                else if (res.claimed()) row.append(Format.RED).append(" x ");
                else row.append(Format.DARK_GRAY).append(" . ");
            }
            p.send(row.toString());
        }
        p.send(Format.GRAY + "   S (+Z)");
        p.send(Format.GOLD + "[P]" + Format.YELLOW + " You  " + Format.GREEN + "+ " + Format.GRAY + "Your Faction  " + Format.RED + "x " + Format.GRAY + "Other Faction  "
                + Format.DARK_GRAY + ". " + Format.GRAY + "Wilderness");
    }

    private void chat(CorePlayer p, String[] args) {
        if (args.length < 2) { p.send(Format.RED + "Usage: /faction chat <message>"); return; }
        String message = String.join(" ", Arrays.copyOfRange(args, 1, args.length));
        withGuild(p, "You are not in a faction.", g -> {
            String tag = g.get("tag").getAsString();
            Set<String> names = new HashSet<>();
            if (g.has("members")) g.getAsJsonArray("members").forEach(m -> names.add(m.getAsJsonObject().get("name").getAsString().toLowerCase(Locale.ROOT)));
            String line = Format.GREEN + "[Faction " + tag + "] " + Format.YELLOW + p.name() + ": " + Format.WHITE + message;
            // Only guild members read guild chat.
            for (CorePlayer other : env.platform.online()) if (names.contains(other.name().toLowerCase(Locale.ROOT)) || other.uuid().equals(p.uuid())) other.send(line);
        });
    }

    private void save() {
        try {
            if (homesFile.getParent() != null) Files.createDirectories(homesFile.getParent());
            Path tmp = homesFile.resolveSibling(homesFile.getFileName() + ".tmp");
            try (Writer w = Files.newBufferedWriter(tmp)) { GSON.toJson(new TreeMap<>(guildHomes), w); }
            Files.move(tmp, homesFile, StandardCopyOption.REPLACE_EXISTING);
        } catch (IOException e) {
            env.log.warning("Failed to save faction homes: " + e.getMessage());
        }
    }

    private void setHome(CorePlayer p) {
        withGuild(p, "You must be in a faction to set a faction home.", g -> {
            String role = g.get("role").getAsString();
            if (!role.equals("leader") && !role.equals("officer")) { p.send(Format.RED + "Only faction leaders and officers can set the faction home."); return; }
            guildHomes.put(g.get("id").getAsString(), p.pos());
            save();
            p.send(Format.GREEN + "Faction home waypoint set at your location!");
        });
    }

    private void home(CorePlayer p) {
        withGuild(p, "You are not in a faction.", g -> {
            Pos at = guildHomes.get(g.get("id").getAsString());
            if (at == null) { p.send(Format.RED + "Your faction has not set a faction home yet! Use /faction sethome"); return; }
            ess.teleportTo(p, at);
            p.send(Format.GREEN + "Teleported to faction home base.");
        });
    }

    private void members(CorePlayer p) {
        withGuild(p, "You are not in a faction.", g -> {
            p.send(Format.GOLD + "=== Members of " + Format.YELLOW + g.get("name").getAsString() + Format.GOLD + " ===");
            for (JsonElement el : g.getAsJsonArray("members")) {
                JsonObject m = el.getAsJsonObject();
                String name = m.get("name").getAsString();
                String status = env.platform.playerByName(name).isPresent() ? Format.GREEN + "[Online]" : Format.GRAY + "[Offline]";
                p.send(Format.WHITE + " - " + name + " (" + Format.AQUA + m.get("role").getAsString() + Format.WHITE + ") " + status);
            }
        });
    }

    // ---- invitations ------------------------------------------------------------

    private void invite(CorePlayer p, String[] args) {
        if (args.length < 1) { p.send(Format.RED + "Usage: /faction invite <player>"); return; }
        JsonObject body = uuidBody(p);
        body.addProperty("target", args[0]);
        env.io(() -> env.panel.call("guilds/invite/send", body).getAsJsonObject(),
                r -> p.send(Format.GREEN + "Invitation sent to " + Format.YELLOW + (r.has("player") ? r.get("player").getAsString() : args[0]) + Format.GREEN + "."),
                e -> p.send(Format.RED + "Could not invite: " + e));
    }

    private void answer(CorePlayer p, String[] args, boolean accept) {
        JsonObject body = uuidBody(p);
        body.addProperty("tag", args.length > 0 ? args[0] : "");
        body.addProperty("accept", accept);
        env.io(() -> { JsonObject r = env.panel.call("guilds/invite/respond", body).getAsJsonObject(); if (accept) env.panel.claimsChanged(); return r; },
                r -> p.send(accept ? Format.GREEN + "Welcome to " + Format.YELLOW + "[" + r.get("tag").getAsString() + "] " + r.get("guild").getAsString() + Format.GREEN + "!"
                        : Format.YELLOW + "Invitation declined."),
                e -> p.send(Format.RED + e));
    }

    /** Guild homes by guild id, for the map. */
    Map<String, Pos> guildHomes() { return Map.copyOf(guildHomes); }

    // ---- bank ---------------------------------------------------------------

    /** Shared faction storage. Pages are bought with /guild upgrade vault; roles can be denied access in the launcher. */
    private void factionVault(CorePlayer p, String[] args) {
        if (vaults == null || !vaults.supported() || !env.modules.get().enabled("vaults")) { p.send(Format.RED + "Faction vaults are not available on this server."); return; }
        int page = 1;
        if (args.length > 0) {
            try { page = Integer.parseInt(args[0]); } catch (NumberFormatException e) { p.send(Format.RED + "Usage: /faction vault [page]"); return; }
        }
        if (page < 1 || page > 9) { p.send(Format.RED + "Faction vault pages are numbered 1 to 9."); return; }
        vaults.open(p, "faction", page, "Faction vault " + page);
    }

    private void outpost(CorePlayer p, String[] args) {
        if (!env.modules.get().enabled("factions") || !env.modules.get().enabled("economy")) { p.send(Format.RED + "Outposts are disabled."); return; }
        if (args.length != 1 || !args[0].equalsIgnoreCase("place")) { p.send(Format.YELLOW + "Buy a flag with /faction upgrade outposts, collect it from /vault, then /faction outpost place."); return; }
        Item held = p.heldItem().orElse(null);
        if (held == null || held.count() != 1 || !Item.registryId(held.id()).equals("minecraft:white_banner")) { p.send(Format.RED + "Hold a single purchased Outpost Flag."); return; }
        var match = java.util.regex.Pattern.compile("VeloraOutpostId\\s*:\\s*\"([0-9a-fA-F-]{36})\"").matcher(held.data());
        if (!match.find()) { p.send(Format.RED + "This banner is not a purchased Outpost Flag."); return; }
        Pos at = p.pos();
        if (!env.platform.canPlaceOutpostFlag(at)) { p.send(Format.RED + "Stand in an empty block above solid ground to place the flag."); return; }
        JsonObject body = uuidBody(p);
        body.addProperty("id", match.group(1)); body.addProperty("dimension", at.world());
        body.addProperty("x", at.blockX()); body.addProperty("y", at.blockY()); body.addProperty("z", at.blockZ());
        env.io(() -> env.panel.call("guilds/outposts/place", body), r -> {
            if (env.platform.player(p.uuid()).isEmpty() || !p.heldItem().filter(held::equals).isPresent() || !env.platform.placeOutpostFlag(at)) {
                p.send(Format.YELLOW + "The flag location is registered. Keep the flag and retry at the same location when it is clear."); return;
            }
            p.clearHeld(); env.platform.savePlayer(p.uuid());
            p.send(Format.GREEN + "Outpost placed. Claim this chunk, then adjacent chunks within 3 chunks of the flag. Maximum: 12 claims.");
        }, e -> p.send(Format.RED + "Outpost: " + e));
    }

    /** Faction upgrades (Velora SMP). Buying names the tier just shown, so repeating the command cannot buy twice. */
    private void upgrade(CorePlayer p, String[] args) {
        JsonObject body = uuidBody(p);
        String track = args.length > 0 ? args[0].toLowerCase(Locale.ROOT) : null;
        if (track != null && !List.of("claims", "members", "outposts", "vault").contains(track)) { p.send(Format.RED + "Usage: /faction upgrade [claims|members|outposts|vault] [tier]"); return; }
        if (track != null && args.length > 1) {
            int tier;
            try { tier = Integer.parseInt(args[1]); } catch (NumberFormatException e) { p.send(Format.RED + "The tier must be a number."); return; }
            body.addProperty("track", track);
            body.addProperty("expected_tiers", tier - 1);
        }
        env.io(() -> env.panel.call("guilds/upgrades", body).getAsJsonObject(), r -> {
            if (r.has("ok")) { p.send(Format.GREEN + "Bought " + track + " tier " + r.get("tiers").getAsLong() + "."); r = r.getAsJsonObject("upgrades"); }
            JsonObject tracks = r.getAsJsonObject("tracks");
            p.send(Format.GOLD + "=== Faction upgrades " + Format.GRAY + "(bank " + env.money(r.get("bank_cents").getAsLong() / 100.0) + ")");
            for (String name : List.of("claims", "members", "outposts", "vault")) {
                if (track != null && !track.equals(name)) continue;
                JsonObject t = tracks.getAsJsonObject(name);
                long have = t.get("tiers").getAsLong(), max = t.get("max").getAsLong();
                String next = have >= max ? Format.DARK_GRAY + "maxed" : Format.YELLOW + "/faction upgrade " + name + " " + (have + 1) + Format.GRAY + " for " + env.money(t.get("price_cents").getAsLong() / 100.0);
                p.send(Format.WHITE + " " + name + Format.GRAY + " " + have + "/" + max + "  " + next);
            }
            p.send(Format.GRAY + "Outposts: " + r.get("outposts").getAsLong() + "/" + r.get("outpost_limit").getAsLong()
                    + "  ·  +" + r.get("claims_per_tier").getAsLong() + " chunks or +" + r.get("members_per_tier").getAsLong() + " members per tier");
        }, e -> p.send(Format.RED + "Faction upgrades: " + e));
    }

    private void bank(CorePlayer p, String[] args) {
        if (args.length >= 2 && (args[0].equalsIgnoreCase("deposit") || args[0].equalsIgnoreCase("withdraw"))) {
            String action = args[0].toLowerCase(Locale.ROOT);
            if (!p.hasPermission("velora.command.faction.bank." + action)) { p.send(Format.RED + "You do not have permission to " + action + " faction money."); return; }
            Double value = Format.amount(args[1]);
            if (value == null) { p.send(Format.RED + "Enter an amount of at least 0.01, for example 25 or 12.50."); return; }
            JsonObject payload = uuidBody(p);
            payload.addProperty("username", p.name());
            payload.addProperty("amount", value);
            payload.addProperty("action", action);
            jobs.enqueue(p, "guilds/bank/transfer", payload, List.of(), null);
            return;
        }
        if (args.length > 0 && !args[0].equalsIgnoreCase("balance") && !args[0].equalsIgnoreCase("info")) {
            p.send(Format.RED + "Usage: /faction bank [deposit|withdraw <amount>]");
            return;
        }
        env.io(() -> env.panel.call("guilds/bank", uuidBody(p)).getAsJsonObject(), info -> {
            if (!info.has("guild") || info.get("guild").isJsonNull()) { p.send(Format.RED + "You are not in a faction on this server."); return; }
            JsonObject g = info.getAsJsonObject("guild");
            p.send(Format.GOLD + "=== [" + g.get("tag").getAsString() + "] " + g.get("name").getAsString() + " Bank ===");
            p.send(Format.YELLOW + "Balance: " + Format.GREEN + env.money(info.get("balance").getAsDouble()) + Format.GRAY + "   Your wallet: "
                    + env.money(info.get("my_balance").getAsDouble()) + "   Role: " + info.get("role").getAsString());
            for (JsonElement el : info.getAsJsonArray("recent")) {
                JsonObject row = el.getAsJsonObject();
                String kind = row.get("kind").getAsString();
                boolean out = kind.equals("withdraw") || kind.equals("purchase") || kind.equals("transfer_out");
                p.send(Format.GRAY + " " + (out ? Format.RED + "-" : Format.GREEN + "+") + env.money(row.get("amount").getAsDouble()) + Format.GRAY + " "
                        + kind.replace('_', ' ') + " by " + row.get("who").getAsString());
            }
            p.send(Format.GRAY + "/faction bank deposit|withdraw <amount>, /faction sell, /faction market, /faction pay <tag> <amount>");
        }, e -> p.send(Format.RED + "Could not load the faction bank: " + e));
    }

    private void pay(CorePlayer p, String[] args) {
        if (args.length < 2) { p.send(Format.RED + "Usage: /faction pay <faction tag> <amount>"); return; }
        Double value = Format.amount(args[1]);
        if (value == null) { p.send(Format.RED + "Enter an amount of at least 0.01, for example 25 or 12.50."); return; }
        JsonObject payload = uuidBody(p);
        payload.addProperty("to_tag", args[0]);
        payload.addProperty("amount", value);
        jobs.enqueue(p, "guilds/bank/pay", payload, List.of(), null);
    }
}
