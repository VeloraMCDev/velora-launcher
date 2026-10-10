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
    public final CoreMap map = new CoreMap();
    private String panel = "", requestId = "";
    private String[] fragments;
    private long requestedAt, nextMap;
    private long lastSent;
    private final java.util.Map<String, Pending> pending = new java.util.HashMap<>();
    private static final class Pending {
        String[] pieces;
        long at=System.currentTimeMillis();
        JsonObject queued;
        final java.util.function.Consumer<com.google.gson.JsonElement> ok;
        final java.util.function.Consumer<String> fail;
        Pending(java.util.function.Consumer<com.google.gson.JsonElement> ok,java.util.function.Consumer<String> fail) {this.ok=ok;this.fail=fail;}
    }
    public void request(String operation, JsonObject args, java.util.function.Consumer<com.google.gson.JsonElement> ok, java.util.function.Consumer<String> fail) {
        if (!state.connected || !ClientPlayNetworking.canSend(TO_SERVER)) {fail.accept("Velora Core server connection is unavailable.");return;}
        if (!pending.isEmpty()) {fail.accept("Wait for the current request.");return;}
        String id=java.util.UUID.randomUUID().toString(); Pending p=new Pending(ok,fail);pending.put(id,p); nextMap=System.currentTimeMillis()+15_000;
        JsonObject request=new JsonObject();request.addProperty("t","request");request.addProperty("id",id);request.addProperty("operation",operation);request.add("args",args);
        p.queued=request;
    }
    private void handleRequestPacket(String json) {
        try {
            JsonObject m=com.google.gson.JsonParser.parseString(json).getAsJsonObject();
            if (!m.has("t") || !m.get("t").getAsString().equals("response")) return;
            String id=m.get("id").getAsString();Pending p=pending.get(id);if(p==null)return;
            int count=m.get("parts").getAsInt(),at=m.get("part").getAsInt();String piece=m.get("json").getAsString();
            if(count<1||count>120||at<0||at>=count||piece.length()>4000)throw new IllegalArgumentException();
            if(p.pieces==null)p.pieces=new String[count];if(p.pieces.length!=count)throw new IllegalArgumentException();p.pieces[at]=piece;
            for(String part:p.pieces)if(part==null)return;
            JsonObject response=com.google.gson.JsonParser.parseString(String.join("",p.pieces)).getAsJsonObject();pending.remove(id);
            if(response.get("ok").getAsBoolean())p.ok.accept(response.get("data"));else p.fail.accept(response.get("error").getAsString());
        } catch(RuntimeException malformed) { /* A malformed response expires rather than acting on partial data. */ }
    }

    Link(ClientState state, ClientConfig config) { this.state = state; this.config = config; }

    void register() {
        ClientPlayNetworking.registerGlobalReceiver(TO_CLIENT, (client, handler, buf, sender) -> {
            String json = buf.readUtf(32767 * 3);
            client.execute(() -> {
                handleMapPacket(json);
                handleRequestPacket(json);
                toast(state.handle(json));
                String open = state.pendingOpen;
                state.pendingOpen = null;
                // A right-clicked market point opens its window, unless another screen is already up.
                if (open != null && config.enabled && client.screen == null) {
                    if (open.equals("market") && config.module("economy") && state.module("economy")) client.setScreen(new net.scopenet.client.screen.MarketScreen());
                    if(open.equals("physical_shop")&&config.module("economy")&&state.module("economy"))client.setScreen(new net.scopenet.client.screen.PhysicalShopScreen());
                }
            });
        });
        ClientPlayConnectionEvents.JOIN.register((handler, sender, client) -> send("hello"));
        ClientPlayConnectionEvents.DISCONNECT.register((handler, client) -> { pending.clear(); state.reset(); map.reset(); panel = ""; requestId = ""; fragments = null; nextMap = 0; });
    }

    public void send(String type) {
        JsonObject m = new JsonObject();
        m.addProperty("t", type);
        if (type.equals("hello")) { m.addProperty("mod", "0.1.0"); m.addProperty("protocol", 2); }
        FriendlyByteBuf buf = PacketByteBufs.create();
        buf.writeUtf(m.toString(), 32767 * 3);
        try { ClientPlayNetworking.send(TO_SERVER, buf); } catch (RuntimeException ignored) { /* not in a world */ }
        lastSent=System.currentTimeMillis();
    }

    public void mapTick() {
        long now = System.currentTimeMillis();
        if(!requestId.isEmpty()&&now-requestedAt>10_000){requestId="";fragments=null;}
        if(!pending.isEmpty()&&requestId.isEmpty()&&now-lastSent>=450){
            Pending p=pending.values().iterator().next();
            if(p.queued!=null){FriendlyByteBuf buf=PacketByteBufs.create();buf.writeUtf(p.queued.toString());ClientPlayNetworking.send(TO_SERVER,buf);p.queued=null;p.at=now;lastSent=now;}
        }
        for (var entry:new java.util.ArrayList<>(pending.entrySet())) if(now-entry.getValue().at>15_000) {pending.remove(entry.getKey());entry.getValue().fail.accept("Request timed out. Refresh before retrying.");}
        if (!pending.isEmpty()) return;
        if (!state.connected || !state.module("map") || !config.module("map") || panel.isEmpty() || now < nextMap) return;
        nextMap = now + 15_000; requestId = java.util.UUID.randomUUID().toString(); fragments = null; requestedAt = now;
        JsonObject request = new JsonObject(); request.addProperty("t", "request"); request.addProperty("id", requestId);
        request.addProperty("operation", "map"); request.add("args", new JsonObject());
        FriendlyByteBuf buf = PacketByteBufs.create(); buf.writeUtf(request.toString());
        if (ClientPlayNetworking.canSend(TO_SERVER)) {ClientPlayNetworking.send(TO_SERVER, buf);lastSent=now;}
    }
    private void handleMapPacket(String json) {
        try {
            JsonObject message = com.google.gson.JsonParser.parseString(json).getAsJsonObject();
            String type = message.has("t") ? message.get("t").getAsString() : "";
            if (type.equals("hello") && message.has("panel")) { panel = message.get("panel").getAsString(); nextMap = System.currentTimeMillis() + 1_000; }
            if (!type.equals("response") || requestId.isEmpty() || !message.get("id").getAsString().equals(requestId) || System.currentTimeMillis() - requestedAt > 10_000) return;
            int parts = message.get("parts").getAsInt(), part = message.get("part").getAsInt();
            String fragment = message.get("json").getAsString();
            if (parts < 1 || parts > 120 || part < 0 || part >= parts || fragment.length() > 4_000) { fragments = null; requestId = ""; return; }
            if (fragments == null) fragments = new String[parts];
            if (fragments.length != parts) { fragments = null; requestId = ""; return; }
            fragments[part] = fragment;
            for (String piece : fragments) if (piece == null) return;
            JsonObject response = com.google.gson.JsonParser.parseString(String.join("", fragments)).getAsJsonObject();
            fragments = null; requestId = "";
            if (response.has("ok") && response.get("ok").getAsBoolean() && response.get("data").isJsonObject()) map.receive(panel, response.getAsJsonObject("data"));
            else map.message = "Map unavailable";
        } catch (RuntimeException ignored) { fragments = null; }
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
