package net.velora.client.screen;

import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;
import net.velora.client.*;

/** Full map with pan, zoom, independent layer preferences and local waypoints. */
public final class MapScreen extends Screen {
    private double cx, cz, scale = 0.5;
    private int mapTop;
    private boolean editing, busy;
    private Integer selectedX, selectedZ;
    private String status="";
    private Button claim, unclaim, waypoint, removeWaypoint;
    public MapScreen() { super(Component.literal("Velora SMP map")); }
    @Override protected void init() {
        if (minecraft.player != null) { cx = minecraft.player.getX(); cz = minecraft.player.getZ(); }
        int index = 0, columns = Math.max(1, (width - 20) / 64);
        for (String layer : new String[]{"players", "claims", "spawn", "warps", "homes", "shops", "waypoints"}) {
            int buttonX = 10 + (index % columns) * 64, buttonY = 10 + (index / columns) * 23; index++;
            addRenderableWidget(Button.builder(label(layer), button -> {
                var config = VeloraClient.config(); config.mapLayers.put(layer, !Boolean.TRUE.equals(config.mapLayers.get(layer))); config.save(); button.setMessage(label(layer));
            }).bounds(buttonX, buttonY, 60, 20).build());
        }
        addRenderableWidget(Button.builder(minimapLabel(), button -> {
            var config = VeloraClient.config(); var minimap = config.widget("minimap");
            minimap.enabled = !minimap.enabled; config.save(); button.setMessage(minimapLabel());
        }).bounds(10 + (index % columns) * 64, 10 + (index / columns) * 23, 60, 20).build()); index++;
        addRenderableWidget(Button.builder(Component.literal("Size"), button -> {
            var config = VeloraClient.config(); var minimap = config.widget("minimap");
            minimap.compact = !minimap.compact; config.save();
        }).bounds(10 + (index % columns) * 64, 10 + (index / columns) * 23, 60, 20).build()); index++;
        if (VeloraClient.state().module("factions")) {
            addRenderableWidget(Button.builder(Component.literal("Claims"),button->{editing=!editing;updateButtons();})
                .bounds(10+(index%columns)*64,10+(index/columns)*23,60,20).build());index++;
        }
        mapTop = 15 + ((index + columns - 1) / columns) * 23;
        waypoint=addRenderableWidget(Button.builder(Component.literal("Add waypoint"), button -> {
            var config = VeloraClient.config(); var map = VeloraClient.link().map;
            if (config.waypoints.size() >= 256 || map.scope().endsWith("|")) return;
            config.waypoints.add(new ClientConfig.Waypoint(map.scope(), CoreMap.dimension(), "Waypoint " + (config.waypoints.size() + 1), cx, cz)); config.save();
        }).bounds(10, height - 28, 110, 20).build());
        removeWaypoint=addRenderableWidget(Button.builder(Component.literal("Remove last"), button -> {
            var config = VeloraClient.config(); String scope = VeloraClient.link().map.scope();
            for (int at = config.waypoints.size() - 1; at >= 0; at--) if (config.waypoints.get(at).scope().equals(scope)) { config.waypoints.remove(at); config.save(); break; }
        }).bounds(124, height - 28, 100, 20).build());
        claim=addRenderableWidget(Button.builder(Component.literal("Claim"),button->edit("claim")).bounds(10,height-28,85,20).build());
        unclaim=addRenderableWidget(Button.builder(Component.literal("Unclaim"),button->edit("unclaim")).bounds(100,height-28,85,20).build());
        updateButtons();
        addRenderableWidget(Button.builder(Component.literal("Close"), button -> onClose()).bounds(width - 70, height - 28, 60, 20).build());
    }
    private void updateButtons() {
        if(claim==null)return;
        claim.visible=unclaim.visible=editing;claim.active=unclaim.active=!busy&&selectedX!=null;
        waypoint.visible=removeWaypoint.visible=!editing;
    }
    private void edit(String action) {
        if(busy||selectedX==null)return;
        busy=true;updateButtons();status="Updating chunk "+selectedX+", "+selectedZ+"…";
        var args=new com.google.gson.JsonObject();args.addProperty("action",action);args.addProperty("x",selectedX);args.addProperty("z",selectedZ);
        VeloraClient.link().request("claim_edit",args,data->{busy=false;status=action.equals("claim")?"Chunk claimed":"Chunk unclaimed";VeloraClient.link().send("claims");updateButtons();},error->{busy=false;status=error;updateButtons();});
    }
    private static Component label(String layer) { return Component.literal((Boolean.TRUE.equals(VeloraClient.config().mapLayers.get(layer)) ? "✓ " : "") + layer); }
    private static Component minimapLabel() { return Component.literal(VeloraClient.config().widget("minimap").enabled ? "Mini: on" : "Mini: off"); }
    @Override public void render(GuiGraphics graphics, int mouseX, int mouseY, float delta) {
        renderBackground(graphics);
        VeloraClient.link().map.render(graphics, 8, mapTop, Math.max(1, width - 16), Math.max(1, height - mapTop - 38), cx, cz, scale, CoreMap.dimension(), true);
        if(editing) {
            int mapHeight=Math.max(1,height-mapTop-38);double centerY=mapTop+mapHeight/2.0;
            graphics.enableScissor(8,mapTop,width-8,height-38);
            if(scale>=0.25) {
                int firstX=(int)Math.floor((cx-(width-16)/2.0/scale)/16),firstZ=(int)Math.floor((cz-mapHeight/2.0/scale)/16);
                for(int i=0;i<Math.min(256,(int)((width-16)/(16*scale))+2);i++){int x=(int)Math.round(width/2.0+((firstX+i)*16-cx)*scale);graphics.fill(x,mapTop,x+1,height-38,0x447c68ad);}
                for(int i=0;i<Math.min(256,(int)(mapHeight/(16*scale))+2);i++){int y=(int)Math.round(centerY+((firstZ+i)*16-cz)*scale);graphics.fill(8,y,width-8,y+1,0x447c68ad);}
            }
            if(selectedX!=null){int x=(int)Math.round(width/2.0+(selectedX*16.0-cx)*scale),y=(int)Math.round(centerY+(selectedZ*16.0-cz)*scale),size=Math.max(1,(int)Math.ceil(16*scale));graphics.fill(x,y,x+size,y+size,0x887d55c7);}
            graphics.disableScissor();
        }
        graphics.drawString(font, status.isEmpty()?(editing?"Right-click a chunk, then choose Claim or Unclaim":"Drag to pan · Scroll to zoom · Waypoint at map center"):status, 10, height - 39, Ui.MUTED, false);
        super.render(graphics, mouseX, mouseY, delta);
    }
    @Override public boolean mouseClicked(double mouseX,double mouseY,int button) {
        if(editing&&!busy&&button==1&&mouseX>=8&&mouseX<width-8&&mouseY>mapTop&&mouseY<height-38){
            selectedX=(int)Math.floor((cx+(mouseX-width/2.0)/scale)/16);
            selectedZ=(int)Math.floor((cz+(mouseY-(mapTop+Math.max(1,height-mapTop-38)/2.0))/scale)/16);
            status="Selected chunk "+selectedX+", "+selectedZ;updateButtons();return true;
        }
        return super.mouseClicked(mouseX,mouseY,button);
    }
    @Override public boolean mouseDragged(double mouseX, double mouseY, int button, double dx, double dy) {
        if (button == 0 && mouseY > mapTop && mouseY < height - 38) { cx -= dx / scale; cz -= dy / scale; return true; }
        return super.mouseDragged(mouseX, mouseY, button, dx, dy);
    }
    @Override public boolean mouseScrolled(double mouseX, double mouseY, double amount) {
        double centerY=mapTop+Math.max(1,height-mapTop-38)/2.0;
        double worldX = cx + (mouseX - width / 2.0) / scale, worldZ = cz + (mouseY - centerY) / scale;
        scale = Math.max(1.0 / 64, Math.min(8, scale * Math.pow(1.3, amount)));
        cx = worldX - (mouseX - width / 2.0) / scale; cz = worldZ - (mouseY - centerY) / scale; return true;
    }
    @Override public boolean isPauseScreen() { return false; }
}
