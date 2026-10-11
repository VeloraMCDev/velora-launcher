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
}
