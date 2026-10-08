package net.scopenet.client;

import com.google.gson.JsonObject;
import net.fabricmc.fabric.api.client.networking.v1.ClientPlayConnectionEvents;
import net.fabricmc.fabric.api.client.networking.v1.ClientPlayNetworking;
import net.fabricmc.fabric.api.networking.v1.PacketByteBufs;
import net.minecraft.client.Minecraft;
import net.minecraft.network.FriendlyByteBuf;
import net.minecraft.network.chat.Component;
import net.minecraft.client.gui.components.toasts.SystemToast;
import net.minecraft.resources.ResourceLocation;

/** The channel to the Velora server mod or plugin. Messages are JSON in a Minecraft UTF string. */
public final class Link {
    private static final ResourceLocation TO_CLIENT = new ResourceLocation("scopenet", "s2c");
    private static final ResourceLocation TO_SERVER = new ResourceLocation("scopenet", "c2s");

    private final ClientState state;
    private final ClientConfig config;

    Link(ClientState state, ClientConfig config) { this.state = state; this.config = config; }

    void register() {
        ClientPlayNetworking.registerGlobalReceiver(TO_CLIENT, (client, handler, buf, sender) -> {
            String json = buf.readUtf(32767 * 3);
            client.execute(() -> {
                toast(state.handle(json));
                String open = state.pendingOpen;
                state.pendingOpen = null;
                // A right-clicked market point opens its window, unless another screen is already up.
                if (open != null && config.enabled && client.screen == null) {
                    if (open.equals("market")) client.setScreen(new net.scopenet.client.screen.MarketScreen());
                }
            });
        });
        ClientPlayConnectionEvents.JOIN.register((handler, sender, client) -> send("hello"));
        ClientPlayConnectionEvents.DISCONNECT.register((handler, client) -> state.reset());
    }

    public void send(String type) {
        JsonObject m = new JsonObject();
        m.addProperty("t", type);
        if (type.equals("hello")) m.addProperty("mod", "0.1.0");
        FriendlyByteBuf buf = PacketByteBufs.create();
        buf.writeUtf(m.toString(), 32767 * 3);
        try { ClientPlayNetworking.send(TO_SERVER, buf); } catch (RuntimeException ignored) { /* not in a world */ }
    }

    /** Run a server command as the player, exactly as if typed. */
    public static void command(String command) {
        Minecraft mc = Minecraft.getInstance();
        if (mc.player != null) mc.player.connection.sendCommand(command.startsWith("/") ? command.substring(1) : command);
    }

    private void toast(String[] t) {
        if (t == null) return;
        boolean wanted = switch (t[0]) {
            case "level_up" -> config.notifications.levelUp;
            case "achievement" -> config.notifications.achievements;
            case "guild_join", "guild_leave" -> config.notifications.guild;
            case "error" -> config.notifications.errors;
            default -> true;
        };
        if (!wanted || !config.enabled) return;
        SystemToast.add(Minecraft.getInstance().getToasts(), SystemToast.SystemToastIds.PERIODIC_NOTIFICATION, Component.literal(t[1]), Component.literal(t[2]));
    }
}
