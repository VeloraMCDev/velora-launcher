package net.velora.client.screen;
import com.google.gson.*;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;
import net.velora.client.*;
import java.util.UUID;

/** Reviewed Darknet purchases use the same catalog, receipts and vault delivery as launcher/web. */
public final class DarknetScreen extends Screen {
    private JsonArray products;
    private JsonObject pendingPurchase;
    private String message="Loading Darknet...";
    private boolean busy;
    private int page,left,top;
    private double balance;
    public DarknetScreen(){super(Component.literal("Darknet"));}
    @Override protected void init(){
        left=(width-320)/2;top=(height-222)/2;
        add("Back",left+10,top+195,72,18,()->minecraft.setScreen(new MenuScreen()));
        add("Refresh",left+238,top+195,72,18,this::load);
        if(pendingPurchase!=null)add("Retry purchase",left+104,top+195,122,18,this::buy);
        else if(products!=null){
            for(int i=0;i<5&&page*5+i<products.size();i++){final JsonObject product=products.get(page*5+i).getAsJsonObject();
                add("Buy "+Ui.money("$",product.get("price_cents").getAsLong()/100.0),left+205,top+43+i*27,105,20,()->review(product));
            }
            add("Previous",left+10,top+174,80,16,()->{page=Math.max(0,page-1);rebuildWidgets();});
            add("Next",left+230,top+174,80,16,()->{if((page+1)*5<products.size())page++;rebuildWidgets();});
        }
        if(products==null&&!busy)load();
    }
    private void add(String text,int x,int y,int w,int h,Runnable run){Button b=addRenderableWidget(Button.builder(Component.literal(text),button->{if(!busy)run.run();}).bounds(x,y,w,h).build());b.active=!busy;}
    private void load(){busy=true;VeloraClient.link().request("darknet",new JsonObject(),data->{busy=false;JsonObject result=data.getAsJsonObject();products=result.getAsJsonArray("products");balance=result.has("balance")&&!result.get("balance").isJsonNull()?result.get("balance").getAsDouble():VeloraClient.state().balance;message="Purchases are delivered to your vault.";rebuildWidgets();},error->{busy=false;if(products==null)products=new JsonArray();message=error;rebuildWidgets();});}
    private void review(JsonObject product){
        minecraft.setScreen(new net.minecraft.client.gui.screens.ConfirmScreen(yes->{minecraft.setScreen(this);if(yes){pendingPurchase=new JsonObject();pendingPurchase.addProperty("operation_id",UUID.randomUUID().toString());pendingPurchase.add("product_id",product.get("id"));pendingPurchase.add("expected_price_cents",product.get("price_cents"));buy();}},Component.literal("Confirm Darknet purchase"),Component.literal(product.get("item_name").getAsString()+" for "+Ui.money("$",product.get("price_cents").getAsLong()/100.0)+" in-game dollars.")));
    }
    private void buy(){if(pendingPurchase==null||busy)return;busy=true;message="Waiting for server...";rebuildWidgets();
        VeloraClient.link().request("darknet_buy",pendingPurchase,data->{busy=false;JsonObject result=data.getAsJsonObject();balance=result.get("balance").getAsDouble();pendingPurchase=null;message="Purchased. Delivery is queued for your vault.";products=null;load();},error->{busy=false;message=error+" Retry keeps the purchase identity.";rebuildWidgets();});
    }
    @Override public void render(GuiGraphics g,int mx,int my,float delta){
        renderBackground(g);Ui.window(g,left,top,320,222);g.drawString(font,"Darknet",left+10,top+10,Ui.ACCENT,true);g.drawString(font,Ui.money("$",balance),left+207,top+10,Ui.GOLD,true);
        if(products!=null)for(int i=0;i<5&&page*5+i<products.size();i++){JsonObject p=products.get(page*5+i).getAsJsonObject();g.drawString(font,font.plainSubstrByWidth(p.get("item_name").getAsString(),185),left+10,top+44+i*27,Ui.TEXT,false);g.drawString(font,"Quantity: "+p.get("amount").getAsInt(),left+10,top+55+i*27,Ui.MUTED,false);}
        g.drawString(font,font.plainSubstrByWidth(message,298),left+10,top+30,Ui.MUTED,false);super.render(g,mx,my,delta);
    }
    @Override public boolean isPauseScreen(){return false;}
}
