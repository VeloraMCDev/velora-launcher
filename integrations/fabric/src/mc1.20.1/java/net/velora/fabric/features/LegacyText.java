package net.velora.fabric.features;

import net.minecraft.ChatFormatting;
import net.minecraft.network.chat.Component;
import net.minecraft.network.chat.Style;
/** Converts panel-owned legacy color templates into actual text styles, including RGB colors. */
final class LegacyText {
    static Component component(String raw) {
        var out=Component.empty();Style style=Style.EMPTY;StringBuilder text=new StringBuilder();
        for(int i=0;i<raw.length();i++) {
            char c=raw.charAt(i);
            if(c!='\u00a7'||i+1>=raw.length()){text.append(c);continue;}
            if(!text.isEmpty()){out.append(Component.literal(text.toString()).setStyle(style));text.setLength(0);}
            char code=Character.toLowerCase(raw.charAt(++i));
            if(code=='x'&&i+12<raw.length()) {
                StringBuilder hex=new StringBuilder();int at=i;
                for(int j=0;j<6;j++){if(at+2>=raw.length()||raw.charAt(at+1)!='\u00a7'||Character.digit(raw.charAt(at+2),16)<0){hex.setLength(0);break;}hex.append(raw.charAt(at+2));at+=2;}
                if(hex.length()==6){style=Style.EMPTY.withColor(Integer.parseInt(hex.toString(),16));i=at;}continue;
            }
            ChatFormatting format=ChatFormatting.getByCode(code);
            if(format==null)continue;
            if(format==ChatFormatting.RESET)style=Style.EMPTY;
            else if(format.isColor())style=Style.EMPTY.applyFormat(format);
            else style=style.applyFormat(format);
        }
        if(!text.isEmpty())out.append(Component.literal(text.toString()).setStyle(style));return out;
    }
}
