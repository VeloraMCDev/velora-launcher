package net.scopenet.client;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

class WordsTest {
    @Test void titlesAreCapitalisedEverywhere() {
        assertEquals("Ender Chest", Words.title("ender_chest"));
        assertEquals("Guild Home", Words.title("guild_home"));
        assertEquals("Pending Incoming", Words.title("pending-incoming"));
        assertEquals("", Words.title(null));
        assertEquals("Claim this chunk", Words.sentence("claim this chunk"));
    }

    @Test void worldsAndItemsReadNaturally() {
        assertEquals("Overworld", Words.world("world"));
        assertEquals("The Nether", Words.world("minecraft:the_nether"));
        assertEquals("The End", Words.world("world_the_end"));
        assertEquals("Diamond Pickaxe", Words.item("DIAMOND_PICKAXE"));
        assertEquals("Diamond Pickaxe", Words.item("minecraft:diamond_pickaxe"));
    }

    @Test void durationsAndNumbers() {
        assertEquals("Now", Words.duration(0));
        assertEquals("1m 30s", Words.duration(90));
        assertEquals("2h 1m", Words.duration(7300));
        assertEquals("1d 1h", Words.duration(90000));
        assertEquals("1,250", Words.number(1250));
        assertEquals("1,250.50", Words.number(1250.5));
        assertEquals(1800, Words.secondsUntil("2026-01-01T00:30:00Z", java.time.Instant.parse("2026-01-01T00:00:00Z").toEpochMilli()));
        assertEquals(-1, Words.secondsUntil("nonsense", 0));
        assertEquals("Hello", Words.plain("&6Hel§alo"));
    }
}
