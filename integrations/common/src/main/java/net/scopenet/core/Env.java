package net.scopenet.core;

import java.nio.file.Path;
import java.util.Random;
import java.util.concurrent.Callable;
import java.util.function.Consumer;
import java.util.function.LongSupplier;
import java.util.logging.Logger;

/** Everything a command needs: the platform, the panel, settings and a way to do network work safely. */
public final class Env {
    public final Platform platform;
    public final Panel panel;
    public volatile Features features;
    /** Utility-command settings from the panel; platforms point this at the integration. */
    public volatile java.util.function.Supplier<Utilities> utilities = () -> Utilities.DEFAULT;
    public volatile java.util.function.Supplier<CoreModules> modules = CoreModules::all;
    public final Path dataDir;
    public final LongSupplier clock;
    public final Random rng;
    public final Logger log;

    public Env(Platform platform, Panel panel, Features features, Path dataDir, LongSupplier clock, Random rng, Logger log) {
        this.platform = platform; this.panel = panel; this.features = features; this.dataDir = dataDir;
        this.clock = clock; this.rng = rng; this.log = log;
    }

    /** Do {@code work} off the server thread, then call back on it with the result or an error message. */
    public <T> void io(Callable<T> work, Consumer<T> ok, Consumer<String> fail) {
        platform.runAsync(() -> {
            try {
                T result = work.call();
                platform.runMain(() -> ok.accept(result));
            } catch (Exception e) {
                String message = e.getMessage() != null && !e.getMessage().isBlank() ? e.getMessage() : e.getClass().getSimpleName();
                platform.runMain(() -> fail.accept(message));
            }
        });
    }

    public String money(double value) { return Format.money(features.currencySymbol(), value); }
}
