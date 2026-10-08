package net.scopenet.client;

import com.google.gson.JsonArray;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;
import net.minecraft.util.FormattedCharSequence;
import java.util.ArrayList;
import java.util.List;

/** Readable dialogue from server-authored Content Studio NPCs, with ordinary chat retained as fallback. */
final class DialogueScreen extends Screen {
    private final JsonArray text;
    private final List<FormattedCharSequence> lines=new ArrayList<>();
    private int left,top,w,h,offset;
    DialogueScreen(String title,JsonArray text){super(Component.literal(Presentation.plain(title)));this.text=text;}
    @Override protected void init(){
        w=Math.min(360,width-16);h=Math.min(240,height-16);left=(width-w)/2;top=(height-h)/2;lines.clear();
        text.forEach(line->lines.addAll(font.split(Component.literal(Presentation.plain(line.getAsString())),w-28)));
        addRenderableWidget(Button.builder(Component.literal("Continue"),b->onClose()).bounds(left+w-94,top+h-28,84,20).build());
        addRenderableWidget(Button.builder(Component.literal("Quest journal"),b->Companion.open("quests")).bounds(left+10,top+h-28,102,20).build());
    }
    @Override public boolean mouseScrolled(double x,double y,double dx,double dy){offset=Math.clamp(offset-(int)Math.signum(dy),0,Math.max(0,lines.size()-(h-75)/12));return true;}
    @Override public void extractRenderState(GuiGraphicsExtractor g,int mx,int my,float delta){
        g.fill(0,0,width,height,0x99000000);Companion.panel(g,left,top,w,h);
        g.text(font,title,left+14,top+14,Companion.preferences.accentColor());g.horizontalLine(left+12,left+w-12,top+30,0xff4b5057);
        for(int i=offset;i<Math.min(lines.size(),offset+(h-75)/12);i++)g.text(font,lines.get(i),left+14,top+42+(i-offset)*12,0xffeee8dd);
        super.extractRenderState(g,mx,my,delta);
    }
    @Override public boolean isPauseScreen(){return false;}
}
