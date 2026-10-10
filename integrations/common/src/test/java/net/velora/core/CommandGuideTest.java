package net.velora.core;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

class CommandGuideTest {
    @org.junit.jupiter.api.io.TempDir java.nio.file.Path directory;
    @Test void everySharedCommandHasHelp() {
        Kit kit = new Kit(directory);
        var commands = new CommandSet(kit.env, EssentialsConfig.defaults()).all();
        var documented = CommandGuide.ENTRIES.stream().map(CommandGuide.Entry::command).collect(java.util.stream.Collectors.toSet());
        assertEquals(java.util.List.of(), commands.stream().map(CoreCommand::name).filter(name -> !documented.contains(name)).toList());
    }
    @Test void onlyEnabledAndPermittedCommandsAreShown() {
        Kit.Player player = new Kit.Player("Alex");
        player.denied.add("velora.command.pay");
        var entries = CommandGuide.visible(player, name -> name.equals("pay") || name.equals("balance"), "");
        assertFalse(entries.isEmpty());
        assertTrue(entries.stream().allMatch(e -> e.command().equals("balance")));
    }
    @Test void commandDetailsCanContinueOnAnotherPage() {
        Kit.Player player = new Kit.Player("Alex");
        CommandGuide.show(player, name -> true, new String[]{"faction"});
        assertTrue(player.heard("Next: /help faction 2"));
        assertTrue(player.heard("/faction create"));
        player.inbox.clear();
        CommandGuide.show(player, name -> true, new String[]{"faction", "2"});
        assertTrue(player.heard("Velora help 2/"));
        assertFalse(player.heard("/faction create"));
    }
    @Test void flightHelpChecksEitherActualGrantNode() {
        Kit.Player player = new Kit.Player("Alex");
        player.denied.add("free.fly"); player.denied.add("faction.fly");
        assertTrue(CommandGuide.visible(player, name -> true, "fly").isEmpty());
        player.granted.add("faction.fly");
        assertFalse(CommandGuide.visible(player, name -> true, "fly").isEmpty());
    }
    @Test void disabledModulesDoNotAppearInHelp() {
        Kit kit = new Kit(directory);
        kit.env.modules = () -> CoreModules.pending();
        var commands = new CommandSet(kit.env, EssentialsConfig.defaults()).all();
        Kit.Player player = new Kit.Player("Alex");
        assertTrue(CommandGuide.visible(player, name -> commands.stream().anyMatch(c -> c.name().equals(name) && c.available()), "faction").isEmpty());
    }
    @Test void invalidPagesAndUnknownQueriesAreActionable() {
        Kit.Player player = new Kit.Player("Alex");
        CommandGuide.show(player, name -> true, new String[]{"0"});
        assertTrue(player.heard("Choose a help page"));
        player.inbox.clear();
        CommandGuide.show(player, name -> true, new String[]{"missing-command"});
        assertTrue(player.heard("No available commands match"));
    }
}
