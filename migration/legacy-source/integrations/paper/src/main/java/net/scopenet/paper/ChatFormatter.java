package net.scopenet.paper;

import net.scopenet.api.ScopenetPlayer;
import net.scopenet.integration.ChatLayout;
import net.scopenet.paper.compat.ScopenetApiImpl;
import org.bukkit.entity.Player;

import java.util.UUID;
import java.util.function.Function;
import java.util.function.Supplier;

/**
 * Formats player chat from the layout designed in the admin panel. LuckPerms prefixes and suffixes are read live from
 * LuckPerms when it is installed (so a rank change shows on the very next message), and fall back to the copy the panel
 * holds. Nothing here makes a network call on the chat thread.
 */
public final class ChatFormatter {
    /** What LuckPerms says about a player right now. Any field may be null. */
    public record Meta(String prefix, String suffix, String groupDisplay) {}

    /** Set by the LuckPerms module while it is running; absent otherwise. */
    private static volatile Function<UUID, Meta> metaSource;

    public static void metaSource(Function<UUID, Meta> source) { metaSource = source; }

    private final ScopenetApiImpl profiles;
    private final Supplier<ChatLayout> layout;

    public ChatFormatter(ScopenetApiImpl profiles, Supplier<ChatLayout> layout) {
        this.profiles = profiles;
        this.layout = layout;
    }

    public ChatLayout layout() { return layout.get(); }

    /** The Bukkit format string for this player; {@code %1$s} is their display name and {@code %2$s} the message. */
    public String format(Player player) {
        ScopenetPlayer p = profiles.peek(player.getUniqueId()).orElse(null);
        Function<UUID, Meta> source = metaSource;
        Meta live = null;
        if (source != null) {
            try { live = source.apply(player.getUniqueId()); } catch (RuntimeException | LinkageError ignored) { /* fall back to the panel's copy */ }
        }
        String prefix = live != null && live.prefix() != null ? live.prefix() : p == null ? null : p.rankPrefix();
        String suffix = live != null && live.suffix() != null ? live.suffix() : p == null ? null : p.rankSuffix();
        String group = live != null && live.groupDisplay() != null ? live.groupDisplay()
                : p == null ? null : p.rankDisplay() != null && !p.rankDisplay().isBlank() ? p.rankDisplay() : p.rankGroup();
        return layout.get().render(new ChatLayout.Parts(
                p == null || p.guild() == null ? null : p.guild().tag(),
                p == null || p.guild() == null ? null : p.guild().name(),
                p == null ? null : p.rankTitle(),
                p == null ? null : p.rankGlyph(),
                group, prefix, suffix,
                p == null ? 0 : p.globalLevel()));
    }

    public String message(Player player, String raw) {
        return layout.get().message(raw, player.hasPermission("scopenet.chat.color"));
    }
}
