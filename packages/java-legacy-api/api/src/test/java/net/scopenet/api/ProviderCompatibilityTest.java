package net.scopenet.api;

import java.lang.reflect.Proxy;
import java.util.Optional;
import java.util.UUID;
import java.util.concurrent.CompletableFuture;
import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

class ProviderCompatibilityTest {
    private ScopenetApi synthetic() {
        return (ScopenetApi) Proxy.newProxyInstance(ScopenetApi.class.getClassLoader(), new Class<?>[]{ScopenetApi.class},
            (proxy, method, args) -> switch (method.getName()) {
                case "getPlayer" -> CompletableFuture.completedFuture(Optional.empty());
                case "getCachedPlayer" -> Optional.empty();
                default -> throw new UnsupportedOperationException(method.getName());
            });
    }

    @Test void consumer_and_replacement_share_provider_identity_without_stale_unregister() {
        assertFalse(ScopenetApiProvider.isAvailable());
        assertThrows(IllegalStateException.class, ScopenetApiProvider::get);
        ScopenetApi first = synthetic();
        ScopenetApi replacement = synthetic();
        try {
            ScopenetApiProvider.register(first);
            assertSame(first, ScopenetApiProvider.get());
            assertTrue(ScopenetApiProvider.get().getPlayer(UUID.randomUUID()).join().isEmpty());
            ScopenetApiProvider.register(replacement);
            ScopenetApiProvider.unregister(first);
            assertSame(replacement, ScopenetApiProvider.get());
            assertTrue(ScopenetApiProvider.get().getCachedPlayer(UUID.randomUUID()).isEmpty());
        } finally { ScopenetApiProvider.unregister(replacement); }
        assertFalse(ScopenetApiProvider.isAvailable());
        assertThrows(IllegalStateException.class, ScopenetApiProvider::get);
    }
}
