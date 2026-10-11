package net.velora.paper.chat;

import net.kyori.adventure.text.Component;
import net.kyori.adventure.text.serializer.legacy.LegacyComponentSerializer;
import net.velora.paper.config.VeloraConfig;
import org.bukkit.entity.Player;

import java.util.regex.Matcher;
import java.util.regex.Pattern;

public final class ChatFormatter {
    private static final Pattern HEX_HASH_PATTERN = Pattern.compile("&#([0-9a-fA-F]{6})");
    private static final Pattern HEX_TAG_PATTERN = Pattern.compile("<#([0-9a-fA-F]{6})>");
    private static final Pattern COLOR_PATTERN = Pattern.compile("[§&][0-9a-fk-orA-FK-OR]");
    private static final Pattern HEX_SECTION_PATTERN = Pattern.compile("§x(§[0-9a-fA-F]){6}");

    private final VeloraConfig config;
    private final LuckPermsHook luckPerms;

    public ChatFormatter(VeloraConfig config, LuckPermsHook luckPerms) {
        this.config = config;
        this.luckPerms = luckPerms;
    }

    public static String colorize(String text) {
        if (text == null || text.isEmpty()) return "";

        // Replace <#RRGGBB> with &#RRGGBB
        Matcher tagMatcher = HEX_TAG_PATTERN.matcher(text);
        text = tagMatcher.replaceAll("&#$1");

        // Replace &#RRGGBB with §x§R§R§G§G§B§B
        Matcher hexMatcher = HEX_HASH_PATTERN.matcher(text);
        StringBuilder sb = new StringBuilder();
        while (hexMatcher.find()) {
            String hex = hexMatcher.group(1);
            StringBuilder replacement = new StringBuilder("§x");
            for (char c : hex.toCharArray()) {
                replacement.append('§').append(Character.toLowerCase(c));
            }
            hexMatcher.appendReplacement(sb, Matcher.quoteReplacement(replacement.toString()));
        }
        hexMatcher.appendTail(sb);

        // Replace standard & color codes with §
        char[] chars = sb.toString().toCharArray();
        for (int i = 0; i < chars.length - 1; i++) {
            if (chars[i] == '&' && "0123456789abcdefklmnorABCDEFKLMNOR".indexOf(chars[i + 1]) >= 0) {
                chars[i] = '§';
            }
        }
        return new String(chars);
    }

    public static String stripColors(String text) {
        if (text == null) return "";
        String s = HEX_HASH_PATTERN.matcher(text).replaceAll("");
        s = HEX_TAG_PATTERN.matcher(s).replaceAll("");
        s = HEX_SECTION_PATTERN.matcher(s).replaceAll("");
        return COLOR_PATTERN.matcher(s).replaceAll("");
    }

    public static String formatRaw(
            String template,
            String prefix,
            String suffix,
            String group,
            String player,
            String displayName,
            String worldName,
            String rawMessage,
            boolean mayColor
    ) {
        String processedMessage = mayColor ? colorize(rawMessage) : stripColors(rawMessage);

        String formatted = template
                .replace("{prefix}", prefix != null ? prefix : "")
                .replace("{suffix}", suffix != null ? suffix : "")
                .replace("{group}", group != null ? group : "")
                .replace("{player}", player != null ? player : "")
                .replace("{name}", player != null ? player : "")
                .replace("{displayname}", displayName != null ? displayName : (player != null ? player : ""))
                .replace("{world}", worldName != null ? worldName : "world")
                .replace("{message}", processedMessage);

        return colorize(formatted);
    }

    public String formatMessage(Player player, String rawMessage) {
        boolean mayColor = config.isAllowColors() && (player.hasPermission("velora.chat.color") || player.isOp());
        String prefix = luckPerms.getPrefix(player);
        String suffix = luckPerms.getSuffix(player);
        String group = luckPerms.getPrimaryGroup(player);
        String worldName = player.getWorld() != null ? player.getWorld().getName() : "world";

        return formatRaw(
                config.getChatFormat(),
                prefix,
                suffix,
                group,
                player.getName(),
                player.getDisplayName(),
                worldName,
                rawMessage,
                mayColor
        );
    }

    public Component formatComponent(Player player, Component rawMessageComponent) {
        String rawText = LegacyComponentSerializer.legacySection().serialize(rawMessageComponent);
        String formattedString = formatMessage(player, rawText);
        return LegacyComponentSerializer.legacySection().deserialize(formattedString);
    }
}
