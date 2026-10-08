package net.scopenet.core;

/**
 * How this server carries out rewards the panel hands over. Permissions and groups go through console commands so any
 * permissions plugin works; the defaults are LuckPerms'. Custom admin commands stay off unless the server owner allows them.
 */
public record RewardRules(boolean enabled, boolean allowCommands, String permissionCommand, String permissionDenyCommand,
                          String permissionTempCommand, String groupCommand) {
    public static RewardRules defaults() {
        return new RewardRules(true, false, "lp user {player} permission set {node} true", "lp user {player} permission set {node} false",
                "lp user {player} permission settemp {node} {value} {duration}", "lp user {player} parent add {group}");
    }
}
