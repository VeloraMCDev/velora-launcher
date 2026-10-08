package net.scopenet.worldmap;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

class PackagedPaletteTest {
    @Test void original_texture_table_is_packaged_and_used_instead_of_fallback() throws Exception {
        try (var table = BlockColors.class.getResourceAsStream("/net/scopenet/worldmap/block_colors.csv")) {
            assertNotNull(table, "renderer artifact requires the legacy color table");
            assertTrue(new String(table.readAllBytes(), java.nio.charset.StandardCharsets.UTF_8).contains("stone,989898"));
        }
        assertEquals(0xFF989898, BlockColors.of("minecraft:stone"));
        assertNotEquals(MapColors.base(MapColors.STONE), BlockColors.of("minecraft:stone"));
        assertEquals(0, BlockColors.of("minecraft:air"));
        assertEquals(BlockColors.WATER_COLOR, BlockColors.of("minecraft:water"));
    }
}
