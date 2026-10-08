package net.scopenet.client;

import com.google.gson.*;
import net.fabricmc.fabric.api.client.networking.v1.*;
import net.fabricmc.fabric.api.networking.v1.PayloadTypeRegistry;
import net.minecraft.resources.Identifier;
import net.minecraft.network.RegistryFriendlyByteBuf;
import net.minecraft.network.codec.StreamCodec;
import net.minecraft.network.protocol.common.custom.CustomPacketPayload;
import net.minecraft.client.Minecraft;
import java.util.*;
import java.util.function.Consumer;

public final class Connection {
    public record Incoming(String json) implements CustomPacketPayload {
        static final Type<Incoming> TYPE = new Type<>(Identifier.parse("scopenet:s2c"));
        static final StreamCodec<RegistryFriendlyByteBuf,Incoming> CODEC = StreamCodec.of((b,p)->b.writeUtf(p.json,32767),b->new Incoming(b.readUtf(32767)));
        public Type<? extends CustomPacketPayload> type(){return TYPE;}
    }
    public record Outgoing(String json) implements CustomPacketPayload {
        static final Type<Outgoing> TYPE = new Type<>(Identifier.parse("scopenet:c2s"));
        static final StreamCodec<RegistryFriendlyByteBuf,Outgoing> CODEC = StreamCodec.of((b,p)->b.writeUtf(p.json,32767),b->new Outgoing(b.readUtf(32767)));
        public Type<? extends CustomPacketPayload> type(){return TYPE;}
    }
    private static final class Pending {
        final Consumer<JsonElement> ok; final Consumer<String> fail; final long deadline=System.currentTimeMillis()+12000;
        String[] parts;
        Pending(Consumer<JsonElement> ok,Consumer<String> fail){this.ok=ok;this.fail=fail;}
    }
    private final Map<String,Pending> pending=new HashMap<>();
    private long nextHello; private int attempts; private long lastSend;
    public boolean ready(){return Companion.model.modern&&pending.isEmpty()&&System.currentTimeMillis()-lastSend>=450;}
    public void register() {
        PayloadTypeRegistry.clientboundPlay().register(Incoming.TYPE,Incoming.CODEC);
        PayloadTypeRegistry.serverboundPlay().register(Outgoing.TYPE,Outgoing.CODEC);
        ClientPlayNetworking.registerGlobalReceiver(Incoming.TYPE,(p,c)->c.client().execute(()->receive(p.json)));
        ClientPlayConnectionEvents.JOIN.register((h,s,c)->{reset();nextHello=System.currentTimeMillis()+1000;});
        ClientPlayConnectionEvents.DISCONNECT.register((h,c)->reset());
    }
    private void reset(){pending.clear();Companion.model.reset();attempts=0;lastSend=0;nextHello=0;}
    public void tick() {
        long now=System.currentTimeMillis();
        if(!Companion.model.connected && nextHello>0 && now>=nextHello && attempts<3){
            JsonObject hello=new JsonObject();hello.addProperty("t","hello");hello.addProperty("protocol",2);hello.addProperty("mod",Companion.VERSION);
            send(hello);attempts++;nextHello=now+2500;
        }
        for(String id:new ArrayList<>(pending.keySet())){Pending p=pending.get(id);if(now>p.deadline){pending.remove(id);p.fail.accept("Request timed out. Refresh to check its result before repeating an action.");}}
    }
    public void sendType(String type){JsonObject m=new JsonObject();m.addProperty("t",type);send(m);}
    private boolean send(JsonObject m){try{if(Minecraft.getInstance().getConnection()==null)return false;ClientPlayNetworking.send(new Outgoing(m.toString()));lastSend=System.currentTimeMillis();return true;}catch(RuntimeException e){return false;}}
    public void request(String op,JsonObject args,Consumer<JsonElement> ok,Consumer<String> fail) {
        if(!Companion.model.modern){fail.accept("Update the Velora Paper plugin to use this screen.");return;}
        if(!pending.isEmpty() || System.currentTimeMillis()-lastSend<450){fail.accept("Please wait a moment, then try again.");return;}
        String id=UUID.randomUUID().toString();JsonObject m=new JsonObject();m.addProperty("t","request");m.addProperty("id",id);m.addProperty("operation",op);m.add("args",args);
        pending.put(id,new Pending(ok,fail));if(!send(m)){pending.remove(id);fail.accept("The server connection is unavailable.");}
    }
    public void request(String op,Consumer<JsonElement> ok,Consumer<String> fail){request(op,new JsonObject(),ok,fail);}
    private void receive(String json) {
        try {
            JsonObject m=JsonParser.parseString(json).getAsJsonObject();String type=Model.text(m,"t");
            if(type.equals("response")){
                String id=Model.text(m,"id");Pending p=pending.get(id);if(p==null)return;
                int count=(int)Model.number(m,"parts"),part=(int)Model.number(m,"part");String text=Model.text(m,"json");
                if(count<1||count>125||part<0||part>=count||text.length()>4000)throw new IllegalArgumentException();
                if(p.parts==null)p.parts=new String[count];if(p.parts.length!=count)throw new IllegalArgumentException();p.parts[part]=text;
                if(Arrays.stream(p.parts).anyMatch(Objects::isNull))return;
                pending.remove(id);JsonObject response=JsonParser.parseString(String.join("",p.parts)).getAsJsonObject();
                if(Model.flag(response,"ok"))p.ok.accept(response.get("data"));else p.fail.accept(Model.text(response,"error"));
            }else if(type.equals("dialogue")){
                Minecraft mc=Minecraft.getInstance();
                if(Companion.preferences.enabled&&mc.gui.screen()==null)mc.gui.setScreen(new DialogueScreen(Model.text(m,"title"),Model.array(m,"lines")));
            }else if(type.equals("open")){
                if(Companion.preferences.enabled && Minecraft.getInstance().gui.screen()==null)Companion.open(Model.text(m,"screen"));
                else if(Model.text(m,"screen").equals("market"))command("market");
            }else Companion.model.handle(m);
        }catch(RuntimeException e){Companion.model.error="Invalid companion message from server.";}
    }
    public static void command(String command){Minecraft mc=Minecraft.getInstance();if(mc.player!=null)mc.player.connection.sendCommand(command);}
}
