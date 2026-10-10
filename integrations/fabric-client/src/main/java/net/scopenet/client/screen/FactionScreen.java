package net.scopenet.client.screen;
import com.google.gson.*;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.components.EditBox;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;
import net.scopenet.client.*;

/** Faction overview and bank actions. Permission checks and prices stay on the server. */
public final class FactionScreen extends Screen {
    private JsonObject faction,bank;
    private String message="Loading faction...";
    private boolean busy;
    private EditBox amount;
    private int left,top;
    public FactionScreen(){super(Component.literal("Faction"));}
    @Override protected void init(){
        left=(width-340)/2;top=(height-230)/2;
        add("Back",left+10,top+205,70,18,()->minecraft.setScreen(new MenuScreen()));
        add("Refresh",left+260,top+205,70,18,this::load);
        add("Claim map",left+10,top+170,100,20,()->minecraft.setScreen(new MapScreen()));
        add("Faction vault",left+120,top+170,105,20,()->{Link.command("faction vault 1");onClose();});
        add("Management",left+235,top+170,95,20,()->{Link.command("faction");onClose();});
        amount=new EditBox(font,left+10,top+140,86,18,Component.literal("Bank amount"));amount.setValue("50");amount.setMaxLength(12);addRenderableWidget(amount);
        add("Deposit",left+105,top+140,100,18,()->transfer("deposit"));add("Withdraw",left+215,top+140,115,18,()->transfer("withdraw"));
        if(faction==null&&!busy)load();
    }
    private void add(String text,int x,int y,int w,int h,Runnable run){Button b=addRenderableWidget(Button.builder(Component.literal(text),button->{if(!busy)run.run();}).bounds(x,y,w,h).build());b.active=!busy;}
    private void transfer(String action){try{var value=new java.math.BigDecimal(amount.getValue());if(value.signum()<=0||value.scale()>2||value.compareTo(new java.math.BigDecimal("1000000000"))>0)throw new IllegalArgumentException();Link.command("faction bank "+action+" "+value.toPlainString());onClose();}catch(RuntimeException invalid){message="Enter a positive dollar amount with at most two decimals.";}}
    private void load(){busy=true;ScopenetClient.link().request("guild",new JsonObject(),data->{
        busy=false;if(data==null||data.isJsonNull()){message="Join or create a faction with /faction.";faction=new JsonObject();rebuildWidgets();return;}faction=data.getAsJsonObject();
        busy=true;ScopenetClient.link().request("guild_bank",new JsonObject(),result->{busy=false;bank=result.getAsJsonObject();message="Bank actions follow your faction role permissions.";rebuildWidgets();},error->{busy=false;if(faction==null)faction=new JsonObject();message=error;rebuildWidgets();});
    },error->{busy=false;if(faction==null)faction=new JsonObject();message=error;rebuildWidgets();});}
    @Override public void render(GuiGraphics g,int mx,int my,float delta){
        renderBackground(g);Ui.window(g,left,top,340,230);g.drawString(font,"Faction",left+10,top+10,Ui.ACCENT,true);
        if(faction!=null&&faction.has("name")){
            g.drawString(font,"["+faction.get("tag").getAsString()+"] "+faction.get("name").getAsString(),left+10,top+30,Ui.TEXT,true);
            if(faction.has("members")){JsonArray roster=faction.getAsJsonArray("members");g.drawString(font,"Roster: "+roster.size()+" members",left+10,top+47,Ui.MUTED,false);for(int i=0;i<Math.min(4,roster.size());i++){JsonObject member=roster.get(i).getAsJsonObject();String name=member.has("name")?member.get("name").getAsString():member.has("username")?member.get("username").getAsString():member.get("uuid").getAsString();g.drawString(font,font.plainSubstrByWidth(name,300),left+10,top+62+i*12,Ui.TEXT,false);}}
            if(bank!=null&&bank.has("balance"))g.drawString(font,"Bank "+Ui.money("$",bank.get("balance").getAsDouble()),left+210,top+47,Ui.GOLD,true);
        }
        g.drawString(font,font.plainSubstrByWidth(message,320),left+10,top+121,Ui.MUTED,false);super.render(g,mx,my,delta);
    }
    @Override public boolean isPauseScreen(){return false;}
}
