package net.velora.gateway;
import com.velocitypowered.api.proxy.*;
import com.velocitypowered.api.proxy.server.*;
import com.velocitypowered.api.event.player.*;
import com.velocitypowered.api.event.connection.*;
import org.slf4j.Logger;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;
import java.nio.file.Path;
import java.net.InetSocketAddress;
import java.util.*;
import java.lang.reflect.Method;
import static org.mockito.Mockito.*;
import static org.junit.jupiter.api.Assertions.*;

class GatewayTest {
    @TempDir Path dir;
    @Test void syncFailuresAreReportedOncePreserveRoutesAndLogRecovery() throws Exception {
        ProxyServer proxy=mock(ProxyServer.class);Logger logger=mock(Logger.class);
        when(proxy.getAllPlayers()).thenReturn(List.of());
        Gateway g=new Gateway(proxy,logger,dir);Controller controller=mock(Controller.class);
        var snapshot=new Routes.Snapshot(1,List.of(new Routes.Route("smp","SMP","smp.example.org","100.64.0.2",25565,false)));
        apply(g,snapshot);String persisted=java.nio.file.Files.readString(dir.resolve("routes.json"));
        var field=Gateway.class.getDeclaredField("controller");field.setAccessible(true);field.set(g,controller);
        var sync=Gateway.class.getDeclaredMethod("sync");sync.setAccessible(true);
        when(controller.fetch()).thenThrow(new Controller.Failure("HTTP 401"),new Controller.Failure("HTTP 401"))
            .thenReturn(snapshot);
        doThrow(new Controller.Failure("HTTP 401")).doThrow(new Controller.Failure("HTTP 401")).doNothing().when(controller).report(any());
        sync.invoke(g);sync.invoke(g);sync.invoke(g);
        assertEquals(persisted,java.nio.file.Files.readString(dir.resolve("routes.json")));
        verify(logger,times(1)).warn("Route sync failed; retaining last-known-good snapshot: HTTP 401");
        verify(logger,times(1)).warn("Gateway status report failed: HTTP 401");
        verify(logger).info("Gateway route sync recovered");verify(logger).info("Gateway status report accepted by Panel");
        verify(proxy,never()).registerServer(any());
    }
    @Test void syncDoesNotLogAnUnexpectedExceptionContainingCredentials() throws Exception {
        ProxyServer proxy=mock(ProxyServer.class);Logger logger=mock(Logger.class);
        Gateway g=new Gateway(proxy,logger,dir);Controller controller=mock(Controller.class);
        var field=Gateway.class.getDeclaredField("controller");field.setAccessible(true);field.set(g,controller);
        when(controller.fetch()).thenThrow(new IllegalArgumentException("synthetic-private-credential"));
        doThrow(new IllegalArgumentException("synthetic-private-credential")).when(controller).report(any());
        var sync=Gateway.class.getDeclaredMethod("sync");sync.setAccessible(true);sync.invoke(g);
        verify(logger,never()).warn(contains("synthetic-private-credential"));
    }
    @Test void absentStartupConfigurationFailsClosedWithAnActionableSettingName() {
        ProxyServer proxy=mock(ProxyServer.class);Logger logger=mock(Logger.class);
        Gateway g=new Gateway(proxy,logger,dir,()->Settings.load(dir,Map.of()));
        g.initialize(new com.velocitypowered.api.event.proxy.ProxyInitializeEvent());
        verify(logger).error(eq("Gateway failed closed: {}"),contains("Missing VELORA_GATEWAY_PANEL"));
        verify(proxy,never()).registerServer(any());
        var login=new PreLoginEvent(player("smp.velora.scopedd.lol"),"Synthetic");g.login(login);
        assertFalse(login.getResult().isAllowed());
    }
    private Gateway gateway(ProxyServer proxy) {
        when(proxy.getAllPlayers()).thenReturn(List.of());
        when(proxy.registerServer(any())).thenAnswer(i -> { RegisteredServer s = mock(RegisteredServer.class); when(s.getServerInfo()).thenReturn(i.getArgument(0)); return s; });
        return new Gateway(proxy, mock(Logger.class), dir);
    }
    private void apply(Gateway g, Routes.Snapshot s) throws Exception {
        Method m = Gateway.class.getDeclaredMethod("apply", Routes.Snapshot.class, boolean.class); m.setAccessible(true); m.invoke(g, s, true);
    }
    private Player player(String host) {
        Player p = mock(Player.class); when(p.getUniqueId()).thenReturn(UUID.randomUUID()); when(p.getVirtualHost()).thenReturn(Optional.of(InetSocketAddress.createUnresolved(host,25565))); return p;
    }
    @Test void rejectsSwitchingAndDisabledRoutes() throws Exception {
        ProxyServer proxy = mock(ProxyServer.class); Gateway g = gateway(proxy);
        var smp = new Routes.Route("smp","SMP","smp.velora.scopedd.lol","100.64.0.2",25565,true);
        var unleashed = new Routes.Route("unleashed","Unleashed","unleashed.velora.scopedd.lol","100.64.0.3",25565,true);
        apply(g, new Routes.Snapshot(1,List.of(smp,unleashed)));
        Player p = player(smp.hostname()); var initial = new PlayerChooseInitialServerEvent(p, null); g.initial(initial);
        RegisteredServer assigned = initial.getInitialServer().orElseThrow();
        var own = new ServerPreConnectEvent(p,assigned); g.connect(own); assertTrue(own.getResult().isAllowed());
        var other = new ServerPreConnectEvent(p,mock(RegisteredServer.class)); g.connect(other); assertFalse(other.getResult().isAllowed());
        apply(g, new Routes.Snapshot(2,List.of(unleashed)));
        var disabled = new ServerPreConnectEvent(p,assigned); g.connect(disabled); assertFalse(disabled.getResult().isAllowed());
        verify(proxy).unregisterServer(assigned.getServerInfo());
    }
    @Test void unchangedSessionsAndLastKnownGoodSurviveRejectedUpdate() throws Exception {
        ProxyServer proxy = mock(ProxyServer.class); Gateway g = gateway(proxy);
        var r = new Routes.Route("smp","SMP","smp.velora.scopedd.lol","100.64.0.2",25565,true);
        apply(g,new Routes.Snapshot(1,List.of(r)));
        Player p = player(r.hostname()); var initial = new PlayerChooseInitialServerEvent(p,null); g.initial(initial);
        apply(g,new Routes.Snapshot(2,List.of(r)));
        verify(proxy,times(1)).registerServer(any()); verify(proxy,never()).unregisterServer(any());
        String good = java.nio.file.Files.readString(dir.resolve("routes.json"));
        assertThrows(Exception.class,() -> apply(g,new Routes.Snapshot(3,List.of(r,r))));
        assertEquals(good,java.nio.file.Files.readString(dir.resolve("routes.json")));
        var own = new ServerPreConnectEvent(p,initial.getInitialServer().orElseThrow()); g.connect(own); assertTrue(own.getResult().isAllowed());
        ProxyServer restoredProxy = mock(ProxyServer.class); Gateway restored = gateway(restoredProxy);
        apply(restored, new com.google.gson.Gson().fromJson(good, Routes.Snapshot.class));
        var restoredInitial = new PlayerChooseInitialServerEvent(player(r.hostname()), null); restored.initial(restoredInitial);
        assertTrue(restoredInitial.getInitialServer().isPresent());
    }
    @Test void unknownHostnameNeverGetsInitialServer() {
        ProxyServer proxy = mock(ProxyServer.class); Gateway g = gateway(proxy);
        Player p = player("unknown.example.com"); var initial = new PlayerChooseInitialServerEvent(p,null); g.initial(initial);
        assertTrue(initial.getInitialServer().isEmpty()); verify(p).disconnect(any());
    }
    @Test void persistenceFailureKeepsPreviousRouteAndCleansNewRegistration() throws Exception {
        ProxyServer proxy = mock(ProxyServer.class); Gateway g = gateway(proxy);
        var r = new Routes.Route("smp","SMP","smp.velora.scopedd.lol","100.64.0.2",25565,true);
        apply(g,new Routes.Snapshot(1,List.of(r)));
        Player p = player(r.hostname()); var initial = new PlayerChooseInitialServerEvent(p,null); g.initial(initial);
        RegisteredServer old = initial.getInitialServer().orElseThrow();
        java.nio.file.Files.createDirectory(dir.resolve("routes.json.tmp"));
        var changed = new Routes.Route(r.id(),r.experience(),r.hostname(),"100.64.0.3",25566,true);
        assertThrows(Exception.class, () -> apply(g,new Routes.Snapshot(2,List.of(changed))));
        verify(proxy, never()).unregisterServer(old.getServerInfo());
        verify(proxy, times(1)).unregisterServer(any());
        var own = new ServerPreConnectEvent(p,old); g.connect(own); assertTrue(own.getResult().isAllowed());
    }
    @Test void knownHostRequiresOnlineAuthenticationAndKickCannotFallback() throws Exception {
        ProxyServer proxy = mock(ProxyServer.class); Gateway g = gateway(proxy);
        var r = new Routes.Route("smp","SMP","smp.velora.scopedd.lol","100.64.0.2",25565,true);
        apply(g,new Routes.Snapshot(1,List.of(r)));
        var known = new PreLoginEvent(player(r.hostname()), "Synthetic");
        known.setResult(PreLoginEvent.PreLoginComponentResult.forceOfflineMode()); g.login(known);
        assertSame(PreLoginEvent.PreLoginComponentResult.forceOnlineMode(), known.getResult());
        var unknown = new PreLoginEvent(player("unknown.example.com"), "Synthetic"); g.login(unknown);
        assertFalse(unknown.getResult().isAllowed());
        var kicked = mock(KickedFromServerEvent.class); g.kicked(kicked);
        verify(kicked).setResult(isA(KickedFromServerEvent.DisconnectPlayer.class));
    }
}
