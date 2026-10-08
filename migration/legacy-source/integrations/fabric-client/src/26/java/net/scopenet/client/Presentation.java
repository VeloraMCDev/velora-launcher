package net.scopenet.client;

import com.google.gson.JsonObject;
import net.minecraft.core.component.DataComponents;
import net.minecraft.core.registries.Registries;
import net.minecraft.client.Minecraft;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.Identifier;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.component.CustomModelData;
import net.minecraft.world.item.component.ItemLore;
import java.util.List;

/** Safe previews of vanilla-backed content. Server resource packs own models, entities and block displays. */
final class Presentation {
    static String plain(String s){return s.replaceAll("(?i)[&§][0-9a-fk-or]","");}
    static ItemStack icon(String id){
        try{
            String key=id.toLowerCase(java.util.Locale.ROOT);if(!key.contains(":"))key="minecraft:"+key;
            var connection=Minecraft.getInstance().getConnection();if(connection==null)return ItemStack.EMPTY;
            // 26.3 prototypes belong to the connection's bound holders. Static holders have no components.
            return connection.registryAccess().lookupOrThrow(Registries.ITEM).get(Identifier.parse(key)).map(ItemStack::new).orElse(ItemStack.EMPTY);
        }catch(Exception e){return ItemStack.EMPTY;}
    }
    static ItemStack stack(JsonObject spec){
        ItemStack stack=icon(Model.text(spec,"item"));if(stack.isEmpty())return stack;
        String name=plain(Model.text(spec,"name"));if(!name.isBlank())stack.set(DataComponents.CUSTOM_NAME,Component.literal(name));
        int model=spec.has("customModelData")?(int)Model.number(spec,"customModelData"):(int)Model.number(spec,"custom_model_data");
        if(model>=0&&(spec.has("customModelData")||spec.has("custom_model_data")))stack.set(DataComponents.CUSTOM_MODEL_DATA,new CustomModelData(List.of((float)model),List.of(),List.of(),List.of()));
        List<Component> lore=Model.array(spec,"lore").asList().stream().limit(32).map(e->(Component)Component.literal(plain(e.getAsString()))).toList();
        if(!lore.isEmpty())stack.set(DataComponents.LORE,new ItemLore(lore));
        if(Model.flag(spec,"glow"))stack.set(DataComponents.ENCHANTMENT_GLINT_OVERRIDE,true);
        stack.setCount(Math.clamp((int)Model.number(spec,"amount"),1,64));return stack;
    }
}
