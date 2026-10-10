package net.velora.core;
import org.junit.jupiter.api.Test;
import java.util.List;
import static org.junit.jupiter.api.Assertions.*;
class CommandSuggestionsTest {
    @Test void nestedArgumentsAndPermissionFilteringWork() {
        assertEquals(List.of("deposit","withdraw"), CommandSuggestions.choices("guild",new String[]{"bank",""},List.of(),n->true));
        assertEquals(List.of("Alex"),CommandSuggestions.choices("guild",new String[]{"kick",""},List.of("Alex"),n->true));
        assertTrue(CommandSuggestions.choices("guild",new String[]{"flags",""},List.of(),n->true).contains("pvp"));
        assertEquals(List.of("on","off"),CommandSuggestions.choices("guild",new String[]{"flags","pvp",""},List.of(),n->true));
        assertEquals(List.of("help"),CommandSuggestions.choices("velora",new String[]{""},List.of(),n->n.endsWith("help")));
        assertEquals(List.of("help","panel"),CommandSuggestions.choices("velora",new String[]{""},List.of(),n->n.endsWith("help")||n.endsWith("panel")));
        assertFalse(CommandSuggestions.choices("market",new String[]{""},List.of(),n->false).contains("point"));
        assertEquals(List.of("Alex"),CommandSuggestions.filter(List.of("Alex","Steve"),new String[]{"kick","al"}));
    }
}
