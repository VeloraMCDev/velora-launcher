package net.scopenet.integration;

import com.google.gson.JsonObject;

import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * The chat layout an admin designs in the panel, turned into a Bukkit-style format string.
 *
 * <p>The main template mixes placeholders ({@code {guild} {title} {rank_title} {group} {prefix} {name} {suffix} {level} {message}}) with
 * {@code &} colour codes and {@code &#RRGGBB} colours. Optional parts (guild, title, group, level) have their own small
 * templates and vanish when empty. LuckPerms prefixes and suffixes are inserted as-is, colours included, with the spacing
 * handled here so the admin never has to think about it.</p>
 */
public final class ChatLayout {
    /** What the layout needs to know about one player. Any value may be null or blank. */
    public record Parts(String guildTag, String guildName, String title, String titleGlyph, String group, String prefix, String suffix, int level) {}

    private static final Pattern HEX = Pattern.compile("&#([0-9a-fA-F]{6})");
    private static final Pattern MARKDOWN = Pattern.compile("(\\*\\*(.+?)\\*\\*|__(.+?)__|~~(.+?)~~|`(.+?)`|\\*(.+?)\\*)");
    private static final Pattern PLACEHOLDER = Pattern.compile("\\{([a-z_]+)}");

    private final boolean enabled, markdown, allowColors;
    private final String format, guildFormat, titleFormat, groupFormat, levelFormat;

    public static final ChatLayout DEFAULT = new ChatLayout(true, "{guild}{title}{group}{prefix}{name}{suffix}&7: &f{message}",
            "&3[{tag}] ", "&6[{title}] ", "", "", true, true);

    public ChatLayout(boolean enabled, String format, String guildFormat, String titleFormat, String groupFormat, String levelFormat,
                      boolean markdown, boolean allowColors) {
        this.enabled = enabled;
        this.format = format;
        this.guildFormat = guildFormat;
        this.titleFormat = titleFormat;
        this.groupFormat = groupFormat;
        this.levelFormat = levelFormat;
        this.markdown = markdown;
        this.allowColors = allowColors;
    }

    /** Reads the {@code chat} object of a sync response; missing or broken fields fall back to the defaults. */
    public static ChatLayout fromJson(JsonObject o) {
        if (o == null) return DEFAULT;
        return new ChatLayout(
                bool(o, "enabled", true),
                str(o, "format", DEFAULT.format),
                str(o, "guild_format", DEFAULT.guildFormat),
                str(o, "title_format", DEFAULT.titleFormat),
                str(o, "group_format", DEFAULT.groupFormat),
                str(o, "level_format", DEFAULT.levelFormat),
                bool(o, "markdown", true),
                bool(o, "allow_colors", true));
    }

    private static boolean bool(JsonObject o, String k, boolean d) {
        try { return o.has(k) && !o.get(k).isJsonNull() ? o.get(k).getAsBoolean() : d; } catch (RuntimeException e) { return d; }
    }

    private static String str(JsonObject o, String k, String d) {
        try { return o.has(k) && !o.get(k).isJsonNull() ? o.get(k).getAsString() : d; } catch (RuntimeException e) { return d; }
    }

    public boolean enabled() { return enabled; }
    public boolean allowColors() { return allowColors; }

    /** {@code &} and {@code &#RRGGBB} to section-sign codes. Text that already uses section signs is left alone. */
    public static String colorize(String s) {
        if (s == null || s.isEmpty()) return "";
        Matcher m = HEX.matcher(s);
        StringBuilder out = new StringBuilder();
        while (m.find()) {
            StringBuilder hex = new StringBuilder("§x");
            for (char c : m.group(1).toCharArray()) hex.append('§').append(Character.toLowerCase(c));
            m.appendReplacement(out, Matcher.quoteReplacement(hex.toString()));
        }
        m.appendTail(out);
        char[] b = out.toString().toCharArray();
        for (int i = 0; i < b.length - 1; i++) {
            if (b[i] == '&' && "0123456789abcdefklmnorABCDEFKLMNOR".indexOf(b[i + 1]) >= 0) b[i] = '§';
        }
        return new String(b);
    }

    /** Removes colour codes of both styles, plus the characters that could break out of a template. */
    public static String plain(String s) {
        if (s == null) return "";
        String t = HEX.matcher(s).replaceAll("");
        t = t.replaceAll("[§&][0-9a-fk-orA-FK-OR]", "").replaceAll("§x(§[0-9a-fA-F]){6}", "");
        return t.replace('%', ' ').replace('[', ' ').replace(']', ' ').replace('{', ' ').replace('}', ' ').trim();
    }

    /** True for the single private-use character the panel assigns to a rank image. */
    private static boolean glyph(String s) {
        return s != null && s.length() == 1 && s.charAt(0) >= '\uF700' && s.charAt(0) <= '\uF8FF';
    }

    private static boolean blank(String s) { return s == null || s.isBlank(); }

    /** {@code %} would be read as a format specifier by Bukkit. */
    private static String esc(String s) { return s.replace("%", "%%"); }

    private static String fill(String template, String... kv) {
        String out = template;
        for (int i = 0; i + 1 < kv.length; i += 2) out = out.replace("{" + kv[i] + "}", kv[i + 1]);
        return colorize(out);
    }

    /**
     * The format string for {@code AsyncPlayerChatEvent#setFormat}: {@code %1$s} is the player's display name and
     * {@code %2$s} the message.
     */
    public String render(Parts p) {
        String guild = blank(p.guildTag()) ? "" : fill(guildFormat, "tag", plain(p.guildTag()), "guild_name", plain(p.guildName()));
        String title = blank(p.title()) ? "" : fill(titleFormat, "title", plain(p.title()));
        // The PNG title: a private-use character the resource pack draws as the image. Players without a PNG get the text title.
        String rankTitle = glyph(p.titleGlyph()) ? p.titleGlyph() + "§r " : title;
        String group = blank(p.group()) ? "" : fill(groupFormat, "group", plain(p.group()));
        String level = p.level() <= 0 ? "" : fill(levelFormat, "level", String.valueOf(p.level()));
        // LuckPerms values are the admin's own text: keep their colours, add the spacing around them.
        String prefix = blank(p.prefix()) ? "" : "§r" + colorize(p.prefix().stripTrailing()) + " ";
        String suffix = blank(p.suffix()) ? "" : " §r" + colorize(p.suffix().stripLeading());

        StringBuilder out = new StringBuilder();
        Matcher m = PLACEHOLDER.matcher(format);
        int last = 0;
        while (m.find()) {
            out.append(esc(colorize(format.substring(last, m.start()))));
            last = m.end();
            String v;
            switch (m.group(1)) {
                case "guild" -> v = guild;
                case "title" -> v = title;
                case "rank_title" -> v = rankTitle;
                case "group" -> v = group;
                case "level" -> v = level;
                case "prefix" -> v = prefix;
                case "suffix" -> v = suffix;
                case "name" -> { out.append("%1$s"); continue; }
                case "message" -> { out.append("%2$s"); continue; }
                default -> v = "";
            }
            out.append(esc(v));
        }
        out.append(esc(colorize(format.substring(last))));
        return out.toString();
    }

    /** The message itself: colours only for players allowed them, then the light markdown. */
    public String message(String raw, boolean mayColor) {
        String input = allowColors && mayColor ? colorize(raw) : raw.replaceAll("[§&][0-9a-fk-orA-FK-OR]", "").replaceAll("&#[0-9a-fA-F]{6}", "");
        if (!markdown) return input;
        Matcher matcher = MARKDOWN.matcher(input);
        StringBuilder out = new StringBuilder();
        while (matcher.find()) {
            String code = matcher.group(2) != null ? "§l" : matcher.group(3) != null ? "§n" : matcher.group(4) != null ? "§m" : matcher.group(5) != null ? "§7" : "§o";
            String text = matcher.group(2) != null ? matcher.group(2) : matcher.group(3) != null ? matcher.group(3)
                    : matcher.group(4) != null ? matcher.group(4) : matcher.group(5) != null ? matcher.group(5) : matcher.group(6);
            matcher.appendReplacement(out, Matcher.quoteReplacement(code + text + "§r§f"));
        }
        matcher.appendTail(out);
        return out.toString();
    }
}
