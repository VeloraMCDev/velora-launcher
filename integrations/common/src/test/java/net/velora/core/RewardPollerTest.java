package net.velora.core;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class RewardPollerTest {
    @Test void customRewardsKeepItemStylingAndAmount(@TempDir Path dir) {
        Kit kit = new Kit(dir); var p = kit.platform.add("Alex");
        var poller = new RewardPoller(kit.env, RewardRules.defaults());
        var reward = delivery(1, p.id.toString(), "custom_item", "{\"custom\":\"blade\",\"amount\":128,\"spec\":{\"item\":\"minecraft:diamond_sword\",\"name\":\"&bBlade\",\"enchants\":{\"sharpness\":10},\"custom_model_data\":10000}}");
        assertNull(poller.apply(reward));
        assertEquals(128, kit.platform.given.get(0).amount());
        assertEquals("&bBlade",kit.platform.given.get(0).name());
        assertEquals(10,kit.platform.given.get(0).enchants().get("sharpness"));
        assertEquals(10000,kit.platform.given.get(0).customModelData());
        reward.getAsJsonObject("payload").remove("spec");
        assertNotNull(poller.apply(reward));
    }
    /** A platform that records what was given and run. */
    static final class Recording implements Platform {
        final Kit.FakePlatform inner;
        final List<String> given = new ArrayList<>(), ran = new ArrayList<>();
        boolean knowsItems = true, knowsCommands = true;
        Recording(Kit.FakePlatform inner) { this.inner = inner; }
        public java.util.Optional<CorePlayer> player(java.util.UUID u) { return inner.player(u); }
        public java.util.Optional<CorePlayer> playerByName(String n) { return inner.playerByName(n); }
        public java.util.Collection<? extends CorePlayer> online() { return inner.online(); }
        public java.util.Optional<Pos> safeSurface(String w, int x, int z) { return inner.safeSurface(w, x, z); }
        public Pos worldSpawn(String w) { return inner.worldSpawn(w); }
        public void runMain(Runnable r) { r.run(); }
        public void runAsync(Runnable r) { r.run(); }
        @Override public boolean giveItem(java.util.UUID p, String item, int amount) { if (knowsItems) given.add(item + "x" + amount); return knowsItems; }
        @Override public boolean console(String c) { if (knowsCommands) ran.add(c); return knowsCommands; }
    }

    private static JsonObject delivery(long id, String uuid, String kind, String payload) {
        JsonObject o = new JsonObject();
        o.addProperty("id", id);
        o.addProperty("uuid", uuid);
        o.addProperty("kind", kind);
        o.add("payload", JsonParser.parseString(payload));
        return o;
    }

    @Test void rewardsAreCarriedOutAndReported(@TempDir Path dir) {
        Kit kit = new Kit(dir);
        Kit.Player steve = kit.platform.add("Steve");
        Recording platform = new Recording(kit.platform);
        Env env = new Env(platform, kit.panel, Features.all(), dir, kit.now::get, new java.util.Random(1), java.util.logging.Logger.getLogger("t"));
        JsonObject[] ack = new JsonObject[1];
        kit.panel.handlers.put("rewards/ack", b -> { ack[0] = b; return new JsonObject(); });
        kit.panel.handlers.put("rewards/poll", b -> {
            JsonArray list = new JsonArray();
            String id = steve.id.toString();
            list.add(delivery(1, id, "item", "{\"item\":\"minecraft:diamond\",\"amount\":3}"));
            list.add(delivery(2, id, "permission", "{\"node\":\"essentials.fly\",\"value\":true,\"minutes\":0}"));
            list.add(delivery(3, id, "permission", "{\"node\":\"essentials.fly\",\"value\":true,\"minutes\":10080}"));
            list.add(delivery(4, id, "group", "{\"group\":\"vip\"}"));
            list.add(delivery(5, id, "command", "{\"command\":\"give {player} cake 1\"}"));
            list.add(delivery(6, id, "message", "{\"text\":\"Well done!\"}"));
            JsonObject o = new JsonObject();
            o.add("deliveries", list);
            return o;
        });
        RewardPoller poller = new RewardPoller(env, RewardRules.defaults());
        poller.tick();
        assertEquals(List.of("minecraft:diamondx3"), platform.given);
        assertEquals(List.of("lp user Steve permission set essentials.fly true", "lp user Steve permission settemp essentials.fly true 10080m",
                "lp user Steve parent add vip"), platform.ran, "custom commands are off by default");
        assertEquals(5, ack[0].getAsJsonArray("done").size());
        assertEquals(1, ack[0].getAsJsonArray("failed").size());
        assertTrue(ack[0].getAsJsonArray("failed").get(0).getAsJsonObject().get("error").getAsString().contains("allow-commands"));
        assertTrue(steve.heard("Well done!"));
        assertTrue(steve.heard("3x Diamond"));
    }

    @Test void commandsRunWhenTheOwnerAllowsThem(@TempDir Path dir) {
        Kit kit = new Kit(dir);
        Kit.Player steve = kit.platform.add("Steve");
        Recording platform = new Recording(kit.platform);
        Env env = new Env(platform, kit.panel, Features.all(), dir, kit.now::get, new java.util.Random(1), java.util.logging.Logger.getLogger("t"));
        RewardRules r = RewardRules.defaults();
        RewardPoller poller = new RewardPoller(env, new RewardRules(true, true, r.permissionCommand(), r.permissionDenyCommand(), r.permissionTempCommand(), r.groupCommand()));
        assertNull(poller.apply(delivery(1, steve.id.toString(), "command", "{\"command\":\"give {player} cake 1\"}")));
        assertEquals(List.of("give Steve cake 1"), platform.ran);
        platform.knowsItems = false;
        assertTrue(poller.apply(delivery(2, steve.id.toString(), "item", "{\"item\":\"mod:thing\",\"amount\":1}")).contains("unknown item"));
        kit.platform.online.clear();
        assertEquals("player left", poller.apply(delivery(3, steve.id.toString(), "message", "{\"text\":\"hi\"}")));
    }
}
