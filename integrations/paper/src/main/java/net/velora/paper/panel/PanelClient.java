package net.velora.paper.panel;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import net.velora.paper.config.VeloraConfig;

import java.io.IOException;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.time.Duration;
import java.util.ArrayList;
import java.util.List;
import java.util.UUID;
import java.util.logging.Logger;

public class PanelClient {
    private final VeloraConfig config;
    private final Logger logger;
    private final HttpClient httpClient;

    private volatile HelloResult lastHello = HelloResult.failure("Not initialized");

    public PanelClient(VeloraConfig config, Logger logger) {
        this.config = config;
        this.logger = logger;
        this.httpClient = HttpClient.newBuilder()
                .connectTimeout(Duration.ofSeconds(4))
                .followRedirects(HttpClient.Redirect.NEVER)
                .build();
    }

    public HelloResult getLastHello() {
        return lastHello;
    }

    public boolean isLinked() {
        return lastHello != null && lastHello.ok();
    }

    private HttpRequest.Builder request(String endpoint, int timeoutSeconds) {
        String url = config.getPanelUrl() + "/api/server/v1/" + endpoint;
        HttpRequest.Builder builder = HttpRequest.newBuilder(URI.create(url))
                .timeout(Duration.ofSeconds(timeoutSeconds))
                .header("Content-Type", "application/json");

        String token = config.getToken();
        if (token != null && !token.isBlank()) {
            builder.header("Authorization", "Bearer " + token);
        }
        return builder;
    }

    public HelloResult hello(String software, String mcVersion, String pluginVersion, boolean onlineMode, int maxPlayers) {
        if (!config.hasToken()) {
            this.lastHello = HelloResult.failure("No server token configured. Run /velora link <token> to link with the panel.");
            return this.lastHello;
        }

        JsonObject body = new JsonObject();
        body.addProperty("software", software);
        body.addProperty("mc_version", mcVersion);
        body.addProperty("plugin_version", pluginVersion);
        body.addProperty("online_mode", onlineMode);
        body.addProperty("max_players", maxPlayers);

        try {
            HttpRequest request = request("hello", 8)
                    .POST(HttpRequest.BodyPublishers.ofString(body.toString()))
                    .build();

            HttpResponse<String> response = httpClient.send(request, HttpResponse.BodyHandlers.ofString());
            if (response.statusCode() == 401 || response.statusCode() == 403) {
                this.lastHello = HelloResult.failure("Authentication failed (HTTP " + response.statusCode() + "): Invalid server token");
                return this.lastHello;
            }
            if (response.statusCode() < 200 || response.statusCode() >= 300) {
                String errorMsg = extractErrorMessage(response.body(), "HTTP " + response.statusCode());
                this.lastHello = HelloResult.failure("Panel returned error: " + errorMsg);
                return this.lastHello;
            }

            JsonObject json = JsonParser.parseString(response.body()).getAsJsonObject();
            long serverId = json.has("server_id") ? json.get("server_id").getAsLong() : -1;
            String name = json.has("name") ? json.get("name").getAsString() : "Unknown";
            String brand = json.has("brand") ? json.get("brand").getAsString() : "Velora";
            boolean requireLauncher = json.has("require_launcher") && json.get("require_launcher").getAsBoolean();

            this.lastHello = HelloResult.success(serverId, name, brand, requireLauncher);
            return this.lastHello;
        } catch (InterruptedException e) {
            Thread.currentThread().interrupt();
            this.lastHello = HelloResult.failure("Handshake interrupted: " + e.getMessage());
            return this.lastHello;
        } catch (Exception e) {
            this.lastHello = HelloResult.failure("Failed to connect to panel: " + e.getMessage());
            return this.lastHello;
        }
    }

    public LoginResult login(UUID uuid, String name, String ip) {
        if (!config.hasToken()) {
            return LoginResult.deny(config.getUnavailableMessage());
        }

        JsonObject body = new JsonObject();
        body.addProperty("uuid", uuid.toString());
        body.addProperty("name", name);
        if (ip != null && !ip.isBlank()) {
            body.addProperty("ip", ip);
        }

        try {
            HttpRequest request = request("login", config.getAuthTimeoutSeconds())
                    .POST(HttpRequest.BodyPublishers.ofString(body.toString()))
                    .build();

            HttpResponse<String> response = httpClient.send(request, HttpResponse.BodyHandlers.ofString());
            if (response.statusCode() < 200 || response.statusCode() >= 300) {
                String errorMsg = extractErrorMessage(response.body(), config.getUnavailableMessage());
                logger.warning("Panel auth check returned HTTP " + response.statusCode() + " for " + name + " (" + uuid + "): " + errorMsg);
                return LoginResult.deny(config.getUnavailableMessage());
            }

            JsonObject json = JsonParser.parseString(response.body()).getAsJsonObject();
            boolean allowed = json.has("allowed") && json.get("allowed").getAsBoolean();
            if (!allowed) {
                String denialMsg = json.has("message") && !json.get("message").isJsonNull()
                        ? json.get("message").getAsString()
                        : config.getUnauthorizedMessage();
                return LoginResult.deny(denialMsg);
            }
            return LoginResult.allow();
        } catch (InterruptedException e) {
            Thread.currentThread().interrupt();
            logger.warning("Panel auth check interrupted for " + name + " (" + uuid + ")");
            return LoginResult.deny(config.getUnavailableMessage());
        } catch (Exception e) {
            logger.warning("Panel auth check failed for " + name + " (" + uuid + "): " + e.getMessage());
            return LoginResult.deny(config.getUnavailableMessage());
        }
    }

    public SyncResult sync(JsonObject payload) {
        if (!config.hasToken() || !isLinked()) {
            return SyncResult.failure("Server is not linked to panel");
        }

        try {
            HttpRequest request = request("sync", 15)
                    .POST(HttpRequest.BodyPublishers.ofString(payload.toString()))
                    .build();

            HttpResponse<String> response = httpClient.send(request, HttpResponse.BodyHandlers.ofString());
            if (response.statusCode() < 200 || response.statusCode() >= 300) {
                String errorMsg = extractErrorMessage(response.body(), "HTTP " + response.statusCode());
                return SyncResult.failure(errorMsg);
            }

            List<UUID> kickPlayers = new ArrayList<>();
            try {
                JsonObject json = JsonParser.parseString(response.body()).getAsJsonObject();
                if (json.has("kick") && json.get("kick").isJsonArray()) {
                    JsonArray kicks = json.getAsJsonArray("kick");
                    for (JsonElement elem : kicks) {
                        try {
                            kickPlayers.add(UUID.fromString(elem.getAsString()));
                        } catch (IllegalArgumentException ignored) {}
                    }
                }
            } catch (Exception ignored) {}

            return SyncResult.success(kickPlayers);
        } catch (InterruptedException e) {
            Thread.currentThread().interrupt();
            return SyncResult.failure("Sync interrupted");
        } catch (Exception e) {
            return SyncResult.failure("Sync failed: " + e.getMessage());
        }
    }

    private static String extractErrorMessage(String body, String defaultMessage) {
        if (body == null || body.isBlank()) return defaultMessage;
        try {
            JsonObject json = JsonParser.parseString(body).getAsJsonObject();
            if (json.has("message") && !json.get("message").isJsonNull()) {
                return json.get("message").getAsString();
            }
            if (json.has("error") && !json.get("error").isJsonNull()) {
                return json.get("error").getAsString();
            }
        } catch (Exception ignored) {}
        return defaultMessage;
    }
}
