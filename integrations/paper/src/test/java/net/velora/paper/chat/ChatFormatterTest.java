package net.velora.paper.chat;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

class ChatFormatterTest {

    @Test
    void testColorizeStandard() {
        String input = "&aHello &bWorld&r!";
        String output = ChatFormatter.colorize(input);
        assertEquals("§aHello §bWorld§r!", output);
    }

    @Test
    void testColorizeHex() {
        String input = "&#FF00AAHex <#00FF88>Tag";
        String output = ChatFormatter.colorize(input);
        assertTrue(output.contains("§x§f§f§0§0§a§aHex"));
        assertTrue(output.contains("§x§0§0§f§f§8§8Tag"));
    }

    @Test
    void testStripColors() {
        String input = "&aHello &lWorld&#FF00AA! <#112233>Test";
        String stripped = ChatFormatter.stripColors(input);
        assertEquals("Hello World! Test", stripped);
    }

    @Test
    void testFormatMessageWithPlaceholders() {
        String template = "{prefix}&f{player}{suffix}&7: &f{message}";
        String formatted = ChatFormatter.formatRaw(
                template,
                "&c[Admin] ",
                " &e[VIP]",
                "admin",
                "Steve",
                "Steve",
                "world",
                "&aHello everyone!",
                true
        );

        assertTrue(formatted.contains("§c[Admin]"));
        assertTrue(formatted.contains("§fSteve"));
        assertTrue(formatted.contains("§e[VIP]"));
        assertTrue(formatted.contains("§aHello everyone!"));
    }

    @Test
    void testFormatMessageStripsColorWithoutPermission() {
        String template = "&f{player}&7: &f{message}";
        String formatted = ChatFormatter.formatRaw(
                template,
                "",
                "",
                "default",
                "Alex",
                "Alex",
                "world",
                "&4&lRedBold message! <#123456>Hex",
                false
        );

        assertFalse(formatted.contains("§4"));
        assertFalse(formatted.contains("§l"));
        assertFalse(formatted.contains("§x"));
        assertTrue(formatted.contains("RedBold message! Hex"));
    }

    @Test
    void testFormatMessageWithWorldAndDisplayName() {
        String template = "&8[{world}] &f{displayname}&7: {message}";
        String formatted = ChatFormatter.formatRaw(
                template,
                "",
                "",
                "member",
                "Steve",
                "Steve_The_Builder",
                "nether",
                "hello world",
                false
        );

        assertTrue(formatted.contains("§8[nether]"));
        assertTrue(formatted.contains("Steve_The_Builder"));
        assertTrue(formatted.contains("hello world"));
    }
}
