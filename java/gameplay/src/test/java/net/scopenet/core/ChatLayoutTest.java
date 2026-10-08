package net.scopenet.core;

import com.google.gson.JsonObject;
import net.scopenet.integration.ChatLayout;
import net.scopenet.integration.ChatLayout.Parts;
import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

class ChatLayoutTest {
    static Parts parts(String tag, String title, String prefix, String suffix) {
        return new Parts(tag, "Iron Fortress", title, null, "Admin", prefix, suffix, 12);
    }

    @Test void luckPermsPrefixAndSuffixKeepTheirColoursAndGetSpacing() {
        String f = ChatLayout.DEFAULT.render(parts("IRON", "Veteran", "&c[Admin]", "&7*"));
        assertEquals("§3[IRON] §6[Veteran] §r§c[Admin] %1$s §r§7*§7: §f%2$s", f);
    }

    @Test void hexColoursInPrefixesBecomeVanillaHex() {
        String f = ChatLayout.DEFAULT.render(parts(null, null, "&#ff8800Elite", null));
        assertTrue(f.contains("§x§f§f§8§8§0§0Elite"), f);
        assertFalse(f.contains("&#"), f);
    }

    @Test void emptyPartsVanishWithoutStrayBracketsOrSpaces() {
        assertEquals("%1$s§7: §f%2$s", ChatLayout.DEFAULT.render(parts(null, "  ", null, "")));
    }

    @Test void percentSignsCannotBreakTheFormat() {
        String f = ChatLayout.DEFAULT.render(parts("100%", "Ti%tle", "50%off", null));
        assertFalse(f.matches("(?s).*[^%]%[^%12].*"), f);
        assertTrue(f.contains("%%"), f);
    }

    @Test void customLayoutsFollowTheAdminsTemplate() {
        JsonObject o = new JsonObject();
        o.addProperty("format", "{level} {prefix}{name} &8» &f{message}");
        o.addProperty("level_format", "&e[{level}]");
        ChatLayout l = ChatLayout.fromJson(o);
        assertEquals("§e[12] §r§aVIP %1$s §8» §f%2$s", l.render(parts("IRON", "Veteran", "&aVIP", "&7*")));
    }

    @Test void messagesOnlyKeepColoursForPlayersWhoMayUseThem() {
        assertEquals("§chello", ChatLayout.DEFAULT.message("&chello", true));
        assertEquals("hello", ChatLayout.DEFAULT.message("&chello", false));
        assertEquals("hello", ChatLayout.DEFAULT.message("&#ff0000hello", false));
        assertEquals("§lbold§r§f and §oit§r§f", ChatLayout.DEFAULT.message("**bold** and *it*", false));
    }

    @Test void brokenConfigFallsBackToDefaults() {
        assertEquals(ChatLayout.DEFAULT.render(parts("A", "B", null, null)), ChatLayout.fromJson(null).render(parts("A", "B", null, null)));
        JsonObject o = new JsonObject();
        o.addProperty("enabled", false);
        assertFalse(ChatLayout.fromJson(o).enabled());
    }

    @Test void rankTitleDrawsThePngGlyphAndFallsBackToTheTextTitle() {
        JsonObject o = new JsonObject();
        o.addProperty("format", "{rank_title}{name}&7: &f{message}");
        o.addProperty("title_format", "&6[{title}] ");
        ChatLayout l = ChatLayout.fromJson(o);
        Parts withPng = new Parts(null, null, "Veteran", "\uF701", null, null, null, 5);
        assertEquals("\uF701§r %1$s§7: §f%2$s", l.render(withPng));
        Parts noPng = new Parts(null, null, "Veteran", null, null, null, null, 5);
        assertEquals("§6[Veteran] %1$s§7: §f%2$s", l.render(noPng));
        // Anything that is not one of our glyphs is ignored, so a title can't smuggle text in.
        assertEquals("§6[Veteran] %1$s§7: §f%2$s", l.render(new Parts(null, null, "Veteran", "hello", null, null, null, 5)));
        assertEquals("%1$s§7: §f%2$s", l.render(new Parts(null, null, null, "", null, null, null, 5)));
    }
}
