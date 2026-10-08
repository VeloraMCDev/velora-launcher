package net.scopenet.client.screen;

import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.components.AbstractSliderButton;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;
import net.scopenet.client.ClientConfig;
import net.scopenet.client.ScopenetClient;
import net.scopenet.client.Ui;
import net.scopenet.client.hud.Widgets;

import java.util.function.BooleanSupplier;
import java.util.function.Consumer;

/** Every client option in one place. Changes apply immediately and are saved when you leave. */
public final class SettingsScreen extends Screen {
    private final Screen parent;
    private final ClientConfig config = ScopenetClient.config();

    public SettingsScreen(Screen parent) { super(Component.literal("Velora settings")); this.parent = parent; }

    private static Component onOff(boolean on) { return Component.literal(on ? "§aOn" : "§cOff"); }

    private Button toggle(int x, int y, int w, String label, BooleanSupplier get, Consumer<Boolean> set) {
        return Button.builder(Component.literal(label + ": ").append(onOff(get.getAsBoolean())), b -> {
            set.accept(!get.getAsBoolean());
            b.setMessage(Component.literal(label + ": ").append(onOff(get.getAsBoolean())));
        }).bounds(x, y, w, 20).build();
    }

    private static final class Slider extends AbstractSliderButton {
        private final String label;
        private final double min, max;
        private final Consumer<Double> apply;
        private final boolean percent;

        Slider(int x, int y, int w, String label, double min, double max, double value, boolean percent, Consumer<Double> apply) {
            super(x, y, w, 20, Component.empty(), (value - min) / (max - min));
            this.label = label; this.min = min; this.max = max; this.apply = apply; this.percent = percent;
            updateMessage();
        }

        private double current() { return min + (max - min) * this.value; }

        @Override protected void updateMessage() {
            setMessage(Component.literal(label + ": " + (percent ? Math.round(current() * 100) + "%" : String.format("%.2f", current()))));
        }

        @Override protected void applyValue() { apply.accept(current()); }
    }

    @Override protected void init() {
        int colW = 150, gap = 8;
        int x1 = width / 2 - colW - gap / 2, x2 = width / 2 + gap / 2;
        int y = 34, step = 22;

        addRenderableWidget(toggle(x1, y, colW, "HUD", () -> config.enabled, v -> config.enabled = v));
        addRenderableWidget(toggle(x2, y, colW, "Hide with F3", () -> config.hud.hideInF3, v -> config.hud.hideInF3 = v));
        y += step;
        addRenderableWidget(new Slider(x1, y, colW, "Opacity", 0.1, 1.0, config.hud.opacity, true, v -> config.hud.opacity = v.floatValue()));
        addRenderableWidget(new Slider(x2, y, colW, "Size", 0.5, 2.0, config.hud.scale, true, v -> config.hud.scale = v.floatValue()));
        y += step;
        addRenderableWidget(toggle(x1, y, colW, "Claim borders", () -> config.hud.claimBorders, v -> config.hud.claimBorders = v));
        addRenderableWidget(toggle(x2, y, colW, "Only when sneaking", () -> config.hud.claimBordersOnlyWhenSneaking, v -> config.hud.claimBordersOnlyWhenSneaking = v));
        y += step + 8;

        // One button per widget: On -> Compact -> Off.
        int i = 0;
        for (var widget : Widgets.ALL) {
            ClientConfig.WidgetConfig c = config.widget(widget.id);
            int x = i % 2 == 0 ? x1 : x2;
            addRenderableWidget(Button.builder(widgetLabel(widget.label, c), b -> {
                if (!c.enabled) { c.enabled = true; c.compact = false; }
                else if (!c.compact) c.compact = true;
                else { c.enabled = false; c.compact = false; }
                b.setMessage(widgetLabel(widget.label, c));
            }).bounds(x, y, colW, 20).build());
            if (i % 2 == 1) y += step;
            i++;
        }
        if (i % 2 == 1) y += step;
        y += 8;

        addRenderableWidget(toggle(x1, y, colW, "Level-up toasts", () -> config.notifications.levelUp, v -> config.notifications.levelUp = v));
        addRenderableWidget(toggle(x2, y, colW, "Achievement toasts", () -> config.notifications.achievements, v -> config.notifications.achievements = v));
        y += step;
        addRenderableWidget(toggle(x1, y, colW, "Guild toasts", () -> config.notifications.guild, v -> config.notifications.guild = v));
        addRenderableWidget(toggle(x2, y, colW, "Error toasts", () -> config.notifications.errors, v -> config.notifications.errors = v));
        y += step + 8;

        addRenderableWidget(Button.builder(Component.literal("Edit HUD layout…"), b -> minecraft.setScreen(new HudEditorScreen(this))).bounds(x1, y, colW, 20).build());
        addRenderableWidget(Button.builder(Component.literal("Done"), b -> onClose()).bounds(x2, y, colW, 20).build());
    }

    private static Component widgetLabel(String label, ClientConfig.WidgetConfig c) {
        return Component.literal(label + ": ").append(Component.literal(!c.enabled ? "§cOff" : c.compact ? "§eCompact" : "§aOn"));
    }

    @Override public void render(GuiGraphics g, int mouseX, int mouseY, float delta) {
        renderBackground(g);
        g.drawCenteredString(font, "Velora settings", width / 2, 14, Ui.ACCENT);
        super.render(g, mouseX, mouseY, delta);
    }

    @Override public void onClose() {
        config.save();
        minecraft.setScreen(parent);
    }
}
