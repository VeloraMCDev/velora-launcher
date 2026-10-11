package net.velora.paper.panel;

import org.junit.jupiter.api.Test;

import java.util.List;
import java.util.UUID;

import static org.junit.jupiter.api.Assertions.*;

class PanelResultTest {

    @Test
    void testHelloResultSuccess() {
        HelloResult result = HelloResult.success(42, "SMP Survival", "Velora", true);
        assertTrue(result.ok());
        assertEquals(42, result.serverId());
        assertEquals("SMP Survival", result.serverName());
        assertEquals("Velora", result.brand());
        assertTrue(result.requireLauncher());
        assertNull(result.errorMessage());
    }

    @Test
    void testHelloResultFailure() {
        HelloResult result = HelloResult.failure("Invalid token");
        assertFalse(result.ok());
        assertEquals(-1, result.serverId());
        assertEquals("Invalid token", result.errorMessage());
    }

    @Test
    void testLoginResultAllow() {
        LoginResult result = LoginResult.allow();
        assertTrue(result.allowed());
        assertNull(result.message());
    }

    @Test
    void testLoginResultDeny() {
        LoginResult result = LoginResult.deny("You need a Velora account to join.");
        assertFalse(result.allowed());
        assertEquals("You need a Velora account to join.", result.message());
    }

    @Test
    void testSyncResultSuccess() {
        UUID kickId = UUID.randomUUID();
        SyncResult result = SyncResult.success(List.of(kickId));
        assertTrue(result.ok());
        assertEquals(1, result.kickPlayers().size());
        assertEquals(kickId, result.kickPlayers().get(0));
        assertNull(result.errorMessage());
    }

    @Test
    void testSyncResultFailure() {
        SyncResult result = SyncResult.failure("HTTP 500");
        assertFalse(result.ok());
        assertTrue(result.kickPlayers().isEmpty());
        assertEquals("HTTP 500", result.errorMessage());
    }
}
