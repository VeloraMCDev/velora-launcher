package net.scopenet.core.vault;

import net.fabricmc.fabric.api.screenhandler.v1.ScreenHandlerRegistry;
import net.minecraft.resources.ResourceLocation;
import net.minecraft.world.Container;
import net.minecraft.world.SimpleContainer;
import net.minecraft.world.entity.player.Inventory;
import net.minecraft.world.inventory.ChestMenu;
import net.minecraft.world.inventory.MenuType;

/** Same 63 slots and menu identity on both artifacts. Item state belongs to the server. */
public class VaultMenu extends ChestMenu {
    public static final MenuType<VaultMenu> TYPE = ScreenHandlerRegistry.registerSimple(
            new ResourceLocation("scopenet", "vault_7rows"), (id, inventory) -> new VaultMenu(id, inventory, new SimpleContainer(63)));
    public static void register() { /* Forces registration during initialization on each side. */ }
    public VaultMenu(int id, Inventory inventory, Container container) { super(TYPE, id, inventory, container, 7); }
}
