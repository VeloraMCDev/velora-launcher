package net.velora.client.screen;

import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.screens.inventory.AbstractContainerScreen;
import net.minecraft.network.chat.Component;
import net.minecraft.world.entity.player.Inventory;
import net.velora.core.vault.VaultMenu;

/** Seven rows with a drawn background: no vanilla six-row texture cropping. */
public final class VaultScreen extends AbstractContainerScreen<VaultMenu> {
    public VaultScreen(VaultMenu menu, Inventory inventory, Component title) {
        super(menu, inventory, title); imageWidth = 176; imageHeight = 240; inventoryLabelY = 146;
    }
    @Override protected void renderBg(GuiGraphics graphics, float delta, int mouseX, int mouseY) {
        graphics.fill(leftPos - 2, topPos - 2, leftPos + imageWidth + 2, topPos + imageHeight + 2, 0xff7760b8);
        graphics.fill(leftPos, topPos, leftPos + imageWidth, topPos + imageHeight, 0xff171421);
        for (var slot : menu.slots) {
            int x = leftPos + slot.x, y = topPos + slot.y;
            graphics.fill(x - 1, y - 1, x + 17, y + 17, 0xff514760);
            graphics.fill(x, y, x + 16, y + 16, 0xff282232);
        }
    }
    @Override public void render(GuiGraphics graphics, int mouseX, int mouseY, float delta) {
        renderBackground(graphics); super.render(graphics, mouseX, mouseY, delta); renderTooltip(graphics, mouseX, mouseY);
    }
    @Override protected void renderLabels(GuiGraphics graphics, int mouseX, int mouseY) {
        graphics.drawString(font, title, titleLabelX, titleLabelY, 0xffeee8ff, false);
        graphics.drawString(font, "Click: move whole stack", 8, 137, 0xffa99bbf, false);
        graphics.drawString(font, playerInventoryTitle, inventoryLabelX, inventoryLabelY, 0xffa99bbf, false);
    }
}
