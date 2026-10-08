package net.scopenet.client;

import com.google.gson.JsonObject;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.Font;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.world.item.ItemStack;

/** The on-screen modules. The in-game game HUD and the HUD editor draw them through the same code, so what you place is what you get. */
final class Hud {
    record Content(String label, String value, String sub, double fraction, String icon) {}
    private Hud() {}

    static String name(String id) {
        return switch (id) {
            case "level" -> "Progression"; case "balance" -> "Wallet"; case "guild" -> "Guild"; case "claim" -> "Territory"; case "quests" -> "Quest Tracker"; default -> "Clock";
        };
    }

    /** What a module shows right now, or null when it has nothing to say (then it is simply not drawn). */
    static Content content(String id) {
        Model m = Companion.model;
        switch (id) {
            case "level": return new Content("Level " + (int) Model.number(m.state, "level"), Math.round(Model.number(m.state, "xpPct")) + "% to next level", "", Model.number(m.state, "xpPct") / 100, "minecraft:experience_bottle");
            case "balance": return Model.flag(m.features, "economy") ? new Content("Wallet", m.money(Model.number(m.state, "balance")), "", -1, "minecraft:gold_ingot") : null;
            case "guild": {
                JsonObject guild = Model.object(m.state, "guild");
                return guild.isEmpty() ? null : new Content("Guild", "[" + Model.text(guild, "tag") + "] " + Model.text(guild, "name"), "", -1, "minecraft:shield");
            }
            case "claim": {
                JsonObject owner = Model.object(m.claims, "owner");
                return new Content("Territory", owner.isEmpty() ? "Wilderness" : Model.text(owner, "name"), "", -1, "minecraft:grass_block");
            }
            case "quests": {
                JsonObject row = Companion.pinnedQuest;
                if (row != null) {
                    JsonObject def = Model.object(row, "quest");
                    double target = Math.max(1, Model.number(def, "target_count")), done = Model.number(row, "progress");
                    return new Content("Quest · " + (int) done + " / " + (int) target, Model.text(def, "title"), Model.text(def, "description"), done / target, "minecraft:book");
                }
                JsonObject q = Model.object(m.state, "daily");
                double total = Math.max(1, Model.number(q, "total"));
                return new Content("Daily Quests", (int) Model.number(q, "done") + " of " + (int) Model.number(q, "total") + " done", "", Model.number(q, "done") / total, "minecraft:book");
            }
            default: return new Content("Local Time", java.time.LocalTime.now().withNano(0).toString(), "", -1, "minecraft:clock");
        }
    }

    /** Stand-in text so a module can be placed in the editor before any data has arrived. */
    static Content sample(String id) {
        Content real = Companion.model.connected ? content(id) : null;
        if (real != null) return real;
        return switch (id) {
            case "level" -> new Content("Level 12", "64% to next level", "", .64, "minecraft:experience_bottle");
            case "balance" -> new Content("Wallet", "$1,250.00", "", -1, "minecraft:gold_ingot");
            case "guild" -> new Content("Guild", "[SCP] Scopenet", "", -1, "minecraft:shield");
            case "claim" -> new Content("Territory", "Wilderness", "", -1, "minecraft:grass_block");
            case "quests" -> new Content("Quest · 12 / 50", "Skeleton Slayer", "Defeat 50 skeletons.", .24, "minecraft:book");
            default -> content(id);
        };
    }

    static int width(Font font, Content c) {
        int text = Math.max(font.width(c.value()), Math.max(font.width(c.label()), c.sub().isEmpty() ? 0 : Math.min(font.width(c.sub()), 170)));
        return Math.clamp(text + 36, 112, 214);
    }
    static int height(Content c) { return 28 + (c.sub().isEmpty() ? 0 : 10) + (c.fraction() >= 0 ? 5 : 0); }

    static void draw(GuiGraphicsExtractor g, Font font, Content c, int x, int y, int w, int h, boolean highlight) {
        int accent = Companion.preferences.accentColor();
        Look.tooltip(g, x, y, w, h, highlight ? 0xffffff : accent, Companion.preferences.opacity);
        ItemStack stack = Presentation.icon(c.icon());
        if (!stack.isEmpty()) g.item(stack, x + 6, y + 6);
        int tx = x + 27, tw = w - 33;
        g.text(font, font.plainSubstrByWidth(c.label(), tw), tx, y + 5, Look.alpha(accent, 1));
        g.text(font, font.plainSubstrByWidth(c.value(), tw), tx, y + 16, Look.TEXT);
        int line = y + 27;
        if (!c.sub().isEmpty()) { g.text(font, font.plainSubstrByWidth(c.sub(), tw), tx, line, Look.MUTED); line += 10; }
        if (c.fraction() >= 0) Look.bar(g, tx, y + h - 7, tw, 3, c.fraction(), Look.alpha(accent, 1));
    }

    /** Draws a module at (x, y) in screen units, blown up or shrunk by its own size on top of the global one. */
    static void drawScaled(GuiGraphicsExtractor g, Font font, Content c, int x, int y, int w, int h, double size, boolean highlight) {
        g.pose().pushMatrix(); g.pose().scale((float) size, (float) size);
        draw(g, font, c, Math.round((float) (x / size)), Math.round((float) (y / size)), w, h, highlight);
        g.pose().popMatrix();
    }

    /** Draws every enabled module at its saved position. */
    static void drawAll(GuiGraphicsExtractor g, Minecraft mc) {
        Preferences p = Companion.preferences;
        g.pose().pushMatrix(); g.pose().scale(p.scale, p.scale);
        int sw = (int) (g.guiWidth() / p.scale), sh = (int) (g.guiHeight() / p.scale);
        for (Preferences.Widget widget : p.widgets) {
            if (!widget.enabled) continue;
            Content c = content(widget.id);
            if (c == null) continue;
            int w = width(mc.font, c), h = height(c);
            double bw = w * widget.scale, bh = h * widget.scale;
            drawScaled(g, mc.font, c, (int) (widget.x * Math.max(0, sw - bw)), (int) (widget.y * Math.max(0, sh - bh)), w, h, widget.scale, false);
        }
        g.pose().popMatrix();
    }
}
