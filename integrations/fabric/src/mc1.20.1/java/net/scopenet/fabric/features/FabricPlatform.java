package net.scopenet.fabric.features;

import net.minecraft.core.BlockPos;
import net.minecraft.core.registries.Registries;
import net.minecraft.resources.ResourceKey;
import net.minecraft.resources.ResourceLocation;
import net.minecraft.network.chat.Component;
import net.minecraft.server.MinecraftServer;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.levelgen.Heightmap;
import net.scopenet.core.CorePlayer;
import net.scopenet.core.Platform;
import net.scopenet.core.Pos;

import java.util.ArrayList;
import java.util.Collection;
import java.util.List;
import java.util.Optional;
import java.util.UUID;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

/** The shared logic's view of a running Fabric server. */
final class FabricPlatform implements Platform {
    @Override public boolean offerResourcePack(UUID id, String url, String sha1, boolean required) {
        ServerPlayer p = server.getPlayerList().getPlayer(id); if (p == null) return false;
        p.sendTexturePack(url, sha1, required, Component.literal("Velora custom assets")); return true;
    }
    private final MinecraftServer server;
    private final Perms perms;
    private final ExecutorService io = Executors.newFixedThreadPool(4, r -> {
        Thread t = new Thread(r, "scopenet-io");
        t.setDaemon(true);
        return t;
    });

    FabricPlatform(MinecraftServer server, Perms perms) { this.server = server; this.perms = perms; }

    void shutdown() { io.shutdownNow(); }

    CorePlayer wrap(ServerPlayer p) { return new FabricPlayer(p, this, perms); }

    ServerLevel level(String id) {
        ResourceLocation location = ResourceLocation.tryParse(id);
        return location == null ? null : server.getLevel(ResourceKey.create(Registries.DIMENSION, location));
    }

    @Override public Optional<CorePlayer> player(UUID uuid) { return Optional.ofNullable(server.getPlayerList().getPlayer(uuid)).map(this::wrap); }

    @Override public Optional<CorePlayer> playerByName(String name) { return Optional.ofNullable(server.getPlayerList().getPlayerByName(name)).map(this::wrap); }

    @Override public Collection<? extends CorePlayer> online() {
        List<CorePlayer> out = new ArrayList<>();
        for (ServerPlayer p : server.getPlayerList().getPlayers()) out.add(wrap(p));
        return out;
    }

    @Override public Optional<Pos> safeSurface(String world, int x, int z) {
        ServerLevel level = level(world);
        if (level == null || level.dimensionType().hasCeiling()) return Optional.empty(); // the Nether roof is bedrock
        int y = level.getHeight(Heightmap.Types.MOTION_BLOCKING_NO_LEAVES, x, z);
        if (y <= level.getMinBuildHeight() + 1) return Optional.empty();
        BlockState ground = level.getBlockState(new BlockPos(x, y - 1, z));
        if (!ground.blocksMotion() || !ground.getFluidState().isEmpty()) return Optional.empty();
        return Optional.of(new Pos(world, x + 0.5, y, z + 0.5, 0, 0));
    }

    @Override public Pos worldSpawn(String world) {
        ServerLevel level = level(world);
        if (level == null) level = server.overworld();
        BlockPos spawn = level.getSharedSpawnPos();
        return new Pos(level.dimension().location().toString(), spawn.getX(), spawn.getY(), spawn.getZ(), 0, 0);
    }

    @Override public void runMain(Runnable task) { server.execute(task); }

    @Override public void runAsync(Runnable task) { io.execute(task); }

    @Override public boolean giveItem(UUID id, String item, int amount) {
        ServerPlayer p = server.getPlayerList().getPlayer(id);
        ResourceLocation location = ResourceLocation.tryParse(item);
        if (p == null || location == null) return false;
        var found = net.minecraft.core.registries.BuiltInRegistries.ITEM.getOptional(location);
        if (found.isEmpty() || found.get() == net.minecraft.world.item.Items.AIR) return false;
        net.minecraft.world.item.Item type = found.get();
        for (int left = amount; left > 0; ) {
            int n = Math.min(left, type.getMaxStackSize());
            net.minecraft.world.item.ItemStack stack = new net.minecraft.world.item.ItemStack(type, n);
            p.getInventory().add(stack);
            if (!stack.isEmpty()) p.drop(stack, false);
            left -= n;
        }
        return true;
    }

    @Override public boolean console(String command) {
        server.getCommands().performPrefixedCommand(server.createCommandSourceStack(), command);
        return true;
    }

    // ---- utilities ---------------------------------------------------------------------------------

    /** Players who just stopped flying; fall damage is waived until they land. */
    private final java.util.Map<UUID, Long> softLanding = new java.util.concurrent.ConcurrentHashMap<>();

    boolean waiveFall(UUID id) {
        Long until = softLanding.remove(id);
        return until != null && System.currentTimeMillis() < until;
    }

    private static boolean survival(ServerPlayer p) { return !p.isCreative() && !p.isSpectator(); }

    @Override public boolean giveSpec(UUID id, net.scopenet.core.ItemSpec spec) {
        ServerPlayer p = server.getPlayerList().getPlayer(id);
        if (!spec.dataFormat().isEmpty()) {
            if (p == null || !spec.dataFormat().equals("nbt")) return false;
            try {
                var template = net.minecraft.world.item.ItemStack.of(net.minecraft.nbt.TagParser.parseTag(spec.dataValue()));
                if (template.isEmpty()) return false;
                for (int left = spec.amount(); left > 0;) {
                    int n = Math.min(left, template.getMaxStackSize());
                    var stack = template.copy(); stack.setCount(n);
                    p.getInventory().add(stack); if (!stack.isEmpty()) p.drop(stack, false);
                    left -= n;
                }
                return true;
            } catch (Exception e) { return false; }
        }
        ResourceLocation location = ResourceLocation.tryParse(spec.registryId());
        if (p == null || location == null) return false;
        var found = net.minecraft.core.registries.BuiltInRegistries.ITEM.getOptional(location);
        if (found.isEmpty() || found.get() == net.minecraft.world.item.Items.AIR) return false;
        for (int left = Math.max(1, spec.amount()); left > 0; ) {
            int n = Math.min(left, found.get().getMaxStackSize());
            net.minecraft.world.item.ItemStack stack = new net.minecraft.world.item.ItemStack(found.get(), n);
            if (!spec.isPlain()) style(stack, spec);
            p.getInventory().add(stack);
            if (!stack.isEmpty()) p.drop(stack, false);
            left -= n;
        }
        return true;
    }

    @Override public com.google.gson.JsonArray inventorySnapshot(UUID id) {
        var out = new com.google.gson.JsonArray();
        ServerPlayer p = server.getPlayerList().getPlayer(id);
        if (p == null) return out;
        for (var stack : p.getInventory().items) {
            if (stack.isEmpty()) continue;
            var item = new com.google.gson.JsonObject();
            item.addProperty("item", net.minecraft.core.registries.BuiltInRegistries.ITEM.getKey(stack.getItem()).toString());
            item.addProperty("amount", stack.getCount());
            var data = new com.google.gson.JsonObject(); data.addProperty("format", "nbt");
            data.addProperty("value", stack.save(new net.minecraft.nbt.CompoundTag()).toString()); item.add("data", data);
            out.add(item);
        }
        return out;
    }

    private static void style(net.minecraft.world.item.ItemStack stack, net.scopenet.core.ItemSpec spec) {
        if (!spec.name().isBlank()) stack.setHoverName(FabricText.parse(spec.name()));
        if (!spec.lore().isEmpty()) {
            net.minecraft.nbt.ListTag lore = new net.minecraft.nbt.ListTag();
            for (String line : spec.lore()) lore.add(net.minecraft.nbt.StringTag.valueOf(Component.Serializer.toJson(FabricText.parse(line))));
            stack.getOrCreateTagElement("display").put("Lore", lore);
        }
        for (var e : spec.enchants().entrySet()) {
            ResourceLocation key = ResourceLocation.tryParse(e.getKey().contains(":") ? e.getKey() : "minecraft:" + e.getKey());
            var ench = key == null ? null : net.minecraft.core.registries.BuiltInRegistries.ENCHANTMENT.getOptional(key).orElse(null);
            if (ench != null) stack.enchant(ench, e.getValue());
        }
        int hide = 0;
        if (spec.glow() && spec.enchants().isEmpty()) {
            stack.enchant(net.minecraft.world.item.enchantment.Enchantments.UNBREAKING, 1);
            hide |= 1;
        }
        if (spec.unbreakable()) stack.getOrCreateTag().putBoolean("Unbreakable", true);
        if (spec.hideFlags()) hide = 255;
        if (hide != 0) stack.getOrCreateTag().putInt("HideFlags", hide);
        if (spec.customModelData() >= 0) stack.getOrCreateTag().putInt("CustomModelData", spec.customModelData());
        for (net.scopenet.core.ItemSpec.Attribute a : spec.attributes()) {
            net.minecraft.world.entity.ai.attributes.Attribute attribute = attribute(a.attribute());
            if (attribute == null) continue;
            var op = switch (a.operation()) {
                case "multiply_base" -> net.minecraft.world.entity.ai.attributes.AttributeModifier.Operation.MULTIPLY_BASE;
                case "multiply_total" -> net.minecraft.world.entity.ai.attributes.AttributeModifier.Operation.MULTIPLY_TOTAL;
                default -> net.minecraft.world.entity.ai.attributes.AttributeModifier.Operation.ADDITION;
            };
            net.minecraft.world.entity.EquipmentSlot slot = switch (a.slot()) {
                case "offhand" -> net.minecraft.world.entity.EquipmentSlot.OFFHAND;
                case "head" -> net.minecraft.world.entity.EquipmentSlot.HEAD;
                case "chest" -> net.minecraft.world.entity.EquipmentSlot.CHEST;
                case "legs" -> net.minecraft.world.entity.EquipmentSlot.LEGS;
                case "feet" -> net.minecraft.world.entity.EquipmentSlot.FEET;
                default -> net.minecraft.world.entity.EquipmentSlot.MAINHAND;
            };
            stack.addAttributeModifier(attribute, new net.minecraft.world.entity.ai.attributes.AttributeModifier(UUID.randomUUID(), "scopenet", a.amount(), op), slot);
        }
    }

    private static net.minecraft.world.entity.ai.attributes.Attribute attribute(String id) {
        return switch (id) {
            case "generic.max_health" -> net.minecraft.world.entity.ai.attributes.Attributes.MAX_HEALTH;
            case "generic.attack_damage" -> net.minecraft.world.entity.ai.attributes.Attributes.ATTACK_DAMAGE;
            case "generic.attack_speed" -> net.minecraft.world.entity.ai.attributes.Attributes.ATTACK_SPEED;
            case "generic.movement_speed" -> net.minecraft.world.entity.ai.attributes.Attributes.MOVEMENT_SPEED;
            case "generic.armor" -> net.minecraft.world.entity.ai.attributes.Attributes.ARMOR;
            case "generic.armor_toughness" -> net.minecraft.world.entity.ai.attributes.Attributes.ARMOR_TOUGHNESS;
            case "generic.knockback_resistance" -> net.minecraft.world.entity.ai.attributes.Attributes.KNOCKBACK_RESISTANCE;
            case "generic.luck" -> net.minecraft.world.entity.ai.attributes.Attributes.LUCK;
            case "generic.attack_knockback" -> net.minecraft.world.entity.ai.attributes.Attributes.ATTACK_KNOCKBACK;
            default -> null;
        };
    }

    @Override public void heal(UUID id, double amount) {
        ServerPlayer p = server.getPlayerList().getPlayer(id);
        if (p == null || !survival(p)) return;
        p.setHealth((float) (amount <= 0 ? p.getMaxHealth() : Math.min(p.getMaxHealth(), p.getHealth() + amount)));
        p.clearFire();
    }

    @Override public void feed(UUID id, int amount) {
        ServerPlayer p = server.getPlayerList().getPlayer(id);
        if (p == null) return;
        var food = p.getFoodData();
        food.setFoodLevel(Math.min(20, food.getFoodLevel() + Math.max(0, amount)));
        food.setSaturation(Math.min(20f, food.getSaturationLevel() + Math.max(0, amount)));
    }

    @Override public void setFlight(UUID id, boolean allowed) {
        ServerPlayer p = server.getPlayerList().getPlayer(id);
        if (p == null || !survival(p)) return;
        p.getAbilities().mayfly = allowed;
        if (allowed) {
            p.getAbilities().flying = p.serverLevel().getBlockState(p.blockPosition().below()).isAir();
        } else {
            if (p.getAbilities().flying) softLanding.put(id, System.currentTimeMillis() + 15_000);
            p.getAbilities().flying = false;
        }
        p.onUpdateAbilities();
    }

    @Override public void actionbar(UUID id, String text) {
        ServerPlayer p = server.getPlayerList().getPlayer(id);
        if (p != null) p.displayClientMessage(FabricText.parse(text), true);
    }

    @Override public void openEnderChest(UUID id) {
        ServerPlayer p = server.getPlayerList().getPlayer(id);
        if (p == null) return;
        p.openMenu(new net.minecraft.world.SimpleMenuProvider(
                (windowId, inventory, player) -> net.minecraft.world.inventory.ChestMenu.threeRows(windowId, inventory, player.getEnderChestInventory()),
                Component.translatable("container.enderchest")));
    }

    private static final net.minecraft.world.inventory.MenuType<?>[] CHEST_MENUS = {
            net.minecraft.world.inventory.MenuType.GENERIC_9x1, net.minecraft.world.inventory.MenuType.GENERIC_9x2,
            net.minecraft.world.inventory.MenuType.GENERIC_9x3, net.minecraft.world.inventory.MenuType.GENERIC_9x4,
            net.minecraft.world.inventory.MenuType.GENERIC_9x5, net.minecraft.world.inventory.MenuType.GENERIC_9x6};

    /** Vaults a player has open right now, keyed by owner and number: the live copy that a delivery must go into. */
    private final java.util.Map<String, net.minecraft.world.SimpleContainer> openVaults = new java.util.concurrent.ConcurrentHashMap<>();

    private java.io.File vaultFile(UUID owner) {
        java.io.File dir = java.nio.file.Path.of("config", "scopenet", "vaults").toFile();
        dir.mkdirs();
        return new java.io.File(dir, owner + ".dat");
    }

    @Override public void openVault(UUID id, int number, int rows) {
        ServerPlayer p = server.getPlayerList().getPlayer(id);
        if (p == null) return;
        rows = Math.max(1, Math.min(6, rows));
        final int fixedRows = rows;
        java.io.File file = vaultFile(id);
        net.minecraft.nbt.CompoundTag root = new net.minecraft.nbt.CompoundTag();
        try { if (file.exists()) root = net.minecraft.nbt.NbtIo.readCompressed(file); } catch (java.io.IOException e) { p.sendSystemMessage(Component.literal("§cYour vaults could not be read; ask an admin.")); return; }
        final net.minecraft.nbt.CompoundTag data = root;
        net.minecraft.world.SimpleContainer container = new net.minecraft.world.SimpleContainer(rows * 9);
        net.minecraft.nbt.ListTag saved = data.getList("v" + number, 10);
        int overflow = 0;
        for (int i = 0; i < saved.size(); i++) {
            net.minecraft.nbt.CompoundTag entry = saved.getCompound(i);
            net.minecraft.world.item.ItemStack stack = net.minecraft.world.item.ItemStack.of(entry);
            int slot = entry.getInt("VSlot");
            if (stack.isEmpty()) continue;
            if (slot < container.getContainerSize()) container.setItem(slot, stack);
            else { p.getInventory().add(stack); if (!stack.isEmpty()) p.drop(stack, false); overflow++; }
        }
        if (overflow > 0) p.sendSystemMessage(Component.literal("§eThis vault is smaller than before; " + overflow + " stack(s) were moved to your inventory."));
        Runnable save = () -> {
            net.minecraft.nbt.ListTag list = new net.minecraft.nbt.ListTag();
            for (int slot = 0; slot < container.getContainerSize(); slot++) {
                net.minecraft.world.item.ItemStack stack = container.getItem(slot);
                if (stack.isEmpty()) continue;
                net.minecraft.nbt.CompoundTag entry = stack.save(new net.minecraft.nbt.CompoundTag());
                entry.putInt("VSlot", slot);
                list.add(entry);
            }
            data.put("v" + number, list);
            try { net.minecraft.nbt.NbtIo.writeCompressed(data, file); } catch (java.io.IOException e) { /* retried on the next change */ }
        };
        container.addListener(c -> save.run());
        final String openKey = id + ":" + number;
        p.openMenu(new net.minecraft.world.SimpleMenuProvider(
                (windowId, inventory, player) -> {
                    openVaults.put(openKey, container);
                    return new net.minecraft.world.inventory.ChestMenu(CHEST_MENUS[fixedRows - 1], windowId, inventory, container, fixedRows) {
                        @Override public void removed(net.minecraft.world.entity.player.Player who) {
                            super.removed(who);
                            openVaults.remove(openKey, container);
                        }
                    };
                },
                Component.literal("Vault " + number)));
    }

    @Override public boolean depositToVault(UUID id, net.scopenet.core.Item item, int vaults, int rows) {
        net.minecraft.world.item.ItemStack stack = FabricItems.fromCore(item);
        if (stack.isEmpty()) return false;
        int size = Math.max(1, Math.min(6, rows)) * 9;
        // A vault being looked at is the live copy; its listener saves the change.
        for (int n = 1; n <= Math.max(1, vaults); n++) {
            net.minecraft.world.SimpleContainer open = openVaults.get(id + ":" + n);
            if (open != null) {
                net.minecraft.world.item.ItemStack left = open.addItem(stack.copy());
                if (left.isEmpty()) return true;
            }
        }
        java.io.File file = vaultFile(id);
        net.minecraft.nbt.CompoundTag data = new net.minecraft.nbt.CompoundTag();
        try { if (file.exists()) data = net.minecraft.nbt.NbtIo.readCompressed(file); } catch (java.io.IOException e) { return false; }
        for (int n = 1; n <= Math.max(1, vaults); n++) {
            if (openVaults.containsKey(id + ":" + n)) continue;
            net.minecraft.world.SimpleContainer box = new net.minecraft.world.SimpleContainer(size);
            net.minecraft.nbt.ListTag saved = data.getList("v" + n, 10);
            for (int i = 0; i < saved.size(); i++) {
                net.minecraft.nbt.CompoundTag entry = saved.getCompound(i);
                int slot = entry.getInt("VSlot");
                net.minecraft.world.item.ItemStack there = net.minecraft.world.item.ItemStack.of(entry);
                if (!there.isEmpty() && slot >= 0 && slot < size) box.setItem(slot, there);
            }
            if (!box.addItem(stack.copy()).isEmpty()) continue;
            net.minecraft.nbt.ListTag list = new net.minecraft.nbt.ListTag();
            for (int slot = 0; slot < size; slot++) {
                net.minecraft.world.item.ItemStack there = box.getItem(slot);
                if (there.isEmpty()) continue;
                net.minecraft.nbt.CompoundTag entry = there.save(new net.minecraft.nbt.CompoundTag());
                entry.putInt("VSlot", slot);
                list.add(entry);
            }
            data.put("v" + n, list);
            try { net.minecraft.nbt.NbtIo.writeCompressed(data, file); return true; } catch (java.io.IOException e) { return false; }
        }
        return false;
    }
}
