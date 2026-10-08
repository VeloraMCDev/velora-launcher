package net.scopenet.integration;

import com.google.gson.*;
import java.io.IOException;
import java.net.URI;
import java.net.http.*;
import java.time.Duration;
import java.util.function.BiFunction;
import java.util.function.Supplier;

/** Neutral API v1 transport. Gameplay settings, caches and rules belong to their owner. */
public final class PanelTransport {
    public static class HttpFailure extends IOException {
        public final int status;
        public HttpFailure(int status, String message) { super(message); this.status = status; }
    }
    private final Supplier<ServerConnection> connection;
    private final BiFunction<Integer, String, ? extends IOException> failure;
    private final HttpClient client = HttpClient.newBuilder()
            .connectTimeout(Duration.ofSeconds(3))
            .followRedirects(HttpClient.Redirect.NEVER).build();

    public PanelTransport(ServerConnection connection) { this(() -> connection, HttpFailure::new); }
    public PanelTransport(Supplier<ServerConnection> connection, BiFunction<Integer, String, ? extends IOException> failure) {
        this.connection = connection;
        this.failure = failure;
    }
    public HttpClient http() { return client; }

    private HttpRequest.Builder request(String endpoint, int timeout) {
        ServerConnection settings = connection.get();
        return HttpRequest.newBuilder(URI.create(settings.panel() + "/api/server/v1/" + endpoint))
                .timeout(Duration.ofSeconds(timeout))
                .header("Authorization", "Bearer " + settings.token());
    }
    private JsonElement decode(HttpResponse<String> response) throws IOException {
        if (response.statusCode() < 200 || response.statusCode() >= 300) {
            String msg = "HTTP " + response.statusCode();
            try {
                JsonObject err = JsonParser.parseString(response.body()).getAsJsonObject();
                if (err.has("message")) msg = err.get("message").getAsString();
                else if (err.has("error")) msg = err.get("error").getAsString();
            } catch (Exception ignored) {}
            throw failure.apply(response.statusCode(), msg);
        }
        try { return JsonParser.parseString(response.body()); }
        catch (RuntimeException e) { throw new IOException("Invalid panel response", e); }
    }
    private JsonObject object(HttpResponse<String> response) throws IOException {
        JsonElement value = decode(response);
        try { return value.getAsJsonObject(); }
        catch (RuntimeException e) { throw new IOException("Invalid panel response", e); }
    }
    public JsonElement postElement(String endpoint, JsonObject payload) throws IOException, InterruptedException {
        HttpRequest request = request(endpoint, 5).header("Content-Type", "application/json")
                .POST(HttpRequest.BodyPublishers.ofString(payload.toString())).build();
        return decode(client.send(request, HttpResponse.BodyHandlers.ofString()));
    }
    public JsonObject post(String endpoint, JsonObject payload) throws IOException, InterruptedException {
        return postElement(endpoint, payload).getAsJsonObject();
    }
    public JsonObject get(String endpoint) throws IOException, InterruptedException {
        return object(client.send(request(endpoint, 15).GET().build(), HttpResponse.BodyHandlers.ofString()));
    }
    public JsonObject postBytes(String endpoint, byte[] body) throws IOException, InterruptedException {
        HttpRequest request = request(endpoint, 60).header("Content-Type", "application/octet-stream")
                .POST(HttpRequest.BodyPublishers.ofByteArray(body)).build();
        return object(client.send(request, HttpResponse.BodyHandlers.ofString()));
    }
}
