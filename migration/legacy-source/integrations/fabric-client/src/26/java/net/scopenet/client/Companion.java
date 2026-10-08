package net.scopenet.client;

import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.loader.api.FabricLoader;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.fabric.api.client.keymapping.v1.KeyMappingHelper;
import net.fabricmc.fabric.api.client.rendering.v1.hud.HudElementRegistry;
import net.minecraft.client.*;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.resources.Identifier;
import com.mojang.blaze3d.platform.InputConstants;
import com.google.gson.*;
import java.util.*;
import net.fabricmc.fabric.api.client.screen.v1.ScreenEvents;
import net.fabricmc.fabric.api.client.screen.v1.Screens;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.PauseScreen;
import net.minecraft.client.gui.screens.TitleScreen;
import net.minecraft.network.chat.Component;
import net.minecraft.gizmos.Gizmos;
import net.minecraft.gizmos.GizmoStyle;
import net.minecraft.world.phys.AABB;

public final class Companion implements ClientModInitializer {
    public static final Model model=new Model();
    public static Preferences preferences=Preferences.load();
    public static final Connection connection=new Connection();
    public static String VERSION="";
    public static JsonObject pinnedQuest;
    private final Map<String,KeyMapping> bindings=new LinkedHashMap<>();
    private long reloadAt, questRefreshAt;
    @Override public void onInitializeClient(){
        VERSION=FabricLoader.getInstance().getModContainer("scopenet_client").orElseThrow().getMetadata().getVersion().getFriendlyString();
        connection.register();
        ScreenEvents.AFTER_INIT.register((mc,screen,w,h)->{
            if(preferences.enabled&&screen instanceof PauseScreen){Screens.getWidgets(screen).add(Button.builder(Component.literal("SCOPENET"),b->open("hub")).bounds(8,h-28,98,20).build());}
            if(screen instanceof TitleScreen)ScreenEvents.afterForeground(screen).register((s,g,x,y,d)->{
                if(!preferences.enabled)return;
                g.fill(6,6,23,23,0xff171b20);g.outline(6,6,17,17,preferences.accentColor());g.text(mc.font,"S",12,10,preferences.accentColor());
                g.text(mc.font,preferences.brand+" Companion",29,11,preferences.accentColor());
            });
        });
        KeyMapping.Category category=KeyMapping.Category.register(Identifier.parse("scopenet:companion"));
        for(String page:List.of("hub","vaults","map","guild","market","orders","contracts","quests","travel","friends","hud")){
            int key=page.equals("hub")?InputConstants.KEY_K:InputConstants.UNKNOWN.getValue();
            bindings.put(page,KeyMappingHelper.registerKeyMapping(new KeyMapping("key.scopenet."+page,InputConstants.Type.KEYBOARD,key,category)));
        }
        HudElementRegistry.addLast(Identifier.parse("scopenet:widgets"),(g,d)->hud(g));
        ClientTickEvents.END_CLIENT_TICK.register(mc->{
            connection.tick();
            if(System.currentTimeMillis()>reloadAt&&!(mc.gui.screen() instanceof HudEditorScreen)){preferences=Preferences.load();reloadAt=System.currentTimeMillis()+2000;}
            bindings.forEach((page,key)->{while(key.consumeClick())if(mc.player!=null&&mc.gui.screen()==null&&preferences.enabled)open(page);});
            if(!model.connected){pinnedQuest=null;questRefreshAt=0;}
            if(preferences.enabled&&pinnedQuest!=null&&mc.gui.screen()==null&&connection.ready()&&System.currentTimeMillis()>=questRefreshAt){
                questRefreshAt=System.currentTimeMillis()+30000;
                String id=Model.text(Model.object(pinnedQuest,"quest"),"id");
                connection.request("quests",result->{
                    if(pinnedQuest==null||!Model.text(Model.object(pinnedQuest,"quest"),"id").equals(id)||!result.isJsonArray())return;
                    pinnedQuest=null;
                    for(JsonElement row:result.getAsJsonArray())if(row.isJsonObject()&&Model.text(Model.object(row.getAsJsonObject(),"quest"),"id").equals(id)){pinnedQuest=row.getAsJsonObject();break;}
                },error->{});
            }
            claimBorders(mc);
        });
    }
    private static void claimBorders(Minecraft mc){
        if(!preferences.enabled||!preferences.claimBorders||!model.connected||mc.player==null||mc.level==null||mc.gui.hud.isHidden())return;
        if(!Model.text(model.claims,"dim").equals(mc.level.dimension().identifier().toString()))return;
        int r=(int)Model.number(model.claims,"r"),side=2*r+1;if(r<0||r>4)return;
        String cells=Model.text(model.claims,"cells");int cx=(int)Model.number(model.claims,"cx"),cz=(int)Model.number(model.claims,"cz");
        try(var collection=mc.collectPerTickGizmos()){
            for(int z=0;z<side;z++)for(int x=0;x<side;x++){int index=z*side+x;if(index>=cells.length()||cells.charAt(index)=='0')continue;
                double minX=(cx+x-r)*16.0,minZ=(cz+z-r)*16.0,y=mc.player.getY();int color=cells.charAt(index)=='1'?0xff80bc85:0xffd98572;
                Gizmos.cuboid(new AABB(minX,y-.05,minZ,minX+16,y+2,minZ+16),GizmoStyle.stroke(color,1.5f));
            }
        }
    }
    public static void open(String page){Minecraft.getInstance().gui.setScreen(page.equals("hud")?new HudEditorScreen():new CompanionScreen(page));}
    public static void openHud(){Minecraft.getInstance().gui.setScreen(new HudEditorScreen());}
    public static void panel(GuiGraphicsExtractor g,int x,int y,int w,int h){Look.panel(g,x,y,w,h,preferences.opacity);}
    public static void hud(GuiGraphicsExtractor g){
        Minecraft mc=Minecraft.getInstance();if(!preferences.enabled||!model.connected||mc.player==null||mc.gui.hud.isHidden()||mc.gui.screen()!=null)return;
        Hud.drawAll(g,mc);
        if(preferences.notifications&&model.toastUntil>System.currentTimeMillis()){
            String text=mc.font.plainSubstrByWidth(Words.sentence(model.toast),250);int w=mc.font.width(text)+22;
            Look.tooltip(g,(g.guiWidth()-w)/2,g.guiHeight()-72,w,24,preferences.accentColor(),preferences.opacity);
            g.centeredText(mc.font,text,g.guiWidth()/2,g.guiHeight()-64,Look.TEXT);
        }
    }
}
