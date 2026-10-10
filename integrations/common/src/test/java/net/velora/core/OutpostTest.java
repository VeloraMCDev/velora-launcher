package net.velora.core;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;
import java.nio.file.Path;
import java.util.UUID;
import static org.junit.jupiter.api.Assertions.*;

class OutpostTest {
    @TempDir Path dir;
    private Item flag(String id) { return new Item("minecraft:white_banner", "Outpost Flag", 1,
        "{id:\"minecraft:white_banner\",Count:1b,tag:{VeloraOutpostId:\""+id+"\"}}"); }
    private void place(CommandSet set, Kit.Player p) { Kit.find(set.all(), "guild").run(p,new String[]{"outpost","place"}); }

    @Test void placementConsumesOnlyTheConfirmedFlag() {
        Kit k=new Kit(dir); Kit.Player p=k.platform.add("SyntheticLeader");
        CommandSet set=new CommandSet(k.env,EssentialsConfig.defaults());
        String id=UUID.randomUUID().toString(); p.held=flag(id); k.platform.outpostPlaceable=true;
        k.panel.on("guilds/outposts/place","{\"ok\":true}");
        place(set,p);
        assertNull(p.held);
        assertEquals(1,k.platform.outpostFlags.size());
        assertEquals(id,k.panel.bodies.get(0).get("id").getAsString());
        assertTrue(p.heard("Maximum: 12"));
    }

    @Test void rejectionKeepsTheFlagAndNormalBannersCannotRegister() {
        Kit k=new Kit(dir); Kit.Player p=k.platform.add("SyntheticLeader");
        CommandSet set=new CommandSet(k.env,EssentialsConfig.defaults()); k.platform.outpostPlaceable=true;
        p.held=new Item("minecraft:white_banner","Banner",1,"");
        place(set,p); assertTrue(k.panel.calls.isEmpty()); assertNotNull(p.held);
        Item item=flag(UUID.randomUUID().toString()); p.held=item;
        k.panel.handlers.put("guilds/outposts/place",body->{throw new java.io.IOException("bank or role rejected");});
        place(set,p); assertEquals(item,p.held); assertTrue(k.platform.outpostFlags.isEmpty());
    }

    @Test void changedHeldItemAfterResponseIsNeverDeleted() {
        Kit k=new Kit(dir); Kit.Player p=k.platform.add("SyntheticLeader");
        CommandSet set=new CommandSet(k.env,EssentialsConfig.defaults()); k.platform.outpostPlaceable=true;
        p.held=flag(UUID.randomUUID().toString()); Item other=new Item("minecraft:diamond","Diamond",1,"");
        k.panel.handlers.put("guilds/outposts/place",body->{p.held=other;return com.google.gson.JsonParser.parseString("{\"ok\":true}");});
        place(set,p); assertEquals(other,p.held); assertTrue(k.platform.outpostFlags.isEmpty());
        assertTrue(p.heard("retry at the same location"));
    }
}
