package net.scopenet.client.screen;
import com.google.gson.*;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.components.EditBox;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;
import net.scopenet.client.*;
import java.util.*;

/** All wagers and outcomes are resolved by the panel; the client never rolls or credits money. */
public final class CasinoScreen extends Screen {
    private static final String[] GAMES={"slots","blackjack","wheel","plinko","mines","crash","dice","coinflip","burst","roulette"};
    private String selected="slots",option="heads",message="Loading casino...";
    private JsonObject lobby,game;
    private double balance;
    private boolean busy,needsRefresh=true;
    private EditBox bet;
    private Button play,refresh;
    private int left,top;
    private long nextPoll;
    public CasinoScreen(){super(Component.literal("Casino"));}
    @Override protected void init(){
        left=(width-400)/2;top=(height-230)/2;
        int y=top+30;
        for(String name:GAMES){final String key=name;button(label(name),left+8,y,92,16,()->{selected=key;option=key.equals("roulette")?"red":"heads";resume();rebuildWidgets();});y+=18;}
        bet=new EditBox(font,left+112,top+31,76,18,Component.literal("Bet in dollars"));bet.setMaxLength(12);bet.setValue("10");addRenderableWidget(bet);
        play=button("Review wager",left+204,top+30,104,20,this::review);
        refresh=button("Refresh",left+314,top+30,78,20,this::load);
        button("Back",left+8,top+210,92,16,()->minecraft.setScreen(new MenuScreen()));
        button("Option: "+option,left+112,top+59,160,18,()->{option=selected.equals("roulette")?(option.equals("red")?"black":option.equals("black")?"even":option.equals("even")?"odd":"red"):option.equals("heads")?"tails":"heads";rebuildWidgets();});
        if(game!=null&&active()){
            if(selected.equals("blackjack")){
                button("Hit",left+112,top+178,80,20,()->action("casino_blackjack_hit",new JsonObject()));
                button("Stand",left+200,top+178,80,20,()->action("casino_blackjack_stand",new JsonObject()));
                Button doub=button("Double",left+288,top+178,96,20,()->action("casino_blackjack_double",new JsonObject()));doub.active=game.has("can_double")&&game.get("can_double").getAsBoolean();
            }else if(selected.equals("mines")){
                int size=game.get("size").getAsInt(),tileSize=Math.min(18,80/Math.max(1,size));
                for(int i=0;i<size*size&&i<64;i++){final int cell=i;boolean revealed=game.getAsJsonArray("revealed").contains(new JsonPrimitive(i));
                    Button b=button(revealed?"+":"?",left+112+(i%size)*tileSize,top+94+(i/size)*tileSize,tileSize-1,tileSize-1,()->{JsonObject a=new JsonObject();a.addProperty("tile",cell);action("casino_mines_reveal",a);});b.active=!revealed;
                }
                button("Cash out",left+280,top+178,104,20,()->action("casino_mines_cashout",new JsonObject()));
            }else if(selected.equals("burst")){
                button("Advance round",left+112,top+178,130,20,()->burst(true));button("Cash out",left+254,top+178,130,20,()->burst(false));
            }else if(selected.equals("crash"))button("Cash out now",left+112,top+178,272,20,()->action("casino_crash_cashout",new JsonObject()));
        }
        if(lobby==null&&!busy)load();updateButtons();
    }
    private Button button(String text,int x,int y,int w,int h,Runnable run){return addRenderableWidget(Button.builder(Component.literal(text),b->{if(!busy)run.run();}).bounds(x,y,w,h).build());}
    private static String label(String s){return Character.toUpperCase(s.charAt(0))+s.substring(1);}
    private boolean active(){return game!=null&&game.has("status")&&game.get("status").getAsString().equals("active");}
    private void updateButtons(){if(play!=null)play.active=!busy&&!needsRefresh&&!active()&&lobby!=null&&lobby.has("enabled")&&lobby.get("enabled").getAsBoolean();if(refresh!=null)refresh.active=!busy;}
    private void resume(){game=null;if(lobby!=null&&lobby.has(selected)&&lobby.get(selected).isJsonObject())game=lobby.getAsJsonObject(selected);}
    private void load(){
        busy=true;updateButtons();ScopenetClient.link().request("casino",new JsonObject(),data->{busy=false;needsRefresh=false;lobby=data.getAsJsonObject();balance=lobby.get("balance").getAsDouble();resume();message="In-game dollars only.";rebuildWidgets();},error->{busy=false;needsRefresh=true;message=error;updateButtons();});
    }
    private void review(){
        try{
            double amount=new java.math.BigDecimal(bet.getValue()).doubleValue();if(!Double.isFinite(amount)||amount<=0||amount>balance)throw new IllegalArgumentException();
            final String gameName=selected;final String choice=option;
            minecraft.setScreen(new net.minecraft.client.gui.screens.ConfirmScreen(yes->{minecraft.setScreen(this);if(yes)start(gameName,choice,amount);},Component.literal("Confirm "+label(gameName)+" wager"),Component.literal(Ui.money("$",amount)+" in-game dollars. You may lose the full wager.")));
        }catch(RuntimeException invalid){message="Enter a positive wager within your balance.";}
    }
    private void start(String name,String choice,double amount){
        JsonObject args=new JsonObject();args.addProperty("bet",amount);args.addProperty("operation_id",UUID.randomUUID().toString());
        String operation="casino_"+name;
        switch(name){
            case "blackjack","mines","crash","burst"->operation+="_start";
        }
        if(name.equals("mines"))args.addProperty("mines",3);
        if(name.equals("crash"))args.addProperty("auto",2.0);
        if(name.equals("plinko")){args.addProperty("risk","medium");args.addProperty("rows",12);}
        if(name.equals("dice")){args.addProperty("chance",50);args.addProperty("mode","under");}
        if(name.equals("coinflip"))args.addProperty("side",choice);
        if(name.equals("roulette"))args.addProperty("selection",choice);
        action(operation,args);
    }
    private void burst(boolean advance){JsonObject a=new JsonObject();a.add("id",game.get("id"));a.add("expected_steps",game.get("steps"));a.addProperty("operation_id",UUID.randomUUID().toString());action("casino_burst_"+(advance?"advance":"cashout"),a);}
    private void action(String operation,JsonObject args){
        if(busy)return;busy=true;updateButtons();message="Waiting for server...";
        ScopenetClient.link().request(operation,args,data->{busy=false;JsonObject result=data.getAsJsonObject();if(result.has("balance"))balance=result.get("balance").getAsDouble();game=result.has("game")&&result.get("game").isJsonObject()?result.getAsJsonObject("game"):null;
            if(lobby!=null&&Set.of("blackjack","mines","crash","burst").contains(selected))lobby.add(selected,active()?game:JsonNull.INSTANCE);
            message=result.has("payout")?"Payout: "+Ui.money("$",result.get("payout").getAsDouble()):game!=null&&game.has("payout")?"Payout: "+Ui.money("$",game.get("payout").getAsDouble()):"Server accepted the action.";nextPoll=System.currentTimeMillis()+1000;rebuildWidgets();
        },error->{busy=false;needsRefresh=true;message=error+" Refresh to recover before playing.";updateButtons();});
    }
    @Override public void tick(){super.tick();if(bet!=null)bet.tick();if(selected.equals("crash")&&active()&&!busy&&!needsRefresh&&System.currentTimeMillis()>nextPoll){nextPoll=System.currentTimeMillis()+1000;action("casino_crash_status",new JsonObject());}}
    @Override public void render(GuiGraphics g,int mx,int my,float delta){
        renderBackground(g);Ui.window(g,left,top,400,230);g.drawString(font,"Velora Casino",left+10,top+10,Ui.ACCENT,true);g.drawString(font,Ui.money("$",balance),left+285,top+10,Ui.GOLD,true);
        String info=selected.equals("mines")?"3 mines. Reveal a tile or cash out.":selected.equals("crash")?"Auto cashout: 2x; manual cashout available.":selected.equals("dice")?"50% chance, roll under.":selected.equals("plinko")?"12 rows, medium risk.":selected.equals("burst")?"Risk another round or cash out.":"Review each wager before playing.";
        g.drawString(font,font.plainSubstrByWidth(info,278),left+112,top+83,Ui.MUTED,false);
        if(game!=null){if(game.has("multiplier"))g.drawString(font,String.format(Locale.ROOT,"Multiplier %.2fx",game.get("multiplier").getAsDouble()),left+240,top+111,Ui.GOLD,true);
            if(game.has("steps"))g.drawString(font,"Round "+game.get("steps").getAsInt(),left+112,top+111,Ui.TEXT,false);
            if(game.has("player_total")){g.drawString(font,"Your hand: "+game.get("player")+" = "+game.get("player_total"),left+112,top+108,Ui.TEXT,false);g.drawString(font,"Dealer: "+game.get("dealer")+" = "+game.get("dealer_total")+(game.get("hidden").getAsInt()>0?" + hidden":""),left+112,top+125,Ui.MUTED,false);}
        }
        g.drawString(font,font.plainSubstrByWidth(message,280),left+112,top+211,needsRefresh?Ui.GOLD:Ui.MUTED,false);super.render(g,mx,my,delta);
    }
    @Override public boolean isPauseScreen(){return false;}
}
