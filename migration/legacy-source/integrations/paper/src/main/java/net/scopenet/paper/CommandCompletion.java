package net.scopenet.paper;

import net.scopenet.core.CommandSuggestions;
import org.bukkit.Bukkit;
import org.bukkit.command.*;
import org.bukkit.entity.Player;
import java.util.*;

/** Common completions for Paper's economy, guild and administrative command handlers. */
final class CommandCompletion implements TabCompleter {
    @Override public List<String> onTabComplete(CommandSender sender, Command command, String alias, String[] args) {
        if (!command.testPermissionSilent(sender)) return List.of();
        var players = Bukkit.getOnlinePlayers().stream().filter(p -> !(sender instanceof Player who) || who.canSee(p)).map(Player::getName).toList();
        return CommandSuggestions.filter(CommandSuggestions.choices(command.getName(), args, players, sender::hasPermission), args);
    }
}
