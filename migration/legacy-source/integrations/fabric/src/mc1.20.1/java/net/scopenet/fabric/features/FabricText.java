package net.scopenet.fabric.features;

import net.minecraft.network.chat.Component;
import net.minecraft.network.chat.MutableComponent;
import net.minecraft.network.chat.Style;
import net.minecraft.network.chat.TextColor;

import java.util.regex.Matcher;
import java.util.regex.Pattern;

/** Turns text with {@code &} codes and {@code &#RRGGBB} colours (as written in the admin panel) into a styled component. */
final class FabricText {
    private FabricText() {}

    private static final Pattern CODE = Pattern.compile("[&§]#([0-9a-fA-F]{6})|[&§]([0-9a-fk-orA-FK-OR])");
    private static final int[] COLORS = {0x000000, 0x0000aa, 0x00aa00, 0x00aaaa, 0xaa0000, 0xaa00aa, 0xffaa00, 0xaaaaaa,
            0x555555, 0x5555ff, 0x55ff55, 0x55ffff, 0xff5555, 0xff55ff, 0xffff55, 0xffffff};

    static MutableComponent parse(String text) {
        MutableComponent out = Component.empty();
        Style style = Style.EMPTY.withItalic(false);
        Matcher m = CODE.matcher(text);
        int last = 0;
        while (m.find()) {
            if (m.start() > last) out.append(Component.literal(text.substring(last, m.start())).withStyle(style));
            last = m.end();
            if (m.group(1) != null) {
                style = Style.EMPTY.withItalic(false).withColor(TextColor.fromRgb(Integer.parseInt(m.group(1), 16)));
            } else {
                char c = Character.toLowerCase(m.group(2).charAt(0));
                int idx = Character.digit(c, 16);
                if (idx >= 0) style = Style.EMPTY.withItalic(false).withColor(TextColor.fromRgb(COLORS[idx]));
                else switch (c) {
                    case 'l' -> style = style.withBold(true);
                    case 'o' -> style = style.withItalic(true);
                    case 'n' -> style = style.withUnderlined(true);
                    case 'm' -> style = style.withStrikethrough(true);
                    case 'k' -> style = style.withObfuscated(true);
                    default -> style = Style.EMPTY.withItalic(false);
                }
            }
        }
        if (last < text.length()) out.append(Component.literal(text.substring(last)).withStyle(style));
        return out;
    }
}
