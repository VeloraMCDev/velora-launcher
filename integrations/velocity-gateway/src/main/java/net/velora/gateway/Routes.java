package net.velora.gateway;

import java.net.*;
import java.util.*;

/** Wire contract is documented in README; no game or Panel implementation dependency. */
public final class Routes {
    public record Route(String id, String experience, String hostname, String address, int port, boolean enabled) {}
    public record Snapshot(long revision, List<Route> routes) {}
    public static String hostname(String input) {
        if (input == null) throw new IllegalArgumentException("Missing hostname");
        String h = input.toLowerCase(Locale.ROOT);
        int colon = h.indexOf(':');
        if (colon >= 0) {
            String port = h.substring(colon + 1);
            if (!port.matches("[0-9]{1,5}") || Integer.parseInt(port) < 1 || Integer.parseInt(port) > 65535) throw new IllegalArgumentException("Invalid hostname port");
            h = h.substring(0, colon);
        }
        if (h.endsWith(".")) h = h.substring(0, h.length() - 1);
        if (h.length() > 253 || !h.contains(".")) throw new IllegalArgumentException("Invalid hostname");
        for (String label : h.split("\\.", -1))
            if (!label.matches("[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?")) throw new IllegalArgumentException("Invalid hostname");
        return h;
    }
    public static Map<String, Route> validate(Snapshot snapshot) {
        if (snapshot == null || snapshot.revision < 0 || snapshot.routes == null || snapshot.routes.size() > 128) throw new IllegalArgumentException("Invalid snapshot");
        Map<String, Route> result = new HashMap<>(); Set<String> ids = new HashSet<>();
        for (Route r : snapshot.routes) {
            if (r == null || r.id == null || !r.id.matches("[a-zA-Z0-9-]{1,64}") || !ids.add(r.id) || r.experience == null || r.experience.isBlank() || r.experience.length() > 120 || !hostname(r.hostname).equals(r.hostname) || r.hostname.contains(":")) throw new IllegalArgumentException("Invalid route");
            if (r.address == null || !r.address.matches("[0-9]+\\.[0-9]+\\.[0-9]+\\.[0-9]+")) throw new IllegalArgumentException("Private IPv4 required");
            String[] parts = r.address.split("\\."); int[] a = Arrays.stream(parts).mapToInt(Integer::parseInt).toArray();
            if (Arrays.stream(a).anyMatch(x -> x > 255) || !(a[0] == 10 || a[0] == 172 && a[1] >= 16 && a[1] <= 31 || a[0] == 192 && a[1] == 168 || a[0] == 100 && a[1] >= 64 && a[1] <= 127) || r.port < 1 || r.port > 65535) throw new IllegalArgumentException("Private tunnel endpoint required");
            if (result.put(r.hostname, r) != null) throw new IllegalArgumentException("Duplicate hostname");
        }
        return Map.copyOf(result);
    }
}
