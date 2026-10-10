package net.scopenet.core;

import com.google.gson.JsonObject;
import net.scopenet.integration.ChunkCheckResult;
import net.scopenet.integration.PanelClient;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Path;
import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class CommandsTest {
    @TempDir Path dir;
    Kit kit;
    CommandSet set;
    Kit.Player alex, steve;

    @BeforeEach void setup() {
        kit = new Kit(dir);
        alex = kit.platform.add("Alex");
        steve = kit.platform.add("Steve");
        set = new CommandSet(kit.env, EssentialsConfig.defaults());
    }

    void run(Kit.Player p, String name, String... args) { Kit.find(set.all(), name).run(p, args); }

    // ---- gating -------------------------------------------------------------------------

    @Test void featureSwitchesAndPermissionsGateEveryCommand() {
        alex.denied.add("scopenet.command.home");
        run(alex, "home");
        assertTrue(alex.last().contains("do not have permission"));
        kit.env.features = new Features(false, true, true, true, true, true, "$");
        run(steve, "sethome");
        assertEquals("Essentials are disabled.", steve.last());
        kit.env.features = new Features(true, false, true, true, true, true, "$");
        run(steve, "balance");
        assertEquals("Economy are disabled.", steve.last());
    }

    @Test void everyPaperCommandExists() {
        var names = new java.util.HashSet<String>();
        set.all().forEach(c -> names.add(c.name()));
        for (String n : List.of("spawn", "home", "sethome", "delhome", "back", "tpa", "tpaccept", "tpdeny", "rtp", "warp", "playtime",
                "balance", "pay", "baltop", "shop", "sell", "market", "orders", "contracts", "trade", "transactions", "faction", "claim", "unclaim"))
            assertTrue(names.contains(n), n);
        assertTrue(Kit.find(set.all(), "balance").aliases().contains("bal"));
        assertEquals("scopenet.command.guild", Kit.find(set.all(), "guild").permission());
        assertEquals("faction", Kit.find(set.all(), "guild").name());
        assertSame(Kit.find(set.all(), "faction"), Kit.find(set.all(), "guild"));
    }

    // ---- essentials wiring ---------------------------------------------------------------

    @Test void essentialsCommandsWork() {
        run(alex, "spawn");
        assertEquals(80, alex.pos.y());
        run(alex, "playtime");
        assertTrue(alex.last().contains("1h 1m 40s"));
        run(alex, "playtime", "steve");
        assertTrue(alex.heard("Playtime: Steve"));
        run(alex, "sethome", "a");
        run(alex, "sethome", "b");
        run(alex, "home");
        assertTrue(alex.last().contains("a, b"), alex.last());
        assertEquals(List.of("a", "b"), Kit.find(set.all(), "home").complete(alex, new String[]{""}));
        assertEquals(List.of("Steve"), Kit.find(set.all(), "tpa").complete(alex, new String[]{""}));
    }

    // ---- economy -------------------------------------------------------------------------

    @Test void balancePayAndBaltop() {
        kit.panel.on("economy/balance", "{\"balance\":1234.5}");
        run(alex, "balance");
        assertTrue(alex.heard("Your Balance: $1,234.50"));

        kit.panel.on("economy/transfer", "{\"from_balance\":900.0}");
        run(alex, "pay", "steve", "100");
        assertTrue(alex.heard("Sent $100.00 to Steve. New Balance: $900.00"));
        assertTrue(steve.heard("Received $100.00 from Alex"));
        JsonObject sent = kit.panel.bodies.get(kit.panel.bodies.size() - 1);
        assertEquals(steve.id.toString(), sent.get("to_uuid").getAsString());

        run(alex, "pay", "steve", "-5");
        assertTrue(alex.last().contains("Invalid amount"));
        run(alex, "pay", "alex", "5");
        assertTrue(alex.last().contains("yourself"));
        run(alex, "pay", "ghost", "5");
        assertTrue(alex.last().contains("not online"));
        kit.panel.handlers.put("economy/transfer", b -> { throw new PanelClient.HttpFailure(400, "Insufficient funds"); });
        run(alex, "pay", "steve", "1");
        assertTrue(alex.last().contains("Transfer failed: Insufficient funds"));

        kit.panel.on("economy/baltop", "[{\"rank\":1,\"username\":\"Mia\",\"balance\":9000},{\"rank\":2,\"username\":\"Alex\",\"balance\":5}]");
        run(alex, "baltop");
        assertTrue(alex.heard("#1 Mia: $9,000.00"));
    }

    @Test void sellHandIsEscrowedThenPaid() {
        kit.panel.on("economy/adjust", "{\"balance\":70.0}");
        alex.held = new Item("DIAMOND", "Diamond", 2, "");
        run(alex, "sell", "hand");
        assertNull(alex.held, "taken into escrow");
        assertEquals(1, set.jobs.pending());
        set.tick();
        assertEquals(0, set.jobs.pending());
        JsonObject body = kit.panel.bodies.get(0);
        assertEquals(70.0, body.get("delta").getAsDouble());
        assertTrue(body.has("operation_id"));
        assertTrue(alex.heard("Transaction completed. Balance: $70.00"));
        assertTrue(alex.inventory.isEmpty());

        run(alex, "sell", "hand");
        assertTrue(alex.last().contains("not holding"));
        alex.held = new Item("DIRT", "Dirt", 1, "");
        run(alex, "sell", "hand");
        assertTrue(alex.last().contains("cannot be sold"));
    }

    @Test void rejectedSaleReturnsTheItemsAndOutagesRetryTheSameOperation() {
        alex.held = new Item("DIAMOND", "Diamond", 1, "nbt");
        List<String> ops = new java.util.ArrayList<>();
        kit.panel.handlers.put("economy/adjust", b -> { ops.add(b.get("operation_id").getAsString()); throw new java.io.IOException("connection refused"); });
        run(alex, "sell", "hand");
        set.tick();
        set.tick();
        assertEquals(1, set.jobs.pending(), "outage keeps the job");
        assertEquals(2, ops.size());
        assertEquals(ops.get(0), ops.get(1), "same operation id on retry");

        kit.panel.handlers.put("economy/adjust", b -> { throw new PanelClient.HttpFailure(400, "Sales are paused"); });
        set.tick();
        assertEquals(0, set.jobs.pending());
        assertEquals(1, alex.inventory.size());
        assertEquals("nbt", alex.inventory.get(0).data(), "the very same stack comes back");
        assertTrue(alex.heard("Transaction rejected; held items returned: Sales are paused"));
    }

    @Test void escrowSurvivesARestart() {
        alex.held = new Item("DIAMOND", "Diamond", 1, "");
        run(alex, "sell", "hand");
        CommandSet reopened = new CommandSet(kit.env, EssentialsConfig.defaults());
        assertEquals(1, reopened.jobs.pending());
        kit.panel.on("economy/adjust", "{\"balance\":35.0}");
        reopened.tick();
        assertEquals(0, reopened.jobs.pending());
        assertTrue(alex.heard("Balance: $35.00"));
    }

    @Test void marketListBuyAndSell() {
        kit.panel.on("economy/market", "[{\"id\":7,\"seller_name\":\"Mia\",\"seller_guild\":null,\"item_id\":\"DIAMOND\",\"item_name\":\"Diamond\",\"amount\":3,\"price\":99.5}]");
        run(alex, "market");
        assertTrue(alex.heard("#7 Diamond x3 $99.50 by Mia"));

        kit.panel.on("economy/market/buy", "{\"item_id\":\"DIAMOND\",\"amount\":3,\"new_balance\":10.0}");
        run(alex, "market", "buy", "#7");
        set.tick();
        assertEquals(1, alex.inventory.size());
        assertEquals(3, alex.inventory.get(0).count());
        assertEquals(7, kit.panel.bodies.get(kit.panel.bodies.size() - 1).get("listing_id").getAsLong());

        kit.panel.on("economy/market/list", "{\"id\":8}");
        alex.held = new Item("IRON_INGOT", "Iron Ingot", 5, "x");
        run(alex, "market", "sell", "12.5");
        set.tick();
        JsonObject listed = kit.panel.bodies.stream().filter(b -> b.has("seller_uuid")).findFirst().orElseThrow();
        assertEquals("IRON_INGOT", listed.get("item_id").getAsString());
        assertEquals(5, listed.get("amount").getAsInt());
        assertEquals(12.5, listed.get("price").getAsDouble());

        alex.denied.add("scopenet.command.market.sell");
        alex.held = new Item("IRON_INGOT", "Iron Ingot", 5, "");
        run(alex, "market", "sell", "1");
        assertTrue(alex.last().contains("permission"));
        assertNotNull(alex.held, "nothing taken without permission");
    }

    @Test void auctionsAreListedBidOnAndCollectedFromTheMailbox() {
        String ends = java.time.Instant.now().plusSeconds(2 * 3600 + 60).toString();
        kit.panel.on("economy/market", "[{\"id\":9,\"seller_name\":\"Mia\",\"seller_guild\":null,\"item_id\":\"DIAMOND_SWORD\",\"item_name\":\"Diamond Sword\",\"amount\":1,\"price\":100.0,"
                + "\"kind\":\"auction\",\"ends_at\":\"" + ends + "\",\"current_bid\":120.0,\"bidder_name\":\"Steve\",\"bid_count\":2,\"min_next_bid\":126.0}]");
        run(alex, "market");
        assertTrue(alex.heard("AUCTION"), alex.inbox.toString());
        assertTrue(alex.heard("$120.00 (2 bids)") && alex.heard("ends in 2h") && alex.heard("next bid $126.00"), alex.inbox.toString());

        kit.panel.on("economy/market/bid", "{\"ok\":true,\"message\":\"You lead the auction for Diamond Sword with $126.00.\",\"new_balance\":50.0}");
        run(alex, "market", "bid", "#9");
        set.tick();
        JsonObject bid = kit.panel.bodies.stream().filter(b -> b.has("bidder_uuid")).findFirst().orElseThrow();
        assertEquals(9, bid.get("listing_id").getAsLong());
        assertFalse(bid.has("amount"), "no amount means the minimum bid");
        assertTrue(alex.heard("You lead the auction"));
        run(alex, "market", "bid", "#9", "x");
        assertTrue(alex.last().contains("must be a number"));

        kit.panel.on("economy/market/list", "{\"id\":10}");
        alex.held = new Item("BOW", "Bow", 1, "");
        run(alex, "market", "auction", "25", "48");
        set.tick();
        JsonObject listed = kit.panel.bodies.stream().filter(b -> b.has("seller_uuid")).findFirst().orElseThrow();
        assertEquals("auction", listed.get("kind").getAsString());
        assertEquals(48, listed.get("duration_hours").getAsInt());
        assertEquals(25.0, listed.get("price").getAsDouble());
        alex.held = new Item("BOW", "Bow", 1, "");
        run(alex, "market", "auction", "25", "999");
        assertTrue(alex.last().contains("1 to 168"));
        assertNotNull(alex.held, "nothing is taken for an invalid auction");

        kit.panel.on("economy/market/mailbox/claim", "{\"ok\":true,\"message\":\"Collected 2 item stacks from your mailbox.\",\"items\":["
                + "{\"item_id\":\"DIAMOND_SWORD\",\"item_name\":\"Diamond Sword\",\"amount\":1,\"item_data\":\"snbt\"},{\"item_id\":\"BOW\",\"item_name\":\"Bow\",\"amount\":1,\"item_data\":null}]}");
        alex.inventory.clear();
        run(alex, "market", "claim");
        set.tick();
        assertEquals(2, alex.inventory.size());
        assertEquals("snbt", alex.inventory.get(0).data());
        assertTrue(alex.heard("Collected 2 item stacks"));
    }

    // ---- guilds --------------------------------------------------------------------------

    @Test void guildManagementSubcommandsGoThroughThePanel() {
        kit.panel.on("guilds/manage", "{\"ok\":true,\"guilds\":[{\"name\":\"Iron\",\"tag\":\"IRON\",\"members\":3,\"claims\":5,\"level\":2}]}");
        run(alex, "guild", "list");
        assertTrue(alex.heard("[IRON] Iron - 3 members, 5 chunks, level 2"), alex.inbox.toString());

        kit.panel.on("guilds/manage", "{\"ok\":true,\"message\":\"Steve was removed from Iron.\"}");
        run(alex, "guild", "kick", "Steve");
        JsonObject sent = kit.panel.bodies.get(kit.panel.bodies.size() - 1);
        assertEquals("kick", sent.get("action").getAsString());
        assertEquals("Steve", sent.get("target").getAsString());
        assertEquals(alex.id.toString(), sent.get("uuid").getAsString());
        assertTrue(alex.heard("Steve was removed from Iron."));
        assertTrue(kit.panel.claimsRefreshed);

        run(alex, "guild", "transfer", "Steve");
        assertTrue(alex.last().contains("/faction transfer Steve confirm"), "needs confirmation before anything is sent");
        int calls = kit.panel.calls.size();
        run(alex, "guild", "transfer", "Steve", "confirm");
        assertEquals(calls + 1, kit.panel.calls.size());
        assertEquals("transfer", kit.panel.bodies.get(kit.panel.bodies.size() - 1).get("action").getAsString());

        run(alex, "guild", "post", "Raid", "tonight", "|", "Bring", "potions");
        JsonObject post = kit.panel.bodies.get(kit.panel.bodies.size() - 1);
        assertEquals("Raid tonight", post.get("title").getAsString());
        assertEquals("Bring potions", post.get("text").getAsString());
        run(alex, "guild", "post", "no", "separator");
        assertTrue(alex.last().contains("Usage: /faction post"));

        run(alex, "guild", "info", "Void");
        assertEquals("Void", kit.panel.bodies.get(kit.panel.bodies.size() - 1).get("target").getAsString());
        run(alex, "guild", "role", "Mia", "Scout");
        assertEquals("Scout", kit.panel.bodies.get(kit.panel.bodies.size() - 1).get("text").getAsString());

        kit.panel.handlers.put("guilds/manage", b -> { throw new net.scopenet.integration.PanelClient.HttpFailure(403, "Only the faction leader can change roles"); });
        run(alex, "guild", "promote", "Mia");
        assertTrue(alex.last().contains("Only the faction leader can change roles"));
    }

    @Test void guildFlagsListsAndChangesTheLandRules() {
        kit.panel.on("guilds/manage", "{\"ok\":true,\"can_edit\":true,\"rules\":[{\"id\":\"build\",\"label\":\"Visitors can build\",\"group\":\"Visitors\",\"value\":false,\"editable\":true},"
                + "{\"id\":\"pvp\",\"label\":\"Player vs player\",\"group\":\"Combat\",\"value\":true,\"editable\":false}]}");
        run(alex, "guild", "flags");
        assertEquals("flags", kit.panel.bodies.get(kit.panel.bodies.size() - 1).get("action").getAsString());
        assertTrue(alex.heard("OFF build"), "an off rule is shown as off");
        assertTrue(alex.heard("ON  pvp") && alex.heard("locked by the server"), "a locked rule says so");
        assertTrue(alex.heard("/faction flags <rule> <on|off>"), "someone who can edit is told how");

        kit.panel.on("guilds/manage", "{\"ok\":true,\"message\":\"build is now on for Iron.\"}");
        run(alex, "guild", "flags", "build", "on");
        JsonObject sent = kit.panel.bodies.get(kit.panel.bodies.size() - 1);
        assertEquals("flag", sent.get("action").getAsString());
        assertEquals("build", sent.get("target").getAsString());
        assertEquals("on", sent.get("text").getAsString());
        assertTrue(alex.last().contains("build is now on for Iron."));

        run(alex, "guild", "flags", "build");
        assertTrue(alex.last().contains("Usage: /faction flags"));
        alex.denied.add("scopenet.command.guild.flags");
        run(alex, "guild", "flags");
        assertTrue(alex.last().contains("do not have permission"));
    }

    private void inGuild(String role) {
        kit.panel.on("guilds/player", "{\"in_guild\":true,\"guild\":{\"id\":\"g1\",\"name\":\"Iron\",\"tag\":\"IRON\",\"role\":\"" + role + "\",\"level\":2,\"claims_count\":3,\"max_claims\":16,"
                + "\"members\":[{\"name\":\"Alex\",\"role\":\"leader\"},{\"name\":\"Steve\",\"role\":\"member\"},{\"name\":\"Mia\",\"role\":\"member\"}]}}");
    }

    @Test void guildCreateClaimAndPermissions() {
        kit.panel.on("guilds/create", "{}");
        run(alex, "guild", "create", "Iron", "IRON");
        assertTrue(alex.heard("Faction Iron [IRON] founded"));
        assertTrue(kit.panel.claimsRefreshed);
        run(alex, "guild", "create", "Only");
        assertTrue(alex.last().startsWith("Usage"));

        alex.pos = new Pos("minecraft:overworld", 35, 64, -20, 0, 0); // chunk 2, -2
        kit.panel.on("guilds/claim", "{\"guild_name\":\"Iron\"}");
        run(alex, "claim");
        assertTrue(alex.heard("Chunk claimed successfully for Iron"));
        JsonObject body = kit.panel.bodies.get(kit.panel.bodies.size() - 1);
        assertEquals(2, body.get("chunk_x").getAsInt());
        assertEquals(-2, body.get("chunk_z").getAsInt());
        assertEquals("minecraft:overworld", body.get("dimension").getAsString());

        alex.denied.add("scopenet.command.guild.leave");
        run(alex, "guild", "leave");
        assertTrue(alex.last().contains("permission to use /faction leave"));
        alex.denied.add("scopenet.command.guild.sell.hand");
        run(alex, "guild", "sell", "hand");
        assertTrue(alex.last().contains("/faction sell hand"));
    }

    @Test void guildInfoMembersChatAndMap() {
        inGuild("leader");
        run(alex, "guild");
        assertTrue(alex.heard("Faction: Iron [IRON]"));
        assertTrue(alex.heard("Claimed chunks: 3 / 16"));
        run(alex, "guild", "members");
        assertTrue(alex.heard("Steve (member) [Online]"));
        assertTrue(alex.heard("Mia (member) [Offline]"));

        kit.platform.add("Bob");
        run(alex, "guild", "chat", "hello", "team");
        assertTrue(steve.heard("[Faction IRON] Alex: hello team"));
        Kit.Player bob = kit.platform.online.values().stream().filter(p -> p.name.equals("Bob")).findFirst().orElseThrow();
        assertFalse(bob.heard("hello team"), "non-members don't see faction chat");

        kit.panel.claims.put("minecraft:overworld:1:0", new ChunkCheckResult(true, true, "Iron", "IRON"));
        kit.panel.claims.put("minecraft:overworld:-1:0", new ChunkCheckResult(true, false, "Void", "VOID"));
        run(alex, "guild", "map");
        assertTrue(alex.heard("Territory Map (0, 0)"));
        assertTrue(alex.inbox.stream().anyMatch(s -> s.contains("[P]") && s.contains("x") && s.contains("+")));
    }

    @Test void guildHomeNeedsOfficerAndPersists() {
        inGuild("member");
        run(steve, "guild", "sethome");
        assertTrue(steve.last().contains("Only faction leaders and officers"));
        inGuild("leader");
        alex.pos = new Pos("minecraft:overworld", 100, 64, 100, 0, 0);
        run(alex, "guild", "sethome");
        assertTrue(alex.last().contains("Faction home waypoint set"));
        CommandSet reopened = new CommandSet(kit.env, EssentialsConfig.defaults());
        alex.pos = new Pos("minecraft:overworld", 0, 64, 0, 0, 0);
        Kit.find(reopened.all(), "guild").run(alex, new String[]{"home"});
        assertEquals(100, alex.pos.x());
    }

    @Test void guildBankBalanceDepositAndPay() {
        kit.panel.on("guilds/bank", "{\"guild\":{\"tag\":\"IRON\",\"name\":\"Iron\"},\"role\":\"leader\",\"balance\":150.5,\"my_balance\":749.5,"
                + "\"recent\":[{\"kind\":\"deposit\",\"who\":\"Steve\",\"amount\":250.5,\"note\":\"\",\"at\":\"x\"},{\"kind\":\"withdraw\",\"who\":\"Alex\",\"amount\":100,\"note\":\"\",\"at\":\"x\"}]}");
        run(alex, "guild", "bank");
        assertTrue(alex.heard("[IRON] Iron Bank"));
        assertTrue(alex.heard("Balance: $150.50"));
        assertTrue(alex.heard("-$100.00 withdraw by Alex"));

        kit.panel.on("guilds/bank/transfer", "{\"message\":\"Deposited $50.00 into [IRON] bank.\",\"balance\":200.5}");
        run(alex, "guild", "bank", "deposit", "50");
        set.tick();
        assertTrue(alex.heard("Deposited $50.00 into [IRON] bank. Bank: $200.50"));
        run(alex, "guild", "bank", "withdraw", "0.001");
        assertTrue(alex.last().contains("at least 0.01"));
        alex.denied.add("scopenet.command.guild.bank.withdraw");
        run(alex, "guild", "bank", "withdraw", "5");
        assertTrue(alex.last().contains("permission to withdraw"));

        kit.panel.on("guilds/bank/pay", "{\"message\":\"Paid $10.00 to [VOID].\",\"balance\":190.5}");
        run(alex, "guild", "pay", "VOID", "10");
        set.tick();
        assertTrue(kit.panel.calls.contains("guilds/bank/pay"));
        assertEquals("VOID", kit.panel.bodies.get(kit.panel.bodies.size() - 1).get("to_tag").getAsString());
    }

    @Test void guildSellHandCreditsTheBank() {
        kit.panel.on("guilds/bank/credit", "{\"message\":\"Sold to the shop: $70.00 into [IRON] bank.\",\"balance\":70.0}");
        alex.held = new Item("DIAMOND", "Diamond", 2, "");
        run(alex, "guild", "sell", "hand");
        set.tick();
        JsonObject body = kit.panel.bodies.get(0);
        assertEquals(70.0, body.get("amount").getAsDouble());
        assertFalse(body.has("delta"));
        assertTrue(alex.heard("Bank: $70.00"));
    }
}
