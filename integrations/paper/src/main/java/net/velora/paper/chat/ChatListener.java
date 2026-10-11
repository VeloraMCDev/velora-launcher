package net.velora.paper.chat;

import io.papermc.paper.chat.ChatRenderer;
import io.papermc.paper.event.player.AsyncChatEvent;
import net.kyori.adventure.audience.Audience;
import net.kyori.adventure.text.Component;
import net.velora.paper.analytics.AnalyticsTracker;
import net.velora.paper.config.VeloraConfig;
import org.bukkit.entity.Player;
import org.bukkit.event.EventHandler;
import org.bukkit.event.EventPriority;
import org.bukkit.event.Listener;
import org.bukkit.event.player.AsyncPlayerChatEvent;
import org.jetbrains.annotations.NotNull;

public final class ChatListener implements Listener {
    private final VeloraConfig config;
    private final ChatFormatter formatter;
    private final AnalyticsTracker analyticsTracker;

    public ChatListener(VeloraConfig config, ChatFormatter formatter, AnalyticsTracker analyticsTracker) {
        this.config = config;
        this.formatter = formatter;
        this.analyticsTracker = analyticsTracker;
    }

    /**
     * Modern Paper chat event using Adventure components.
     */
    @EventHandler(priority = EventPriority.HIGHEST, ignoreCancelled = true)
    public void onPaperChat(AsyncChatEvent event) {
        if (!config.isChatEnabled()) return;

        Player player = event.getPlayer();
        analyticsTracker.recordChatMessage(player.getUniqueId(), player.getName());

        event.renderer(new ChatRenderer() {
            @Override
            public @NotNull Component render(
                    @NotNull Player source,
                    @NotNull Component sourceDisplayName,
                    @NotNull Component message,
                    @NotNull Audience viewer
            ) {
                return formatter.formatComponent(source, message);
            }
        });
    }

    /**
     * Legacy Bukkit event fallback for compatibility.
     */
    @EventHandler(priority = EventPriority.LOWEST, ignoreCancelled = true)
    public void onLegacyChat(AsyncPlayerChatEvent event) {
        if (!config.isChatEnabled()) return;
        // In modern Paper servers, AsyncChatEvent handles the rendering.
        // We set format here as fallback for any legacy listeners expecting %1$s / %2$s.
        try {
            String format = config.getChatFormat()
                    .replace("{player}", "%1$s")
                    .replace("{name}", "%1$s")
                    .replace("{displayname}", "%1$s")
                    .replace("{message}", "%2$s");
            event.setFormat(ChatFormatter.colorize(format));
        } catch (Exception ignored) {}
    }
}
