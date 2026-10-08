package net.scopenet.core.map;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;

import java.util.List;
import java.util.Locale;

/**
 * The claims and pins as the JSON the panel's map viewer draws ({@code {"claims":[...],"pins":[...]}}). Details are plain text
 * lines, never HTML: the viewer sets them as text, so nothing a player types can inject markup.
 */
public final class MapOverlay {
    private MapOverlay() {}

    public static JsonObject toJson(MapData data) {
        JsonArray claims = new JsonArray();
        for (MapData.ClaimRegion c : data.claims()) {
            JsonObject o = new JsonObject();
            o.addProperty("id", c.id());
            o.addProperty("guild_id", c.guildId());
            o.addProperty("name", c.name());
            o.addProperty("tag", c.tag());
            o.addProperty("icon", c.icon());
            o.addProperty("color", String.format(Locale.ROOT, "#%06x", c.color() & 0xFFFFFF));
            o.addProperty("dimension", c.dimension());
            o.addProperty("chunks", c.chunks());
            o.addProperty("label_x", c.labelX());
            o.addProperty("label_z", c.labelZ());
            o.add("outer", ring(c.outer()));
            JsonArray holes = new JsonArray();
            for (ChunkRegions.Ring hole : c.holes()) holes.add(ring(hole));
            o.add("holes", holes);
            o.add("lines", lines(c.detail()));
            claims.add(o);
        }
        JsonArray pins = new JsonArray();
        for (MapData.Pin p : data.pins()) {
            JsonObject o = new JsonObject();
            o.addProperty("id", p.id());
            o.addProperty("kind", p.kind());
            o.addProperty("label", p.label());
            o.addProperty("dimension", p.dimension());
            o.addProperty("x", p.x());
            o.addProperty("y", p.y());
            o.addProperty("z", p.z());
            o.add("lines", lines(p.detail()));
            pins.add(o);
        }
        JsonObject root = new JsonObject();
        root.add("claims", claims);
        root.add("pins", pins);
        return root;
    }

    private static JsonArray ring(ChunkRegions.Ring ring) {
        JsonArray out = new JsonArray();
        for (ChunkRegions.Point p : ring.points()) {
            JsonArray pair = new JsonArray();
            pair.add(p.x());
            pair.add(p.z());
            out.add(pair);
        }
        return out;
    }

    /** Turns the model's escaped HTML into plain lines (the markup there is ours; the text inside is already escaped). */
    static JsonArray lines(String html) {
        String text = html == null ? "" : html
                .replaceAll("(?i)<br\\s*/?>|</div>|</li>|</ul>|<ul[^>]*>", "\n")
                .replaceAll("<[^>]*>", "");
        text = text.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#39;", "'")
                .replace("&middot;", "·").replace("&amp;", "&");
        JsonArray out = new JsonArray();
        for (String line : text.split("\n")) {
            String t = line.trim();
            if (!t.isEmpty()) out.add(t.length() > 200 ? t.substring(0, 200) : t);
        }
        return out;
    }
}
