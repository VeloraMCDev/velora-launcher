package net.scopenet.fabric.features;

import net.minecraft.resources.ResourceLocation;
import net.minecraft.server.level.ServerPlayer;
import net.scopenet.core.Placeholders;

import java.lang.reflect.Method;
import java.lang.reflect.Proxy;
import java.util.Optional;

/**
 * Registers %scopenet:level%, %scopenet:balance%… with Placeholder API (Fabric) when it is installed. Done through
 * reflection so SCOPENET builds and runs without it and tolerates small API differences; any problem just leaves
 * the placeholders off and is reported in /scopenet status.
 */
final class PlaceholderApiModule {
    private PlaceholderApiModule() {}

    /** Returns how many placeholders were registered; throws if the API isn't usable. */
    static int register(Placeholders values) throws ReflectiveOperationException {
        Class<?> placeholders = Class.forName("eu.pb4.placeholders.api.Placeholders");
        Class<?> handlerType = Class.forName("eu.pb4.placeholders.api.PlaceholderHandler");
        Class<?> resultType = Class.forName("eu.pb4.placeholders.api.PlaceholderResult");
        Method register = placeholders.getMethod("register", ResourceLocation.class, handlerType);
        Method value = resultType.getMethod("value", String.class);
        Method invalid = resultType.getMethod("invalid", String.class);

        int count = 0;
        for (String key : Placeholders.KEYS) {
            Object handler = Proxy.newProxyInstance(handlerType.getClassLoader(), new Class<?>[]{handlerType}, (proxy, method, args) -> {
                if (method.getDeclaringClass() == Object.class) return method.invoke(new Object(), args);
                Object context = args[0];
                ServerPlayer player = (ServerPlayer) context.getClass().getMethod("player").invoke(context);
                if (player == null) return invalid.invoke(null, "No player");
                Optional<String> text = values.resolve(player.getUUID(), key);
                return text.isPresent() ? value.invoke(null, text.get()) : invalid.invoke(null, "Unknown placeholder");
            });
            register.invoke(null, new ResourceLocation("scopenet", key), handler);
            count++;
        }
        return count;
    }
}
