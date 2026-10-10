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
        rows = Math.max(1, Math.min(7, rows));
        if (rows == 7 && !net.fabricmc.fabric.api.networking.v1.ServerPlayNetworking.canSend(p, new net.minecraft.resources.ResourceLocation("scopenet", "s2c"))) {
            p.sendSystemMessage(Component.literal("Velora Core Client is required to open a seven-row vault.")); return;
        }
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
            if (slot >= 0 && slot < container.getContainerSize()) container.setItem(slot, stack);
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
            // Another vault or a mailbox delivery may have saved since this menu opened.
            // Merge just this vault into the latest root instead of overwriting those changes.
            try {
                net.minecraft.nbt.CompoundTag latest = file.exists() ? net.minecraft.nbt.NbtIo.readCompressed(file) : new net.minecraft.nbt.CompoundTag();
                latest.put("v" + number, list);
                java.nio.file.Path temporary = file.toPath().resolveSibling(file.getName() + ".tmp");
                net.minecraft.nbt.NbtIo.writeCompressed(latest, temporary.toFile());
                try { java.nio.file.Files.move(temporary, file.toPath(), java.nio.file.StandardCopyOption.ATOMIC_MOVE, java.nio.file.StandardCopyOption.REPLACE_EXISTING); }
                catch (java.nio.file.AtomicMoveNotSupportedException unsupported) { java.nio.file.Files.move(temporary, file.toPath(), java.nio.file.StandardCopyOption.REPLACE_EXISTING); }
            } catch (java.io.IOException e) {
                System.err.println("[Velora Core] Vault save failed: " + e.getMessage());
                p.sendSystemMessage(Component.literal("Vault storage failed; contact an administrator immediately."));
            }
        };
        container.addListener(c -> save.run());
        if (overflow > 0) save.run();
        final String openKey = id + ":" + number;
        p.openMenu(new net.minecraft.world.SimpleMenuProvider(
                (windowId, inventory, player) -> {
                    openVaults.put(openKey, container);
                    if (fixedRows == 7) return new net.scopenet.core.vault.VaultMenu(windowId, inventory, container) {
                        @Override public void removed(net.minecraft.world.entity.player.Player who) {
                            super.removed(who); openVaults.remove(openKey, container);
                        }
                    };
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
        int size = Math.max(1, Math.min(7, rows)) * 9;
        // A vault being looked at is the live copy; its listener saves the change.
        for (int n = 1; n <= Math.max(1, vaults); n++) {
            net.minecraft.world.SimpleContainer open = openVaults.get(id + ":" + n);
            if (open != null && fits(open, stack)) {
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

    // ---- cloud vaults --------------------------------------------------------------------------------------

    @Override public boolean cloudVaults() { return true; }

    static com.google.gson.JsonObject stackJson(int slot, net.minecraft.world.item.ItemStack stack) {
        com.google.gson.JsonObject o = new com.google.gson.JsonObject();
        String name = stack.getHoverName().getString();
        o.addProperty("slot", slot);
        o.addProperty("item", net.minecraft.core.registries.BuiltInRegistries.ITEM.getKey(stack.getItem()).toString());
        o.addProperty("count", stack.getCount());
        o.addProperty("name", name.length() > 128 ? name.substring(0, 128) : name);
        o.addProperty("data", stack.save(new net.minecraft.nbt.CompoundTag()).toString());
        o.addProperty("fingerprint", itemFingerprint(stack));
        return o;
    }

    static String itemFingerprint(net.minecraft.world.item.ItemStack stack) {
        String value=net.minecraft.core.registries.BuiltInRegistries.ITEM.getKey(stack.getItem())+"|"+(stack.hasTag()?canonicalTag(stack.getTag()):"");
        try{return java.util.HexFormat.of().formatHex(java.security.MessageDigest.getInstance("SHA-256").digest(value.getBytes(java.nio.charset.StandardCharsets.UTF_8)));}
        catch(java.security.NoSuchAlgorithmException impossible){throw new IllegalStateException(impossible);}
    }
    private static String canonicalTag(net.minecraft.nbt.Tag tag){
        if(tag instanceof net.minecraft.nbt.CompoundTag compound){
            var keys=new java.util.ArrayList<>(compound.getAllKeys());java.util.Collections.sort(keys);
            StringBuilder out=new StringBuilder("{");for(String key:keys)out.append(net.minecraft.nbt.StringTag.valueOf(key)).append(':').append(canonicalTag(compound.get(key))).append(',');return out.append('}').toString();
        }
        if(tag instanceof net.minecraft.nbt.ListTag list){StringBuilder out=new StringBuilder("[");for(var value:list)out.append(canonicalTag(value)).append(',');return out.append(']').toString();}
        return tag.toString();
    }

    private static net.minecraft.world.item.ItemStack stackOf(com.google.gson.JsonObject o) {
        String data = o.has("data") && o.get("data").isJsonPrimitive() ? o.get("data").getAsString() : "";
        String item = o.get("item").getAsString();
        return FabricItems.fromCore(new net.scopenet.core.Item(item, item, o.get("count").getAsInt(), data));
    }

    private static com.google.gson.JsonArray contentsOf(net.minecraft.world.Container container) {
        com.google.gson.JsonArray out = new com.google.gson.JsonArray();
        for (int slot = 0; slot < container.getContainerSize(); slot++) {
            var stack = container.getItem(slot);
            if (!stack.isEmpty()) out.add(stackJson(slot, stack));
        }
        return out;
    }

    /** One open vault's live container, shared by everyone on this server who opens it. */
    private final class CloudView implements VaultView {
        final net.minecraft.world.SimpleContainer container;
        final int rows;
        final String title;
        final java.util.function.Consumer<UUID> closed;
        VaultTransfer transfer;
        boolean busy;

        CloudView(net.minecraft.world.SimpleContainer container, int rows, String title, java.util.function.Consumer<UUID> closed) {
            this.container = container; this.rows = rows; this.title = title; this.closed = closed;
        }

        @Override public com.google.gson.JsonArray contents() { return contentsOf(container); }
        @Override public void transfers(VaultTransfer handler){transfer=handler;}
        @Override public void replace(com.google.gson.JsonArray contents){container.clearContent();for(var e:contents){var o=e.getAsJsonObject();container.setItem(o.get("slot").getAsInt(),stackOf(o));}}
        void click(net.minecraft.world.inventory.ChestMenu menu,int slot,int button,net.minecraft.world.inventory.ClickType type,net.minecraft.world.entity.player.Player who) {
            if(!(who instanceof ServerPlayer p)||busy||transfer==null||slot<0||slot>=menu.slots.size()||button!=0||!(type==net.minecraft.world.inventory.ClickType.PICKUP||type==net.minecraft.world.inventory.ClickType.QUICK_MOVE)||!menu.getCarried().isEmpty())return;
            var selected=menu.slots.get(slot);var stack=selected.getItem();if(stack.isEmpty())return;
            net.minecraft.nbt.ListTag before=p.getInventory().save(new net.minecraft.nbt.ListTag());
            var playerAfter=new java.util.ArrayList<net.minecraft.world.item.ItemStack>();
            for(int i=0;i<p.getInventory().getContainerSize();i++)playerAfter.add(p.getInventory().getItem(i).copy());
            com.google.gson.JsonArray after;
            if(selected.container==p.getInventory()) {
                after=vaultInsert(contents(),rows,new net.scopenet.core.Item(stackJson(0,stack).get("item").getAsString(),stack.getHoverName().getString(),stack.getCount(),stack.save(new net.minecraft.nbt.CompoundTag()).toString()));
                if(after==null){p.sendSystemMessage(Component.literal("This vault has no room for that stack."));return;}
                playerAfter.set(selected.getContainerSlot(),net.minecraft.world.item.ItemStack.EMPTY);
            } else if(selected.container==container) {
                net.minecraft.world.SimpleContainer box=new net.minecraft.world.SimpleContainer(36);
                for(int i=0;i<36;i++)box.setItem(i,playerAfter.get(i).copy());
                if(!fits(box,stack)){p.sendSystemMessage(Component.literal("Your inventory has no room for that stack."));return;}
                box.addItem(stack.copy());for(int i=0;i<36;i++)playerAfter.set(i,box.getItem(i).copy());
                after=new com.google.gson.JsonArray();for(var e:contents())if(e.getAsJsonObject().get("slot").getAsInt()!=selected.getContainerSlot())after.add(e);
            } else return;
            busy=true;menu.broadcastFullState();
            transfer.begin(p.getUUID(),after,id->{
                if(server.getPlayerList().getPlayer(p.getUUID())!=p||!before.equals(p.getInventory().save(new net.minecraft.nbt.ListTag())))return false;
                for(int i=0;i<playerAfter.size();i++)p.getInventory().setItem(i,playerAfter.get(i).copy());
                ((InventoryCheckpoint)p).velora$checkpoint(id);
                try {persistCheckpoint(p);}
                catch(java.io.IOException failure){p.connection.disconnect(Component.literal("Inventory storage interrupted. Reconnect to recover your transfer."));throw new IllegalStateException(failure);}
                return true;
            },committed->{busy=false;menu.broadcastFullState();if(!committed)p.sendSystemMessage(Component.literal("Inventory changed or vault is still saving. Your items were not moved; try again."));});
        }

        @Override public boolean deposit(net.scopenet.core.Item item) {
            var stack = FabricItems.fromCore(item);
            if (stack.isEmpty() || !fits(container, stack)) return false;
            return container.addItem(stack.copy()).isEmpty();
        }

        @Override public void closeAll() {
            for (ServerPlayer p : server.getPlayerList().getPlayers())
                if (p.containerMenu instanceof net.minecraft.world.inventory.ChestMenu menu && menu.getContainer() == container) p.closeContainer();
        }
    }

    @Override public VaultView createVault(String title, int rows, com.google.gson.JsonArray contents, Runnable changed,
                                           java.util.function.Consumer<UUID> closed, UUID firstViewer) {
        int size = Math.max(1, Math.min(7, rows)) * 9;
        net.minecraft.world.SimpleContainer container = new net.minecraft.world.SimpleContainer(size);
        ServerPlayer viewer = server.getPlayerList().getPlayer(firstViewer);
        for (var element : contents) {
            var entry = element.getAsJsonObject();
            var stack = stackOf(entry);
            int slot = entry.get("slot").getAsInt();
            if (stack.isEmpty()) continue;
            if (slot >= 0 && slot < size && container.getItem(slot).isEmpty()) container.setItem(slot, stack);
            else {
                if(viewer!=null)viewer.sendSystemMessage(Component.literal("Stored items exceed this vault's configured size. Ask staff to restore its rows before opening it."));
                return null;
            }
        }
        container.addListener(c -> changed.run());
        return new CloudView(container, size / 9, title, closed);
    }

    @Override public boolean showVault(UUID id, VaultView view) {
        ServerPlayer p = server.getPlayerList().getPlayer(id);
        if (p == null || !(view instanceof CloudView v)) return false;
        if (v.rows == 7 && !net.fabricmc.fabric.api.networking.v1.ServerPlayNetworking.canSend(p, new ResourceLocation("scopenet", "s2c"))) return false;
        p.openMenu(new net.minecraft.world.SimpleMenuProvider((windowId, inventory, player) -> {
            if (v.rows == 7) return new net.scopenet.core.vault.VaultMenu(windowId, inventory, v.container) {
                @Override public void clicked(int slot,int button,net.minecraft.world.inventory.ClickType type,net.minecraft.world.entity.player.Player who){v.click(this,slot,button,type,who);}
                @Override public void removed(net.minecraft.world.entity.player.Player who) { super.removed(who); v.closed.accept(who.getUUID()); }
            };
            return new net.minecraft.world.inventory.ChestMenu(CHEST_MENUS[v.rows - 1], windowId, inventory, v.container, v.rows) {
                @Override public void clicked(int slot,int button,net.minecraft.world.inventory.ClickType type,net.minecraft.world.entity.player.Player who){v.click(this,slot,button,type,who);}
                @Override public void removed(net.minecraft.world.entity.player.Player who) { super.removed(who); v.closed.accept(who.getUUID()); }
            };
        }, Component.literal(v.title)));
        return true;
    }
    private void persistCheckpoint(ServerPlayer player) throws java.io.IOException {
        java.nio.file.Path file=server.getWorldPath(net.minecraft.world.level.storage.LevelResource.PLAYER_DATA_DIR).resolve(player.getUUID()+".dat");
        java.nio.file.Files.createDirectories(file.getParent());
        java.nio.file.Path temporary=file.resolveSibling(file.getFileName()+".velora-"+java.util.UUID.randomUUID()+".tmp");
        var tag=player.saveWithoutId(new net.minecraft.nbt.CompoundTag());
        tag.putInt("DataVersion",net.minecraft.SharedConstants.getCurrentVersion().getDataVersion().getVersion());
        try {
            net.minecraft.nbt.NbtIo.writeCompressed(tag,temporary.toFile());
            try(var channel=java.nio.channels.FileChannel.open(temporary,java.nio.file.StandardOpenOption.WRITE)){channel.force(true);}
            java.nio.file.Files.move(temporary,file,java.nio.file.StandardCopyOption.ATOMIC_MOVE,java.nio.file.StandardCopyOption.REPLACE_EXISTING);
            try(var channel=java.nio.channels.FileChannel.open(file,java.nio.file.StandardOpenOption.WRITE)){channel.force(true);}
        } finally {java.nio.file.Files.deleteIfExists(temporary);}
    }
    @Override public String inventoryCheckpoint(UUID id){ServerPlayer p=server.getPlayerList().getPlayer(id);if(p==null)throw new IllegalStateException("player disconnected before recovery");return ((InventoryCheckpoint)p).velora$checkpoint();}
    @Override public void lockInventory(UUID id,boolean locked){InventoryLocks.set(id,locked);}

    @Override public com.google.gson.JsonArray vaultInsert(com.google.gson.JsonArray contents, int rows, net.scopenet.core.Item item) {
        int size = Math.max(1, Math.min(7, rows)) * 9;
        net.minecraft.world.SimpleContainer box = new net.minecraft.world.SimpleContainer(size);
        com.google.gson.JsonArray kept = new com.google.gson.JsonArray();
        for (var element : contents) {
            var entry = element.getAsJsonObject();
            int slot = entry.get("slot").getAsInt();
            var stack = stackOf(entry);
            if (slot >= 0 && slot < size && !stack.isEmpty() && box.getItem(slot).isEmpty()) box.setItem(slot, stack);
            else kept.add(entry); // beyond a shrunken vault: left exactly as stored
        }
        var incoming = FabricItems.fromCore(item);
        if (incoming.isEmpty() || !fits(box, incoming)) return null;
        if (!box.addItem(incoming.copy()).isEmpty()) return null;
        com.google.gson.JsonArray out = contentsOf(box);
        kept.forEach(out::add);
        return out;
    }

    @Override public com.google.gson.JsonArray localVaults(UUID owner) {
        java.io.File file = vaultFile(owner);
        if (!file.exists()) return null;
        try {
            net.minecraft.nbt.CompoundTag root = net.minecraft.nbt.NbtIo.readCompressed(file);
            com.google.gson.JsonArray vaults = new com.google.gson.JsonArray();
            for (String key : root.getAllKeys()) {
                if (!key.matches("v[0-9]{1,2}")) continue;
                int number = Integer.parseInt(key.substring(1));
                if (number < 1 || number > 54) continue;
                com.google.gson.JsonArray contents = new com.google.gson.JsonArray();
                java.util.Set<Integer> used = new java.util.HashSet<>();
                net.minecraft.nbt.ListTag saved = root.getList(key, 10);
                for (int i = 0; i < saved.size(); i++) {
                    net.minecraft.nbt.CompoundTag entry = saved.getCompound(i);
                    var stack = net.minecraft.world.item.ItemStack.of(entry);
                    int slot = entry.getInt("VSlot");
                    if (stack.isEmpty() || slot < 0 || slot >= 63 || !used.add(slot)) continue;
                    contents.add(stackJson(slot, stack));
                }
                com.google.gson.JsonObject vault = new com.google.gson.JsonObject();
                vault.addProperty("number", number);
                vault.add("contents", contents);
                vaults.add(vault);
            }
            return vaults;
        } catch (java.io.IOException e) {
            throw new IllegalStateException("local vault file for " + owner + " could not be read: " + e.getMessage(), e);
        }
    }

    @Override public void localVaultsMigrated(UUID owner) {
        java.io.File file = vaultFile(owner);
        if (!file.exists()) return;
        java.io.File kept = new java.io.File(file.getParentFile(), file.getName() + ".migrated");
        if (!file.renameTo(kept)) System.err.println("[Velora Core] Could not set aside migrated vault file " + file);
    }

    private long nextPlayerSave;

    @Override public boolean canPlaceOutpostFlag(Pos pos) {
        ServerLevel world = level(pos.world());
        BlockPos at = new BlockPos(pos.blockX(), pos.blockY(), pos.blockZ());
        return world != null && world.hasChunkAt(at) && world.getWorldBorder().isWithinBounds(at)
                && !world.isOutsideBuildHeight(at) && world.isEmptyBlock(at)
                && world.getBlockState(at.below()).blocksMotion();
    }
    @Override public boolean placeOutpostFlag(Pos pos) {
        if (!canPlaceOutpostFlag(pos)) return false;
        return level(pos.world()).setBlock(new BlockPos(pos.blockX(), pos.blockY(), pos.blockZ()),
                net.minecraft.world.level.block.Blocks.WHITE_BANNER.defaultBlockState(), 3);
    }

    /** Coalesced: one player-data save at most every two seconds, however many vault saves land. */
    @Override public void savePlayer(UUID id) {
        long now = System.currentTimeMillis();
        if (now < nextPlayerSave) return;
        nextPlayerSave = now + 2_000;
        server.execute(() -> server.getPlayerList().saveAll());
    }

    /** Check the whole delivery before mutating a live vault; partial inserts must not be retried as full stacks. */
    private static boolean fits(net.minecraft.world.Container container, net.minecraft.world.item.ItemStack incoming) {
        int capacity = 0;
        for (int slot = 0; slot < container.getContainerSize(); slot++) {
            var existing = container.getItem(slot);
            if (existing.isEmpty()) capacity += Math.min(container.getMaxStackSize(), incoming.getMaxStackSize());
            else if (net.minecraft.world.item.ItemStack.isSameItemSameTags(existing, incoming))
                capacity += Math.max(0, Math.min(container.getMaxStackSize(), existing.getMaxStackSize()) - existing.getCount());
            if (capacity >= incoming.getCount()) return true;
        }
        return false;
    }
}
