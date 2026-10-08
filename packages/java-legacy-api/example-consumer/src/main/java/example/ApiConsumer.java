package example;

import java.util.UUID;
import java.util.concurrent.CompletableFuture;
import net.scopenet.api.ScopenetApi;
import net.scopenet.api.ScopenetApiProvider;

/** Compile-only consumer: the integration supplies the same API class at runtime. */
public final class ApiConsumer {
    private ApiConsumer() {}
    public static CompletableFuture<String> playerName(UUID id) {
        ScopenetApi api = ScopenetApiProvider.get();
        return api.getPlayer(id).thenApply(player -> player.map(p -> p.name()).orElse("Unknown player"));
    }
}
