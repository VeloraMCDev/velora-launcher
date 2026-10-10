package net.scopenet.core;

import com.google.gson.JsonParser;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Path;
import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

/** /orders and /contracts: what is sent to the panel, and that items are never lost on the way. */
class BoardTest {
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

    @Test void requestingAnOrderSendsTheItemAmountAndTotal() {
        kit.panel.on("economy/orders/create", "{\"message\":\"Buy order #7 posted.\"}");
        steve.held = new Item("DIAMOND", "Diamond", 1, "");
        run(steve, "orders", "request", "16", "800");
        assertEquals("Buy order #7 posted.", steve.last());
        var body = kit.panel.bodies.get(kit.panel.calls.indexOf("economy/orders/create"));
        assertEquals("DIAMOND", body.get("item_id").getAsString());
        assertEquals(16, body.get("amount").getAsInt());
        assertEquals(800.0, body.get("total").getAsDouble());
        assertTrue(body.has("operation_id"));
        run(steve, "orders", "request", "x", "800");
        assertTrue(steve.last().contains("whole number"));
        run(steve, "orders", "request", "5", "50", "iron_ingot");
        assertEquals("IRON_INGOT", kit.panel.bodies.get(kit.panel.bodies.size() - 1).get("item_id").getAsString());
    }

    @Test void fillingTakesExactlyWhatIsNeededAndQueuesAJob() {
        kit.panel.on("economy/orders", "{\"orders\":[{\"id\":4,\"item_id\":\"COBBLESTONE\",\"item_name\":\"Cobblestone\",\"amount\":100,\"total\":150.0}]}");
        kit.panel.on("economy/orders/fill", "{\"message\":\"Order #4 filled: you earned $150.00.\",\"balance\":1150.0,\"items\":[]}");
        alex.inventory.add(new Item("COBBLESTONE", "Cobblestone", 64, ""));
        alex.inventory.add(new Item("COBBLESTONE", "Cobblestone", 64, ""));
        alex.inventory.add(new Item("COBBLESTONE", "Enchanted", 64, "{tag}"));
        run(alex, "orders", "fill", "4");
        assertEquals(1, set.jobs.pending(), "the items are in escrow until the panel answers");
        assertEquals(1, alex.inventory.size() - 1, "28 cobblestone and the special stack are left");
        assertEquals(28, alex.inventory.stream().filter(i -> i.data().isEmpty()).mapToInt(Item::count).sum());
        set.tick();
        var fill = kit.panel.bodies.get(kit.panel.calls.indexOf("economy/orders/fill"));
        assertEquals(100, fill.get("amount").getAsInt());
        assertEquals("COBBLESTONE", fill.get("item_id").getAsString());
        assertEquals(0, set.jobs.pending());
        assertTrue(alex.heard("Order #4 filled"));
        assertTrue(alex.heard("Balance: $1,150.00"), alex.inbox.toString());
    }

    @Test void requesterChoosesDeadlineAndInvalidMinutesNeverReachPanel() {
        kit.panel.on("economy/orders/create", "{\"message\":\"Posted\"}");
        run(steve,"orders","request","16","80","diamond","45");
        assertEquals(45,kit.panel.bodies.get(kit.panel.bodies.size()-1).get("acceptance_minutes").getAsInt());
        int calls=kit.panel.calls.size();
        for(String deadline:List.of("4","43201","1.5","tomorrow")) run(steve,"orders","request","16","80","diamond",deadline);
        assertEquals(calls,kit.panel.calls.size());
    }

    @Test void notEnoughItemsReturnsEverythingAndSendsNothing() {
        kit.panel.on("economy/orders", "{\"orders\":[{\"id\":4,\"item_id\":\"COBBLESTONE\",\"item_name\":\"Cobblestone\",\"amount\":100,\"total\":150.0}]}");
        alex.inventory.add(new Item("COBBLESTONE", "Cobblestone", 40, ""));
        run(alex, "orders", "fill", "4");
        assertTrue(alex.heard("needs 100x Cobblestone"));
        assertEquals(40, alex.inventory.stream().mapToInt(Item::count).sum());
        assertFalse(kit.panel.calls.contains("economy/orders/fill"));
        assertEquals(0, set.jobs.pending());
        run(alex, "orders", "fill", "99");
        assertTrue(alex.last().contains("isn't on the board"));
    }

    @Test void aRefusedFillGivesTheItemsBack() {
        kit.panel.on("economy/orders", "{\"orders\":[{\"id\":4,\"item_id\":\"COBBLESTONE\",\"item_name\":\"Cobblestone\",\"amount\":10,\"total\":15.0}]}");
        kit.panel.handlers.put("economy/orders/fill", b -> { throw new net.scopenet.integration.PanelClient.HttpFailure(409, "That order is already finished."); });
        alex.inventory.add(new Item("COBBLESTONE", "Cobblestone", 10, ""));
        run(alex, "orders", "fill", "4");
        assertEquals(0, alex.inventory.size());
        set.tick();
        assertEquals(10, alex.inventory.stream().mapToInt(Item::count).sum(), "returned");
        assertTrue(alex.heard("held items returned"));
    }

    @Test void contractsListAndOnlyResourceContractsTakeItems() {
        kit.panel.on("economy/contracts", "{\"enabled\":true,\"contracts\":["
                + "{\"id\":1,\"kind\":\"kill\",\"target\":\"SKELETON\",\"title\":\"Kill 50 Skeletons\",\"required\":50,\"progress\":10,\"reward\":500.0,\"bonus\":false},"
                + "{\"id\":2,\"kind\":\"gather\",\"target\":\"COBBLESTONE\",\"title\":\"Submit 256x Cobblestone\",\"required\":256,\"progress\":56,\"reward\":300.0,\"bonus\":true}],"
                + "\"stats\":{\"done_today\":1,\"daily_limit\":12,\"rerolls_left\":3}}");
        kit.panel.on("economy/contracts/submit", "{\"message\":\"Submit 256x Cobblestone complete: +$300.00\",\"balance\":900.0,\"items\":[]}");
        run(alex, "contracts");
        assertTrue(alex.heard("Kill 50 Skeletons"), alex.inbox.toString());
        assertTrue(alex.heard("10/50"));
        assertTrue(alex.heard("counts automatically"));
        assertTrue(alex.heard("HOT"));
        assertTrue(alex.heard("swaps left: 3"));
        alex.inventory.add(new Item("SKELETON_SKULL", "Skull", 5, ""));
        run(alex, "contracts", "submit");
        assertTrue(alex.last().contains("Nothing in your inventory"));
        alex.inventory.add(new Item("COBBLESTONE", "Cobblestone", 500, ""));
        run(alex, "contracts", "submit");
        set.tick();
        var body = kit.panel.bodies.get(kit.panel.calls.indexOf("economy/contracts/submit"));
        assertEquals(200, body.get("amount").getAsInt(), "only what the contract still needs: 256 - 56");
        assertEquals(2, body.get("contract_id").getAsInt());
        assertEquals(300, alex.inventory.stream().filter(i -> i.id().equals("COBBLESTONE")).mapToInt(Item::count).sum());
        assertTrue(alex.heard("complete: +$300.00"));
    }

    @Test void claimingDroppingAndCancellingNeedAnId() {
        for (String endpoint : List.of("claim", "release", "cancel")) kit.panel.on("economy/orders/" + endpoint, "{\"message\":\"ok " + endpoint + "\"}");
        run(steve, "orders", "pickup");
        assertTrue(steve.last().startsWith("Usage:"));
        run(steve, "orders", "pickup", "#12");
        assertEquals("ok claim", steve.last());
        run(steve, "orders", "drop", "12");
        assertEquals("ok release", steve.last());
        run(steve, "orders", "cancel", "12");
        assertEquals("ok cancel", steve.last());
        assertEquals(12, kit.panel.bodies.get(kit.panel.bodies.size() - 1).get("order_id").getAsInt());
        assertNotNull(JsonParser.parseString("{}"));
    }
}
