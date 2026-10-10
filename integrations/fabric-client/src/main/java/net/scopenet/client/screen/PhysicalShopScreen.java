package net.scopenet.client.screen;
import com.google.gson.*;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;
import net.scopenet.client.*;

/** The server selects the pedestal and derives the buyer identity; no submitted item data is accepted. */
public final class PhysicalShopScreen extends Screen {
    private JsonObject offer;
    private boolean busy,finished;
    private String message="Loading shop...";
    private int left,top;
    public PhysicalShopScreen(){super(Component.literal("Pedestal shop"));}
    @Override protected void init(){
        left=(width-280)/2;top=(height-148)/2;
        addRenderableWidget(Button.builder(Component.literal("Close"),b->onClose()).bounds(left+10,top+113,110,20).build());
        Button buy=addRenderableWidget(Button.builder(Component.literal("Confirm purchase"),b->request(true)).bounds(left+132,top+113,138,20).build());buy.active=offer!=null&&!busy&&!finished;
        if(offer==null&&!busy&&!finished)request(false);
    }
    private void request(boolean buy){busy=true;JsonObject args=new JsonObject();args.addProperty("action",buy?"buy":"view");
        ScopenetClient.link().request("physical_shop",args,data->{busy=false;if(buy){finished=true;message="Purchased. Items are queued for your vault.";}else{offer=data.getAsJsonObject();message="Review the price, then confirm.";}rebuildWidgets();},error->{busy=false;finished=true;message=error;rebuildWidgets();});
    }
    @Override public void render(GuiGraphics g,int mx,int my,float delta){renderBackground(g);Ui.window(g,left,top,280,148);g.drawString(font,"Pedestal shop",left+10,top+10,Ui.ACCENT,true);
        if(offer!=null){g.drawString(font,font.plainSubstrByWidth(offer.get("quantity").getAsInt()+"x "+offer.get("item_name").getAsString(),260),left+10,top+37,Ui.TEXT,true);g.drawString(font,Ui.money("$",offer.get("price_cents").getAsLong()/100.0),left+10,top+55,Ui.GOLD,true);g.drawString(font,"No shop fee. Delivery to your vault.",left+10,top+74,Ui.MUTED,false);}
        g.drawString(font,font.plainSubstrByWidth(message,260),left+10,top+95,Ui.MUTED,false);super.render(g,mx,my,delta);}
    @Override public boolean isPauseScreen(){return false;}
}
