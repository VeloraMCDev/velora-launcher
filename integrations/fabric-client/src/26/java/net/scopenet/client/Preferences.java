package net.scopenet.client;

import com.google.gson.*;
import net.fabricmc.loader.api.FabricLoader;
import java.nio.file.*;
import java.util.*;

/** The launcher and the in-game HUD manager both edit this file. No credentials, server tokens or executable theme code. */
public final class Preferences {
    public static final List<String> WIDGETS = List.of("level", "balance", "guild", "claim", "quests", "clock");
    public boolean enabled = true, notifications = true, reduceMotion = false, claimBorders = false;
    public float opacity = .88f, scale = 1;
    public String brand = "Velora", accent = "#d4ae65";
    public List<Widget> widgets = defaults();
    public static final class Widget {
        public String id; public boolean enabled = true; public double x, y, scale = 1;
        public Widget(String id, double x, double y) { this.id=id; this.x=x; this.y=y; }
    }
    public static List<Widget> defaults() {
        List<Widget> list = new ArrayList<>(List.of(new Widget("level",.02,.04),new Widget("balance",.98,.04),new Widget("guild",.02,.9),new Widget("claim",.5,.04),new Widget("quests",.98,.3)));
        Widget clock = new Widget("clock",.98,.9); clock.enabled = false; list.add(clock);
        return list;
    }
    static Path file() { return FabricLoader.getInstance().getConfigDir().resolve("scopenet-companion.json"); }
    public static Preferences load() {
        try {
            Preferences c = new Gson().fromJson(Files.readString(file()), Preferences.class);
            if (c == null) return new Preferences();
            c.scale = Float.isFinite(c.scale) ? Math.clamp(c.scale,.5f,2f) : 1;
            c.opacity = Float.isFinite(c.opacity) ? Math.clamp(c.opacity,.2f,1f) : .88f;
            if (c.widgets == null) c.widgets = defaults();
            List<Widget> kept = new ArrayList<>();
            for (Widget w : c.widgets) {
                if (w == null || w.id == null || !WIDGETS.contains(w.id) || kept.stream().anyMatch(k -> k.id.equals(w.id))) continue;
                w.x = Double.isFinite(w.x) ? Math.clamp(w.x,0,1) : 0; w.y = Double.isFinite(w.y) ? Math.clamp(w.y,0,1) : 0;
                // Files written before per-module sizes have no scale (Gson then leaves 0).
                w.scale = Double.isFinite(w.scale) && w.scale > 0 ? Math.clamp(w.scale,.5,2.5) : 1;
                kept.add(w);
            }
            // Every module is always manageable in game, even one the file never mentioned.
            for (Widget d : defaults()) if (kept.stream().noneMatch(k -> k.id.equals(d.id))) { d.enabled = false; kept.add(d); }
            c.widgets = kept;
            if (c.brand == null || c.brand.isBlank()) c.brand="Velora";
            c.brand=c.brand.substring(0,Math.min(48,c.brand.length()));
            return c;
        } catch (Exception e) { return new Preferences(); }
    }
    /** Writes only the settings the game manages; anything else the launcher keeps in the file is left exactly as it was. */
    public void save() {
        try {
            Path p = file();
            JsonObject root = new JsonObject();
            try { JsonElement old = JsonParser.parseString(Files.readString(p)); if (old.isJsonObject()) root = old.getAsJsonObject(); } catch (Exception ignored) {}
            root.addProperty("scale", scale); root.addProperty("opacity", opacity);
            root.addProperty("notifications", notifications); root.addProperty("claimBorders", claimBorders); root.addProperty("reduceMotion", reduceMotion);
            root.add("widgets", new Gson().toJsonTree(widgets));
            Path tmp = p.resolveSibling(p.getFileName() + ".tmp");
            Files.writeString(tmp, new GsonBuilder().setPrettyPrinting().create().toJson(root));
            Files.move(tmp, p, StandardCopyOption.REPLACE_EXISTING);
        } catch (Exception ignored) {}
    }
    public Widget widget(String id) { for (Widget w : widgets) if (w.id.equals(id)) return w; return null; }
    public int accentColor() { try { return 0xff000000 | Integer.parseInt(accent.replace("#",""),16) & 0xffffff; } catch (Exception e) { return 0xffd4ae65; } }
}
