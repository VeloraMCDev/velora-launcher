package net.velora.gateway;

import com.google.gson.*;
import java.io.IOException;
import java.net.*;
import java.net.http.*;
import java.time.Duration;
import javax.net.ssl.SSLException;

/** Validates controller responses without exposing response bodies or credentials. */
final class Controller {
    static final class Failure extends Exception { Failure(String message) {super(message);} }
    private final HttpClient http;
    private final String panel, token;
    private final Gson gson = new Gson();
    Controller(HttpClient http, String panel, String token) {
        this.http=http;this.panel=panel.replaceAll("/+$", "");this.token=token;
    }
    private HttpRequest.Builder request(String path) {
        return HttpRequest.newBuilder(URI.create(panel+path)).timeout(Duration.ofSeconds(10))
            .header("Authorization", "Bearer "+token);
    }
    Routes.Snapshot fetch() throws Exception {
        JsonObject body=send(request("/api/gateway/v1/routes").GET().build());
        if (!body.has("revision") || !body.get("revision").isJsonPrimitive()
            || !body.get("revision").getAsJsonPrimitive().isNumber()
            || !body.has("routes") || !body.get("routes").isJsonArray())
            throw new Failure("Panel response is not a route snapshot; check panel-url and reverse-proxy routing");
        Routes.Snapshot snapshot;
        try {snapshot=gson.fromJson(body,Routes.Snapshot.class);Routes.validate(snapshot);}
        catch (RuntimeException e) {throw new Failure("Panel returned an invalid route snapshot");}
        return snapshot;
    }
    void report(Object report) throws Exception {
        JsonObject body=send(request("/api/gateway/v1/status").header("Content-Type","application/json")
            .POST(HttpRequest.BodyPublishers.ofString(gson.toJson(report))).build());
        JsonElement ok=body.get("ok");
        if (ok==null || !ok.isJsonPrimitive() || !ok.getAsJsonPrimitive().isBoolean() || !ok.getAsBoolean())
            throw new Failure("Panel did not acknowledge the status report with ok=true");
    }
    private JsonObject send(HttpRequest request) throws Exception {
        HttpResponse<String> response;
        try {response=http.send(request,HttpResponse.BodyHandlers.ofString());}
        catch (IOException e) {throw new Failure(transportError(e));}
        int code=response.statusCode();
        if (code!=200) {
            String hint=switch(code) {
                case 401,403 -> "; check matching gateway token in the running Panel environment and proxy machine access";
                case 404 -> "; check panel-url and deploy a Panel build with Gateway API support";
                case 429 -> "; controller or proxy rate limit reached";
                default -> code>=300 && code<400 ? " redirect; check panel-url and proxy machine access (redirects are not followed)" : "";
            };
            throw new Failure("HTTP "+code+hint);
        }
        String type=response.headers().firstValue("Content-Type").orElse("").split(";",2)[0].trim();
        if (!type.equalsIgnoreCase("application/json"))
            throw new Failure("HTTP 200 without application/json; check panel-url and proxy login pages");
        if (response.body().length()>256*1024) throw new Failure("Panel JSON response exceeds the size limit");
        try {
            JsonElement body=JsonParser.parseString(response.body());
            if (body.isJsonObject()) return body.getAsJsonObject();
        } catch (RuntimeException ignored) { /* Never log a remote body or parser exception. */ }
        throw new Failure("Panel returned invalid JSON");
    }
    static String transportError(IOException error) {
        for (Throwable e=error;e!=null;e=e.getCause()) {
            if (e instanceof SSLException) return "TLS connection failed; check the Panel certificate and JVM trust store";
            if (e instanceof HttpTimeoutException) return "Panel request timed out";
            if (e instanceof UnknownHostException || e instanceof java.nio.channels.UnresolvedAddressException)
                return "Panel hostname could not be resolved";
        }
        return error instanceof ConnectException ? "Panel connection failed; check network access and listener" : "Panel request failed (I/O error)";
    }
}
