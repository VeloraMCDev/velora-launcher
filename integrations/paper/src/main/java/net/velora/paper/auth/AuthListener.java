package net.velora.paper.auth;

import net.kyori.adventure.text.Component;
import net.velora.paper.config.VeloraConfig;
import net.velora.paper.panel.LoginResult;
import net.velora.paper.panel.PanelClient;
import org.bukkit.event.EventHandler;
import org.bukkit.event.EventPriority;
import org.bukkit.event.Listener;
import org.bukkit.event.player.AsyncPlayerPreLoginEvent;

import java.util.logging.Logger;

public final class AuthListener implements Listener {
    private final VeloraConfig config;
    private final PanelClient panelClient;
    private final Logger logger;

    public AuthListener(VeloraConfig config, PanelClient panelClient, Logger logger) {
        this.config = config;
        this.panelClient = panelClient;
        this.logger = logger;
    }

    @EventHandler(priority = EventPriority.HIGHEST)
    public void onPreLogin(AsyncPlayerPreLoginEvent event) {
        // If vanilla or prior security already disallowed, do not override
        if (event.getLoginResult() != AsyncPlayerPreLoginEvent.Result.ALLOWED) {
            return;
        }

        if (!config.isAuthEnabled()) {
            return;
        }

        // Strict fail-closed check if server token is unconfigured or linking failed
        if (!config.hasToken() || !panelClient.isLinked()) {
            logger.warning("Rejecting login for " + event.getName() + " (" + event.getUniqueId()
                    + ") - Velora panel is not linked.");
            event.disallow(AsyncPlayerPreLoginEvent.Result.KICK_OTHER, Component.text(config.getUnavailableMessage()));
            return;
        }

        String ip = event.getAddress() != null ? event.getAddress().getHostAddress() : null;
        LoginResult verdict = panelClient.login(event.getUniqueId(), event.getName(), ip);

        if (!verdict.allowed()) {
            String denialMessage = verdict.message() != null && !verdict.message().isBlank()
                    ? verdict.message()
                    : config.getUnauthorizedMessage();
            logger.info("Denied login for " + event.getName() + " (" + event.getUniqueId() + "): " + denialMessage);
            event.disallow(AsyncPlayerPreLoginEvent.Result.KICK_OTHER, Component.text(denialMessage));
        }
    }
}
