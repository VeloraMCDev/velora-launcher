package net.scopenet.integration;

import java.net.URI;

/** Velora server API connection. Legacy token and URL semantics stay compatible. */
public record ServerConnection(URI panel, String token) {
    public ServerConnection {
        if (panel == null || panel.getHost() == null || panel.getRawUserInfo() != null
                || panel.getRawQuery() != null || panel.getRawFragment() != null
                || !("https".equals(panel.getScheme()) || "http".equals(panel.getScheme()))) {
            throw new IllegalArgumentException("panel-url must be an HTTP(S) URL without credentials, query or fragment");
        }
        token = token == null ? "" : token.trim();
        if (!token.matches("sn_[A-Za-z0-9]{40}")) {
            throw new IllegalArgumentException("Set the server token from the panel's Servers page");
        }
    }

    public static ServerConnection of(String panel, String token) {
        return new ServerConnection(URI.create(panel.trim().replaceAll("/+$", "")), token);
    }

    @Override public String toString() { return "ServerConnection[panel=" + panel + ", token=<redacted>]"; }
}
