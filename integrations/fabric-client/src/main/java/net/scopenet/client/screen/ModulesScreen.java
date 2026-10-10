package net.scopenet.client.screen;

import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;
import net.scopenet.client.ScopenetClient;
import net.scopenet.client.Ui;

/** Local display preferences can only narrow the server's module policy. */
public final class ModulesScreen extends Screen {
    private final Screen parent;
    public ModulesScreen(Screen parent) { super(Component.literal("Velora Core modules")); this.parent = parent; }
    @Override protected void init() {
        int index = 0;
        var config = ScopenetClient.config();
        for (String id : new String[]{"map", "economy", "vaults", "casino", "analytics", "factions"}) {
            boolean allowed = !ScopenetClient.state().connected || ScopenetClient.state().module(id);
            Button button = Button.builder(label(id, allowed), b -> {
                config.modules.put(id, !config.module(id)); config.save(); b.setMessage(label(id, allowed));
            }).bounds(width / 2 - 110, 42 + index++ * 23, 220, 20).build();
            button.active = allowed; addRenderableWidget(button);
        }
        addRenderableWidget(Button.builder(Component.literal("Done"), b -> onClose()).bounds(width / 2 - 50, 188, 100, 20).build());
    }
    private Component label(String id, boolean allowed) {
        return Component.literal(id + ": " + (!allowed ? "disabled by server" : ScopenetClient.config().module(id) ? "visible" : "hidden"));
    }
    @Override public void render(GuiGraphics graphics, int mouseX, int mouseY, float delta) {
        renderBackground(graphics); graphics.drawCenteredString(font, title, width / 2, 18, Ui.ACCENT);
        super.render(graphics, mouseX, mouseY, delta);
    }
    @Override public void onClose() { ScopenetClient.config().save(); minecraft.setScreen(parent); }
}
