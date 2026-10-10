package net.velora.fabric.features;

import com.google.gson.JsonArray;
import com.mojang.brigadier.CommandDispatcher;
import com.mojang.brigadier.arguments.StringArgumentType;
import com.mojang.brigadier.builder.LiteralArgumentBuilder;
import com.mojang.brigadier.context.CommandContext;
import com.mojang.brigadier.suggestion.Suggestions;
import com.mojang.brigadier.suggestion.SuggestionsBuilder;
import net.fabricmc.fabric.api.entity.event.v1.ServerLivingEntityEvents;
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerLifecycleEvents;
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerTickEvents;
import net.fabricmc.fabric.api.networking.v1.PacketByteBufs;
import net.fabricmc.fabric.api.networking.v1.ServerPlayConnectionEvents;
import net.fabricmc.fabric.api.networking.v1.ServerPlayNetworking;
import net.minecraft.commands.CommandSourceStack;
import net.minecraft.commands.Commands;
import net.fabricmc.fabric.api.event.player.UseBlockCallback;
import net.minecraft.core.BlockPos;
import net.minecraft.network.FriendlyByteBuf;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.InteractionResult;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.ResourceLocation;
import net.minecraft.server.MinecraftServer;
import net.minecraft.server.level.ServerPlayer;
import net.velora.core.*;
import net.velora.core.map.MapService;
import net.velora.fabric.api.VeloraEconomy;
import net.scopenet.integration.Integration;
import net.scopenet.integration.Settings;
import net.velora.minecraft.Bridge;

import java.nio.file.Path;
import java.util.*;
import java.util.concurrent.CompletableFuture;
import java.util.logging.Logger;

/**
 * Everything beyond sign-in on Fabric 1.20.1: the shared commands (homes, warps, TPA, RTP, economy, guilds…), LuckPerms
 * permissions, Placeholder API, the economy API for other mods, and the link to the optional client mod.
 * Runs when Fabric API is installed; all behaviour comes from {@code net.velora.core}.
 */
public final class FabricFeatures implements Runnable {
    private static final Logger LOG = Logger.getLogger("Velora");
    private static final ResourceLocation TO_CLIENT = new ResourceLocation(Wire.S2C);
    private static final ResourceLocation FROM_CLIENT = new ResourceLocation(Wire.C2S);

    private MinecraftServer server;
    private FabricPlatform platform;
    private Env env;
    private CommandSet commands;
    private PlayerCache cache;
    private ClientLink link;
    private Perms perms;
    private FabricPermissions permissions;
    private PhysicalShops physicalShops;
    private final ThreadLocal<Boolean> formattingChat=ThreadLocal.withInitial(()->false);
    private FabricConfig config;
    private ActionPoller poller;
    private net.velora.core.RewardPoller rewards;
    private ResourcePackPoller packs;
    private MapService map;
    private int ticks;
    private String placeholderStatus = "off";

    CommandSet commandSet() { return commands; }

    @Override public void run() {
        net.fabricmc.fabric.api.message.v1.ServerMessageEvents.ALLOW_CHAT_MESSAGE.register((message,sender,params)-> {
            Integration integration=Bridge.integration();
            if(formattingChat.get()||env==null||integration==null||!env.modules.get().enabled("permissions_chat")||!integration.chat().enabled())return true;
            var registry=sender.level().registryAccess().registryOrThrow(net.minecraft.core.registries.Registries.CHAT_TYPE);
            var key=net.minecraft.resources.ResourceKey.create(net.minecraft.core.registries.Registries.CHAT_TYPE,new ResourceLocation("velora_core","core"));
            if(params.chatType()!=registry.get(net.minecraft.network.chat.ChatType.CHAT.location())||registry.get(key.location())==null||!message.filterMask().isEmpty())return true;
            var info=cache.info(sender.getUUID());
            var guild=info==null?null:PlayerCache.obj(info,"guild");var global=info==null?null:PlayerCache.obj(info,"global");
            String[] meta=Perms.luckPermsPresent()?LuckPermsLookup.meta(sender):new String[]{"","",""};
            var parts=new net.scopenet.integration.ChatLayout.Parts(PlayerCache.str(guild,"tag",""),PlayerCache.str(guild,"name",""),PlayerCache.str(global,"title",""),PlayerCache.str(global,"title_glyph",""),meta[0],meta[1],meta[2],(int)PlayerCache.num(global,"level",0));
            String text=String.format(java.util.Locale.ROOT,integration.chat().render(parts),sender.getGameProfile().getName(),integration.chat().message(message.decoratedContent().getString(),perms.has(sender,"velora.chat.colors")));
            formattingChat.set(true);
            try {sender.getServer().getPlayerList().broadcastChatMessage(message.withUnsignedContent(LegacyText.component(text)),sender,net.minecraft.network.chat.ChatType.bind(key,sender));}
            finally {formattingChat.set(false);}
            return false;
        });
        ServerLifecycleEvents.SERVER_STARTED.register(this::started);
        ServerLifecycleEvents.SERVER_STOPPING.register(s -> stopping());
        ServerTickEvents.END_SERVER_TICK.register(s -> tick());
        ServerPlayConnectionEvents.DISCONNECT.register((handler, s) -> {
            InventoryLocks.set(handler.getPlayer().getUUID(),false);
            if (link != null) link.disconnected(handler.getPlayer().getUUID());
            if (commands != null) commands.utilityHub.left(handler.getPlayer().getUUID());
            if (packs != null) packs.left(handler.getPlayer().getUUID());
        });
        ServerPlayConnectionEvents.JOIN.register((handler,sender,s)->{if(commands!=null)commands.utilityHub.cloudVaults.recoverPlayer(handler.getPlayer().getUUID());});
        // Fall damage is waived for a moment after flight is switched off, so leaving your land mid-air isn't a trap.
        ServerLivingEntityEvents.ALLOW_DAMAGE.register((entity, source, amount) ->
                !(entity instanceof ServerPlayer sp && (InventoryLocks.locked(sp.getUUID()) || source.is(net.minecraft.tags.DamageTypeTags.IS_FALL) && platform != null && platform.waiveFall(sp.getUUID()))));
        // /back also returns you to where you died.
        ServerLivingEntityEvents.ALLOW_DEATH.register((entity, source, amount) -> {
            if(entity instanceof ServerPlayer p&&InventoryLocks.locked(p.getUUID()))return false;
            if (entity instanceof ServerPlayer p && commands != null) commands.essentials.remember(p.getUUID(), platform.wrap(p).pos());
            return true;
        });
        // Mob kills count toward kill contracts (the same "mob_kills:ZOMBIE" action Paper reports).
        ServerLivingEntityEvents.AFTER_DEATH.register((entity, source) -> {
            Integration integration = Bridge.integration();
            if (integration == null || entity instanceof ServerPlayer || !(source.getEntity() instanceof ServerPlayer killer)) return;
            String type = net.minecraft.core.registries.BuiltInRegistries.ENTITY_TYPE.getKey(entity.getType()).getPath().toUpperCase(java.util.Locale.ROOT);
            integration.activity.action(killer.getUUID(), killer.getGameProfile().getName(), "mob_kills:" + type, 1);
        });
        // Right-clicking a shop point opens the market or the shop.
        UseBlockCallback.EVENT.register((player, level, hand, hit) -> {
            if (level.isClientSide || hand != InteractionHand.MAIN_HAND || !(player instanceof ServerPlayer sp) || commands == null) return InteractionResult.PASS;
            BlockPos at = hit.getBlockPos();
            if(physicalShops!=null&&physicalShops.use(sp,at))return InteractionResult.SUCCESS;
            Optional<ShopPoints.Shop> shop = commands.shops.at(level.dimension().location().toString(), at.getX(), at.getY(), at.getZ());
            if (shop.isEmpty()) return InteractionResult.PASS;
            CorePlayer p = platform.wrap(sp);
            String kind = shop.get().kind().equals("market") ? "market" : "shop";
            if (!link.open(p, kind)) commands.run(kind, p);
            return InteractionResult.SUCCESS;
        });
        ServerPlayNetworking.registerGlobalReceiver(FROM_CLIENT, (s, player, handler, buf, sender) -> {
            byte[] data = new byte[buf.readableBytes()];
            buf.readBytes(data);
            s.execute(() -> { if (link != null) link.receive(platform.wrap(player), data); });
        });
    }

    // ---- lifecycle -------------------------------------------------------------------------

    private void started(MinecraftServer s) {
        Integration integration = Bridge.integration();
        if (integration == null) { LOG.warning("Velora is not connected to a panel; commands are off."); return; }
        server = s;
        config = FabricConfig.load();
        perms = new Perms(config.everyoneByDefault());
        platform = new FabricPlatform(s, perms);
        Path dir = Path.of("config", "velora-core");
        env = new Env(platform, new PanelAdapter(integration), config.features(integration.settings()), dir, System::currentTimeMillis, new Random(), LOG);
        env.utilities = integration::utilities;
        env.modules = () -> config.modules().intersect(integration.modules());
        if(Perms.luckPermsPresent())permissions=new FabricPermissions(env,()->config.bool("integrations.luckperms.apply-panel-groups",false));
        commands = new CommandSet(env, config.essentials());
        physicalShops = new PhysicalShops(env,commands,platform,s);
        cache = new PlayerCache(env);
        link = new ClientLink(env, cache, s.getMotd(), (player, bytes) -> {
            FriendlyByteBuf buf = PacketByteBufs.create();
            buf.writeBytes(bytes);
            ServerPlayer target = s.getPlayerList().getPlayer(player.uuid());
            if (target != null && ServerPlayNetworking.canSend(target, TO_CLIENT)) ServerPlayNetworking.send(target, TO_CLIENT, buf);
        });
        link.panelUrl(integration.settings().panel().toString());
        link.localRequests((p,msg)->physicalShops.request(p,msg,link));
        physicalShops.link(link);
        integration.onNotifications(events -> { link.onPanelEvents(events); net.velora.core.PanelMessages.deliver(platform, events); });
        Bridge.claimBypass = p -> perms.has(p, "velora.claims.bypass");

        // Actions from the map, and the claims and pins the Velora Map draws.
        poller = new ActionPoller(env, (from, to) -> commands.essentials.tpa(from, new String[]{to.name()}));
        rewards = new net.velora.core.RewardPoller(env, config.rewards());
        packs = new ResourcePackPoller(env, () -> integration.settings().panel().toString());
        map = new MapService(env, integration.client().claims(), commands.mapSource(), cache, integration.settings().panel().toString(),
                config.bool("map.show_homes", false), s.getMotd());
        integration.setMapOverlay(() -> { MapService current = map; return current == null ? "{\"claims\":[],\"pins\":[]}" : net.velora.core.map.MapOverlay.toJson(current.snapshot()).toString(); });
        registerCommands(s.getCommands().getDispatcher());
        registerPlaceholders();
        VeloraEconomy.install(new VeloraEconomy(env.panel, r -> platform.runAsync(r)));
        LOG.info("Velora commands ready (" + commands.all().size() + " commands, "
                + (Perms.luckPermsPresent() ? "LuckPerms" : "default permissions") + ").");
    }

    private void stopping() {
        // Vaults are saved and released before the I/O pool stops.
        if (commands != null) commands.utilityHub.cloudVaults.shutdown();
        VeloraEconomy.uninstall();
        Bridge.claimBypass = p -> false;
        if (platform != null) platform.shutdown();
        commands = null;
        map = null;
    }

    private void registerPlaceholders() {
        if (!config.bool("placeholders.enabled", true)) { placeholderStatus = "off (config)"; return; }
        if (!net.fabricmc.loader.api.FabricLoader.getInstance().isModLoaded("placeholder-api")) { placeholderStatus = "Placeholder API not installed"; return; }
        try {
            int n = PlaceholderApiModule.register(new Placeholders(env, cache));
            placeholderStatus = "active (" + n + ")";
        } catch (Throwable t) {
            placeholderStatus = "failed: " + t;
            LOG.warning("Placeholder API could not be used: " + t);
        }
    }

    private void tick() {
        if (commands == null) return;
        ticks++;
        if (ticks % 20 == 0) {
            if(permissions!=null)permissions.tick();
            if(physicalShops!=null)physicalShops.tick();
            CoreModules modules = env.modules.get();
            Features configured = config.features(Bridge.integration().settings());
            env.features = new Features(configured.essentials(), configured.economy() && modules.enabled("economy"),
                    configured.guilds() && modules.enabled("factions"), configured.social(), configured.landClaiming() && modules.enabled("factions"),
                    configured.clientLink(), configured.currencySymbol());
            cache.tick(p -> link.pushState(p));
            link.tick();
            poller.tick();
            rewards.tick();
            packs.tick();
            map.tick();
        }
        if (ticks % 40 == 0) commands.tick();
        if (ticks % 20 == 0) commands.utilityHub.tick();
    }

    // ---- commands --------------------------------------------------------------------------

    private void registerCommands(CommandDispatcher<CommandSourceStack> dispatcher) {
        dispatcher.register(Commands.literal("pshop").executes(ctx->{if(ctx.getSource().getPlayer()!=null)physicalShops.command(ctx.getSource().getPlayer(),"");return 1;})
            .then(Commands.argument("args",StringArgumentType.greedyString()).executes(ctx->{if(ctx.getSource().getPlayer()!=null)physicalShops.command(ctx.getSource().getPlayer(),StringArgumentType.getString(ctx,"args"));return 1;})));
        List<CoreCommand> all = new ArrayList<>(commands.all());
        all.add(new VeloraCommand(this));
        for (CoreCommand command : all) {
            List<String> names = new ArrayList<>();
            names.add(command.name());
            names.addAll(command.aliases());
            for (String name : names) {
                LiteralArgumentBuilder<CommandSourceStack> root = Commands.literal(name)
                        .executes(ctx -> execute(ctx, command, ""))
                        .then(Commands.argument("args", StringArgumentType.greedyString())
                                .suggests((ctx, builder) -> suggest(ctx, command, builder))
                                .executes(ctx -> execute(ctx, command, StringArgumentType.getString(ctx, "args"))));
                dispatcher.register(root);
            }
        }
    }

    private int execute(CommandContext<CommandSourceStack> ctx, CoreCommand command, String argText) {
        ServerPlayer sp = ctx.getSource().getPlayer();
        if (sp == null) { ctx.getSource().sendFailure(Component.literal("Only players can use this command.")); return 0; }
        String[] args = argText.isBlank() ? new String[0] : argText.trim().split("\\s+");
        CorePlayer player = platform.wrap(sp);
        try {
            command.run(player, args);
        } catch (RuntimeException e) {
            LOG.warning("/" + command.name() + " failed: " + e);
            player.send(Format.RED + "Something went wrong running that command.");
            return 0;
        }
        return 1;
    }

    private CompletableFuture<Suggestions> suggest(CommandContext<CommandSourceStack> ctx, CoreCommand command, SuggestionsBuilder builder) {
        ServerPlayer sp = ctx.getSource().getPlayer();
        if (sp != null) {
            String remaining = builder.getRemaining();
            int lastSpace = remaining.lastIndexOf(' ');
            SuggestionsBuilder argument = builder.createOffset(builder.getStart() + lastSpace + 1);
            String[] typed = remaining.split("\\s+", -1);
            for (String candidate : CommandSuggestions.filter(command.complete(platform.wrap(sp), typed), typed)) argument.suggest(candidate);
            return argument.buildFuture();
        }
        return builder.buildFuture();
    }

    // ---- /velora -------------------------------------------------------------------------

    void sendStatus(CorePlayer p) {
        Integration integration = Bridge.integration();
        Settings s = integration.settings();
        p.send(Format.GOLD + "=== Velora Status ===");
        p.send(Format.GRAY + "Panel: " + Format.WHITE + s.panel() + Format.GRAY + "   Token: " + (s.token().isEmpty() ? Format.RED + "missing" : Format.GREEN + "set"));
        Features f = env.features;
        p.send(Format.GRAY + "Essentials: " + flag(f.essentials()) + Format.GRAY + "  Economy: " + flag(f.economy()) + Format.GRAY + "  Guilds: " + flag(f.guilds())
                + Format.GRAY + "  Claims: " + flag(f.landClaiming()) + Format.GRAY + "  Client link: " + flag(f.clientLink()));
        p.send(Format.GRAY + "Claim index: " + (integration.client().claims().loaded() ? Format.GREEN + integration.client().claims().claimCount() + " claims" : Format.YELLOW + "loading"));
        p.send(Format.GRAY + "Permissions: " + Format.WHITE + (Perms.luckPermsPresent() ? "LuckPerms" : "defaults (" + (config.everyoneByDefault() ? "everyone" : "operators") + ")"));
        p.send(Format.GRAY + "Placeholder API: " + Format.WHITE + placeholderStatus);
        p.send(Format.GRAY + "Pending transactions: " + Format.WHITE + commands.jobs.pending());
    }

    private static String flag(boolean on) { return on ? Format.GREEN + "on" : Format.RED + "off"; }

    void reload(CorePlayer p) {
        try {
            Settings next = Settings.load(FabricConfig.FILE);
            Bridge.integration().reload(next);
            config = FabricConfig.load();
            perms = new Perms(config.everyoneByDefault());
            env.features = config.features(next);
            commands.essentials.configure(config.essentials());
            p.send(Format.GREEN + "Velora config reloaded.");
        } catch (Exception e) {
            p.send(Format.RED + "Reload failed: " + e.getMessage());
        }
    }
}
