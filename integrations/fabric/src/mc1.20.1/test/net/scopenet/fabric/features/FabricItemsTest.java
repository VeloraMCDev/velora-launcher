package net.scopenet.fabric.features;

import net.minecraft.SharedConstants;
import net.minecraft.server.Bootstrap;
import net.minecraft.nbt.CompoundTag;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.Items;
import net.scopenet.core.Item;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

class FabricItemsTest {
    @BeforeAll static void bootstrap(){SharedConstants.tryDetectVersion();Bootstrap.bootStrap();}
    @Test void partialStackRestorationUsesTheTransferredCountAndPreservesExactTag(){
        ItemStack source=new ItemStack(Items.DIAMOND,64);
        CompoundTag tag=new CompoundTag();tag.putString("custom","synthetic product");source.setTag(tag);
        Item encoded=FabricItems.toCore(source);
        ItemStack restored=FabricItems.fromCore(new Item(encoded.id(),encoded.name(),2,encoded.data()));
        assertEquals(2,restored.getCount());assertEquals(source.getTag(),restored.getTag());assertSame(source.getItem(),restored.getItem());
    }
    @Test void shopFingerprintIgnoresCountAndCompoundKeyOrderButDistinguishesTagValues(){
        ItemStack first=new ItemStack(Items.DIAMOND,4),second=new ItemStack(Items.DIAMOND,2);
        CompoundTag a=new CompoundTag(),b=new CompoundTag();a.putInt("Aa",1);a.putInt("BB",2);b.putInt("BB",2);b.putInt("Aa",1);first.setTag(a);second.setTag(b);
        assertEquals(FabricPlatform.itemFingerprint(first),FabricPlatform.itemFingerprint(second));
        b.putInt("BB",3);assertNotEquals(FabricPlatform.itemFingerprint(first),FabricPlatform.itemFingerprint(second));
        assertEquals(64,FabricPlatform.itemFingerprint(first).length());
    }
}
