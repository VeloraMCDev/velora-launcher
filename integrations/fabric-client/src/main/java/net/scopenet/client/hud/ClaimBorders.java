package net.scopenet.client.hud;

import net.fabricmc.fabric.api.client.rendering.v1.WorldRenderContext;
import com.mojang.blaze3d.vertex.PoseStack;
import com.mojang.blaze3d.vertex.VertexConsumer;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.LevelRenderer;
import net.minecraft.client.renderer.RenderType;
import net.minecraft.world.phys.Vec3;
import net.scopenet.client.ClientConfig;
import net.scopenet.client.ClientState;

/** Draws the borders of claimed chunks around you in the world: green for your guild's land, red for anyone else's. */
public final class ClaimBorders {
    private final ClientState state;
    private final ClientConfig config;

    public ClaimBorders(ClientState state, ClientConfig config) { this.state = state; this.config = config; }

    public void render(WorldRenderContext context) {
        Minecraft mc = Minecraft.getInstance();
        if (!config.module("factions") || !state.module("factions")) return;
        if (!config.enabled || !config.hud.claimBorders || !state.connected || !state.featClaims || state.claimCells.isEmpty() || mc.player == null) return;
        if (config.hud.claimBordersOnlyWhenSneaking && !mc.player.isShiftKeyDown()) return;
        if (context.consumers() == null) return;
        // Only the world the server sent the map for.
        String dim = mc.level == null ? "" : mc.level.dimension().location().toString();
        if (!dim.equals(state.claimDim)) return;

        int r = state.claimRadius, side = r * 2 + 1;
        String cells = state.claimCells;
        if (cells.length() != side * side) return;
        Vec3 cam = context.camera().getPosition();
        PoseStack pose = context.matrixStack();
        VertexConsumer lines = context.consumers().getBuffer(RenderType.lines());
        double y0 = Math.floor(mc.player.getY()) - 6, y1 = y0 + 14;
        pose.pushPose();
        pose.translate(-cam.x, -cam.y, -cam.z);
        for (int dz = -r; dz <= r; dz++) {
            for (int dx = -r; dx <= r; dx++) {
                char cell = cells.charAt((dz + r) * side + (dx + r));
                if (cell == '0') continue;
                float cr = cell == '1' ? 0.2f : 0.98f, cg = cell == '1' ? 0.85f : 0.35f, cb = cell == '1' ? 0.55f : 0.45f;
                double x0 = (state.claimCx + dx) * 16.0, z0 = (state.claimCz + dz) * 16.0;
                LevelRenderer.renderLineBox(pose, lines, x0 + 0.02, y0, z0 + 0.02, x0 + 15.98, y1, z0 + 15.98, cr, cg, cb, 0.9f);
            }
        }
        pose.popPose();
    }
}
