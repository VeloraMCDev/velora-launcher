package net.velora.gateway;

import com.google.inject.Inject;
import com.google.gson.Gson;
import com.velocitypowered.api.plugin.Plugin;
import com.velocitypowered.api.plugin.annotation.DataDirectory;
import com.velocitypowered.api.proxy.*;
import com.velocitypowered.api.proxy.server.*;
import com.velocitypowered.api.event.Subscribe;
import com.velocitypowered.api.event.PostOrder;
import com.velocitypowered.api.event.proxy.*;
import com.velocitypowered.api.event.connection.*;
import com.velocitypowered.api.event.player.*;
import net.kyori.adventure.text.Component;
import org.slf4j.Logger;
import java.net.*;
import java.net.http.*;
import java.nio.file.*;
import java.time.Duration;
import java.util.*;
import java.util.concurrent.*;

@Plugin(id="velora-gateway", name="Velora Velocity Gateway", version="0.7.0", authors={"Velora"})
public final class Gateway {
    private record Active(Routes.Snapshot snapshot, Map<String, Routes.Route> routes, Map<String, RegisteredServer> servers) {}
    private final ProxyServer proxy; private final Logger log; private final Path directory;
    private final Gson gson = new Gson();
    private final ScheduledExecutorService worker = Executors.newSingleThreadScheduledExecutor();
    private final HttpClient http = HttpClient.newBuilder().connectTimeout(Duration.ofSeconds(5)).build();
    private volatile Active active = new Active(new Routes.Snapshot(0, List.of()), Map.of(), Map.of());
    private final Map<UUID, String> assignments = new ConcurrentHashMap<>();
    private final Deque<Map<String,String>> events = new ArrayDeque<>();
    private Controller controller; private String error, reportError;
    @FunctionalInterface interface SettingsLoader { Settings load() throws Exception; }
    private final SettingsLoader settings;
    @Inject public Gateway(ProxyServer proxy, Logger log, @DataDirectory Path directory) { this(proxy,log,directory,()->Settings.load(directory,System.getenv())); }
    Gateway(ProxyServer proxy, Logger log, Path directory, SettingsLoader settings) { this.proxy=proxy; this.log=log; this.directory=directory; this.settings=settings; }
    @Subscribe public void initialize(ProxyInitializeEvent event) {
        try {
            Settings configuration=settings.load();controller=new Controller(http,configuration.panel(),configuration.token());
            configuration.verifyAuthentication(proxy.getConfiguration().isOnlineMode(),java.lang.management.ManagementFactory.getRuntimeMXBean().getInputArguments());
            Files.createDirectories(directory);
            Path snapshot = directory.resolve("routes.json");
            if (Files.exists(snapshot)) apply(gson.fromJson(Files.readString(snapshot), Routes.Snapshot.class), false);
            worker.scheduleWithFixedDelay(this::sync, 0, 5, TimeUnit.SECONDS);
        } catch (Exception e) { log.error("Gateway failed closed: {}", e.toString()); worker.shutdownNow(); }
    }
    private synchronized void apply(Routes.Snapshot snapshot, boolean persist) throws Exception {
        Map<String, Routes.Route> routes = Routes.validate(snapshot);
        Map<String, RegisteredServer> servers = new HashMap<>(); List<RegisteredServer> added = new ArrayList<>();
        try {
            for (Routes.Route r : routes.values()) if (r.enabled()) {
                if (r.equals(active.routes.get(r.hostname())) && active.servers.containsKey(r.hostname())) {
                    servers.put(r.hostname(), active.servers.get(r.hostname())); continue;
                }
                String name = "velora-gateway-" + UUID.randomUUID();
                RegisteredServer server = proxy.registerServer(new ServerInfo(name, new InetSocketAddress(r.address(), r.port())));
                added.add(server); servers.put(r.hostname(), server);
            }
            if (persist) {
                Path temp = directory.resolve("routes.json.tmp"); Files.writeString(temp, gson.toJson(snapshot));
                Files.move(temp, directory.resolve("routes.json"), StandardCopyOption.ATOMIC_MOVE, StandardCopyOption.REPLACE_EXISTING);
            }
        } catch (Exception e) { added.forEach(s -> proxy.unregisterServer(s.getServerInfo())); throw e; }
        Active old = active; active = new Active(snapshot, routes, Map.copyOf(servers));
        // Changed/removed routes terminate old sessions; nobody is moved to another experience.
        for (Player p : proxy.getAllPlayers()) {
            String host = assignments.get(p.getUniqueId());
            if (host != null && !Objects.equals(old.routes.get(host), routes.get(host))) p.disconnect(Component.text("This route changed. Please reconnect."));
        }
        old.servers.values().stream().filter(s -> !servers.containsValue(s)).forEach(s -> proxy.unregisterServer(s.getServerInfo()));
    }
    private synchronized void record(String hostname, String kind) {
        if (events.size() == 50) events.removeFirst();
        events.addLast(Map.of("at", java.time.Instant.now().toString(), "hostname", hostname, "kind", kind));
    }
    private synchronized List<Map<String,String>> recentEvents() { return List.copyOf(events); }
    private void sync() {
        try {
            Routes.Snapshot next = controller.fetch();
            if (next.revision() < active.snapshot.revision()) throw new IllegalArgumentException("Stale snapshot");
            if (!next.equals(active.snapshot)) {
                if (next.revision() == active.snapshot.revision()) throw new IllegalArgumentException("Revision conflict");
                apply(next, true);
            }
            if (error!=null) log.info("Gateway route sync recovered");
            error = null;
        } catch (Exception e) {
            String next="Route sync failed; retaining last-known-good snapshot: "+failure(e);
            if (!next.equals(error)) log.warn(next);
            error=next;
        }
        try {
            Active state = active; List<Map<String,Object>> health = new ArrayList<>();
            for (Routes.Route r : state.routes.values()) {
                boolean reachable = false;
                if (r.enabled()) try (Socket socket = new Socket()) { socket.connect(new InetSocketAddress(r.address(), r.port()), 500); reachable = true; } catch (Exception ignored) {}
                health.add(Map.of("id", r.id(), "reachable", reachable, "enabled", r.enabled()));
            }
            Map<String,Object> report = new HashMap<>(); report.put("revision", state.snapshot.revision()); report.put("connections", assignments.size()); report.put("routes", health); report.put("error", error);
            report.put("events", recentEvents());
            controller.report(report);
            if (reportError==null || !reportError.isEmpty()) log.info("Gateway status report accepted by Panel");
            reportError="";
        } catch (Exception e) {
            String next="Gateway status report failed: "+failure(e);
            if (!next.equals(reportError)) log.warn(next);
            reportError=next;
        }
    }
    private String failure(Exception e) {
        if (e instanceof InterruptedException) {Thread.currentThread().interrupt();return "Request interrupted";}
        if (e instanceof Controller.Failure) return e.getMessage();
        if (e instanceof IllegalArgumentException) return "Invalid route revision or snapshot";
        if (e instanceof java.io.IOException) return "Route snapshot persistence failed; check plugin directory permissions and storage";
        return "Unexpected "+e.getClass().getSimpleName();
    }
    private String host(InboundConnection connection) { try { return Routes.hostname(connection.getVirtualHost().orElseThrow().getHostString()); } catch (Exception e) { return ""; } }
    @Subscribe(order=PostOrder.LAST) public void login(PreLoginEvent event) {
        if (!active.servers.containsKey(host(event.getConnection()))) event.setResult(PreLoginEvent.PreLoginComponentResult.denied(Component.text("Unknown or disabled experience hostname.")));
        else if (event.getResult().isAllowed()) event.setResult(PreLoginEvent.PreLoginComponentResult.forceOnlineMode());
    }
    @Subscribe(order=PostOrder.LAST) public void initial(PlayerChooseInitialServerEvent event) {
        String host = host(event.getPlayer()); RegisteredServer server = active.servers.get(host);
        event.setInitialServer(server);
        if (server == null) event.getPlayer().disconnect(Component.text("Experience unavailable."));
        else { assignments.put(event.getPlayer().getUniqueId(), host); record(host, "assigned"); log.info("Gateway connection assigned to {}", host); }
    }
    @Subscribe(order=PostOrder.LAST) public void connect(ServerPreConnectEvent event) {
        String host = assignments.get(event.getPlayer().getUniqueId());
        RegisteredServer allowed = host == null ? null : active.servers.get(host);
        if (allowed == null || !allowed.equals(event.getOriginalServer()) || event.getResult().getServer().filter(allowed::equals).isEmpty()) event.setResult(ServerPreConnectEvent.ServerResult.denied());
    }
    @Subscribe(order=PostOrder.LAST) public void kicked(KickedFromServerEvent event) { event.setResult(KickedFromServerEvent.DisconnectPlayer.create(Component.text("Experience disconnected. Please reconnect."))); }
    @Subscribe public void disconnect(DisconnectEvent event) { String host = assignments.remove(event.getPlayer().getUniqueId()); if (host != null) { record(host, "closed"); log.info("Gateway connection closed"); } }
    @Subscribe public void shutdown(ProxyShutdownEvent event) { worker.shutdownNow(); }
}
