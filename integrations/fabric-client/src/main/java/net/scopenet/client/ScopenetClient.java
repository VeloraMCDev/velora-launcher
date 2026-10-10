package net.scopenet.client;

import com.mojang.blaze3d.platform.InputConstants;
import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.fabric.api.client.keybinding.v1.KeyBindingHelper;
import net.fabricmc.fabric.api.client.rendering.v1.HudRenderCallback;
import net.fabricmc.fabric.api.client.rendering.v1.WorldRenderEvents;
import net.minecraft.client.KeyMapping;
import net.scopenet.client.hud.ClaimBorders;
import net.scopenet.client.hud.Hud;
import net.scopenet.client.screen.MenuScreen;
import org.lwjgl.glfw.GLFW;

/**
 * Velora client: HUD widgets, claim borders, and the market window. Everything it shows comes from the
 * server over the {@code scopenet:s2c} channel, and everything it does is an ordinary command, so the server stays
 * in charge. If the server has no Velora, this does nothing.
 */
public final class ScopenetClient implements ClientModInitializer {
    private static ClientConfig config;
    private static ClientState state;
    private static Link link;

    public static ClientConfig config() { return config; }
    public static ClientState state() { return state; }
    public static Link link() { return link; }

    @Override public void onInitializeClient() {
        net.minecraft.client.gui.screens.MenuScreens.register(net.scopenet.core.vault.VaultMenu.TYPE, net.scopenet.client.screen.VaultScreen::new);
        config = ClientConfig.load();
        state = new ClientState();
        link = new Link(state, config);
        link.register();

        Hud hud = new Hud(state, config);
        HudRenderCallback.EVENT.register((graphics, tickDelta) -> hud.render(graphics, tickDelta));
        ClaimBorders borders = new ClaimBorders(state, config);
        WorldRenderEvents.LAST.register(borders::render);

        KeyMapping menu = KeyBindingHelper.registerKeyBinding(new KeyMapping("key.scopenet.menu", InputConstants.Type.KEYSYM, GLFW.GLFW_KEY_K, "key.categories.scopenet"));
        KeyMapping toggle = KeyBindingHelper.registerKeyBinding(new KeyMapping("key.scopenet.toggle_hud", InputConstants.Type.KEYSYM, GLFW.GLFW_KEY_UNKNOWN, "key.categories.scopenet"));
        KeyMapping vault = KeyBindingHelper.registerKeyBinding(new KeyMapping("key.scopenet.vault", InputConstants.Type.KEYSYM, GLFW.GLFW_KEY_V, "key.categories.scopenet"));
        KeyMapping map = KeyBindingHelper.registerKeyBinding(new KeyMapping("key.scopenet.map", InputConstants.Type.KEYSYM, GLFW.GLFW_KEY_M, "key.categories.scopenet"));
        ClientTickEvents.END_CLIENT_TICK.register(client -> {
            link.mapTick();
            while (menu.consumeClick()) if (client.screen == null && client.player != null) client.setScreen(new MenuScreen());
            while (toggle.consumeClick()) { config.enabled = !config.enabled; config.save(); }
            while (vault.consumeClick()) if (client.screen == null && state.connected && config.module("vaults") && state.module("vaults")) Link.command("vault 1");
            while (map.consumeClick()) if (client.screen == null && state.connected && config.module("map") && state.module("map")) client.setScreen(new net.scopenet.client.screen.MapScreen());
        });
    }
}
