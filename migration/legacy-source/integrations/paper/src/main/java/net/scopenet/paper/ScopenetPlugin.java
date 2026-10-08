package net.scopenet.paper;

import net.scopenet.api.ScopenetApi;
import net.scopenet.api.ScopenetApiProvider;
import net.scopenet.integration.*;
import net.scopenet.paper.compat.CompatManager;
import net.scopenet.paper.compat.EventBridge;
import net.scopenet.paper.compat.ScopenetApiImpl;
import net.scopenet.paper.commands.*;
import org.bukkit.*;
import org.bukkit.entity.Player;
import org.bukkit.block.Block;
import org.bukkit.event.*;
import org.bukkit.event.block.*;
import org.bukkit.event.entity.*;
import org.bukkit.event.player.*;
import org.bukkit.event.inventory.InventoryClickEvent;
import org.bukkit.plugin.java.JavaPlugin;
import java.util.concurrent.TimeUnit;
import java.util.UUID;

public final class ScopenetPlugin extends JavaPlugin implements Listener {
    private Integration integration;
    private EconomyHandler economy;
    private ScopenetApiImpl api;
    private CompatManager compat;
    private ClientLinkBridge clientLink;
    private EssentialsHandler essentialsHandler;
    private GuildHandler guildHandler;
    private ChatFormatter chatFormatter;
    private UtilityBridge utilities;
    private ContentRuntime contentRuntime;

    @Override public void onEnable() {
        // Install the deny-by-default listener even if configuration is invalid.
        getServer().getPluginManager().registerEvents(this, this);
        saveDefaultConfig();
        try {
            Settings settings = readSettings();

            integration = new Integration(
                    settings,
                    getServer().getName(), Bukkit.getBukkitVersion().split("-")[0], getDescription().getVersion(),
                    Bukkit.getOnlineMode(), Bukkit.getMaxPlayers(), getLogger()::warning,
                    (id, message) -> {
                        if (!isEnabled()) return;
                        Bukkit.getScheduler().runTask(this, () -> {
                            Player player = Bukkit.getPlayer(id);
                            if (player != null) player.kickPlayer(message);
                        });
                    });
            Bukkit.getScheduler().runTaskTimer(this, integration::tick, 1, 1);
            // Claim protection runs from a local copy of the panel's claims, refreshed in the background.
            integration.startClaimSync();

            // The SCOPENET Map: renders the world to tiles and sends them, positions, claims and pins to the panel.
            // Started on the first tick so the worlds exist.
            if (getConfig().getBoolean("map.enabled", getConfig().getBoolean("livemap.enabled", true))) {
                Bukkit.getScheduler().runTask(this, () -> {
                    if (integration == null || Bukkit.getWorlds().isEmpty()) return;
                    integration.enableMap(Bukkit.getWorlds().get(0).getWorldFolder().toPath(), getDataFolder().toPath().resolve("map"));
                    Bukkit.getScheduler().runTaskTimer(this, this::snapshotPlayers, 40, 20);
                });
                getServer().getPluginManager().registerEvents(new org.bukkit.event.Listener() {
                    @org.bukkit.event.EventHandler public void onSave(org.bukkit.event.world.WorldSaveEvent event) {
                        if (integration != null) integration.mapSaved();
                    }
                }, this);
            }

            // Developer API: other plugins use SCOPENET through ScopenetApiProvider / the services manager,
            // and receive level-ups, achievements and guild changes as ordinary Bukkit events.
            api = new ScopenetApiImpl(integration.client());
            chatFormatter = new ChatFormatter(api, integration::chat);
            ScopenetApiProvider.register(api);
            getServer().getServicesManager().register(ScopenetApi.class, api, this, org.bukkit.plugin.ServicePriority.Normal);
            // Optional client mod (HUD, claim overlay, shop and market windows).
            clientLink = new ClientLinkBridge(this, integration);
            clientLink.start();
            // /heal /feed /fly /vault /echest /kit /customitem /adminclaim and the claim banner.
            utilities = new UtilityBridge(this, integration);
            utilities.start();
            utilities.cosmeticsChanged(clientLink::refreshCosmetics);
            // Blocks, chests, decorations, NPCs, vehicles, crops and mobs from the Content Studio.
            contentRuntime = new ContentRuntime(this, integration::utilities, utilities::stack);
            contentRuntime.dialogues(clientLink::dialogue);
            contentRuntime.start();
            // Modelled cosmetics from the Cosmetics Studio, shown on the player who equipped them.
            if (getConfig().getBoolean("cosmetics.enabled", true)) {
                CosmeticRuntime cosmetics = new CosmeticRuntime(this, utilities::stack);
                cosmetics.start();
                clientLink.cosmetics(cosmetics);
            }
            EventBridge events = new EventBridge(this, api, getLogger());
            net.scopenet.core.Platform chatPlatform = new UtilityBridge.ChatPlatform(this);
            integration.onNotifications(list -> { events.accept(list); clientLink.panelEvents(list); net.scopenet.core.PanelMessages.deliver(chatPlatform, list); });
            Bukkit.getScheduler().runTaskTimer(this, this::warmProfiles, 100, 20 * 20);

            // Optional integrations (LuckPerms, PlaceholderAPI, Vault, CoreProtect, WorldGuard, Spark) are found and
            // switched on by the compat manager once every plugin has loaded.
            compat = new CompatManager(this, integration, api);
            Bukkit.getScheduler().runTask(this, compat::start);

            // Register /scopenet admin & help command
            ScopenetCommandHandler scopenetCmd = new ScopenetCommandHandler(this, integration, () -> compat);
            bindCommand("scopenet", scopenetCmd);
            bindCommand("map", scopenetCmd);

            // Register Essentials commands & GUI
            {
                EssentialsHandler essentials = new EssentialsHandler(this);
                essentialsHandler = essentials;
                getServer().getPluginManager().registerEvents(essentials, this);
                String[] essCmds = {"spawn", "home", "sethome", "delhome", "back", "tpa", "tpaccept", "tpdeny", "rtp", "warp", "reqwarp", "warprequests", "warpapprove", "warpdeny", "setwarp", "delwarp", "playtime"};
                for (String c : essCmds) bindCommand(c, essentials);
            }
            HandHandler hand = new HandHandler(this);
            getServer().getPluginManager().registerEvents(hand, this);
            bindCommand("hand", hand);
            bindCommand("handview", hand);
            NicknameHandler nick = new NicknameHandler(this);
            getServer().getPluginManager().registerEvents(nick, this);
            bindCommand("nick", nick);

            // Register Economy commands & GUI
            {
                economy = new EconomyHandler(this, integration);
                getServer().getPluginManager().registerEvents(economy, this);
                String[] econCmds = {"balance", "pay", "baltop", "shop", "sell", "market", "orders", "contracts", "trade", "transactions"};
                for (String c : econCmds) bindCommand(c, economy);
            }

            // Register Guilds & Land Claims commands & GUI
            {
                GuildHandler guilds = new GuildHandler(this, integration, economy);
                guildHandler = guilds;
                if (utilities != null) guilds.useManager(utilities::guildManage);
                getServer().getPluginManager().registerEvents(guilds, this);
                bindCommand("guild", guilds);
                bindCommand("claim", guilds);
                bindCommand("unclaim", guilds);
                bindCommand("autoclaim", guilds);
            }

            // Physical shops, actions from the map, and the claims and pins the SCOPENET Map draws.
            clientLink.startMap(essentialsHandler, guildHandler, economy);

            // Reloading an auth plugin cannot silently admit existing sessions.
            for (Player player : Bukkit.getOnlinePlayers()) player.kickPlayer("SCOPENET restarted. Please reconnect.");
            getLogger().info("SCOPENET integration initialized. Configured modules:");
            getLogger().info(" - Leveling: " + (settings.levelingEnabled() ? "ENABLED" : "DISABLED"));
            getLogger().info(" - Quests: " + (settings.questsEnabled() ? "ENABLED" : "DISABLED"));
            getLogger().info(" - Achievements: " + (settings.achievementsEnabled() ? "ENABLED" : "DISABLED"));
            getLogger().info(" - Guilds & Land Claiming: " + (settings.guildsEnabled() && settings.landClaimingEnabled() ? "ENABLED" : "DISABLED"));
            getLogger().info(" - Social: " + (settings.socialEnabled() ? "ENABLED" : "DISABLED"));
            getLogger().info(" - Essentials: " + (settings.essentialsEnabled() ? "ENABLED" : "DISABLED"));
            getLogger().info(" - Economy: " + (settings.economyEnabled() ? "ENABLED" : "DISABLED"));
        } catch (Exception e) {
            getLogger().log(java.util.logging.Level.SEVERE, "SCOPENET is blocking joins", e);
            for (Player player : Bukkit.getOnlinePlayers()) player.kickPlayer(Integration.UNAVAILABLE);
        }
    }

    private Settings readSettings() {
        return Settings.of(
                    getConfig().getString("panel-url", ""),
                    getConfig().getString("token", ""),
                    getConfig().getBoolean("leveling.enabled", true),
                    getConfig().getDouble("leveling.global-xp-multiplier", 1.0),
                    getConfig().getDouble("leveling.server-xp-multiplier", 1.5),
                    getConfig().getBoolean("quests.enabled", true),
                    getConfig().getBoolean("achievements.enabled", true),
                    getConfig().getBoolean("guilds.enabled", true),
                    getConfig().getBoolean("guilds.land-claiming", true),
                    getConfig().getBoolean("social.enabled", true),
                    getConfig().getBoolean("social.chat-prefixes", true),
                    getConfig().getBoolean("essentials.enabled", true),
                    getConfig().getBoolean("economy.enabled", true)
            );
    }

    public void reloadIntegrationSettings() {
        reloadConfig();
        Settings next = readSettings();
        if (integration == null) throw new IllegalStateException("Restart the plugin after correcting its configuration");
        integration.reload(next);
    }

    private void snapshotPlayers() {
        if (integration == null) return;
        java.util.List<net.scopenet.worldmap.WorldMapSync.Player> list = new java.util.ArrayList<>();
        for (Player p : Bukkit.getOnlinePlayers()) {
            Location at = p.getLocation();
            String dim = switch (at.getWorld().getEnvironment()) {
                case NETHER -> "minecraft:the_nether";
                case THE_END -> "minecraft:the_end";
                default -> "minecraft:overworld";
            };
            // Rounded so an idle roster doesn't look "changed" on every tick.
            list.add(new net.scopenet.worldmap.WorldMapSync.Player(p.getUniqueId().toString(), p.getName(), dim,
                    Math.round(at.getX() * 10) / 10.0, Math.round(at.getY() * 10) / 10.0, Math.round(at.getZ() * 10) / 10.0,
                    Math.round(at.getYaw()), Math.round(at.getPitch())));
        }
        integration.mapPlayers(list);
    }

    public static String dimension(World world) {
        return world.getKey().toString();
    }

    /** {@code /market point ...}: manage shop points (blocks that open the market or shop when right-clicked). */
    public void shopCommand(Player player, String[] args) {
        if (clientLink != null) clientLink.shopCommand(player, args);
    }

    private void bindCommand(String name, org.bukkit.command.CommandExecutor executor) {
        org.bukkit.command.PluginCommand cmd = getCommand(name);
        if (cmd != null) {
            cmd.setExecutor(executor);
            if (executor instanceof org.bukkit.command.TabCompleter completer) cmd.setTabCompleter(completer);
            else cmd.setTabCompleter(new CommandCompletion());
        } else {
            getLogger().warning("Could not bind command '/" + name + "' - not found in plugin.yml");
        }
    }

    public boolean giveSpec(UUID id, net.scopenet.core.ItemSpec spec) { return utilities != null && utilities.giveSpec(id, spec); }

    /** Keep the API's cached profiles of online players fresh, for placeholders and getCachedPlayer(). */
    private void warmProfiles() {
        if (api == null) return;
        for (Player p : Bukkit.getOnlinePlayers()) api.peek(p.getUniqueId());
    }

    @Override public void onDisable() {
        if (compat != null) compat.stop();
        if (clientLink != null) clientLink.stop();
        if (utilities != null) utilities.stop();
        if (api != null) {
            getServer().getServicesManager().unregister(ScopenetApi.class, api);
            ScopenetApiProvider.unregister(api);
            api.close();
        }
        if (integration != null) integration.close();
    }

    @EventHandler(priority = EventPriority.HIGHEST)
    public void login(AsyncPlayerPreLoginEvent event) {
        if (event.getLoginResult() != AsyncPlayerPreLoginEvent.Result.ALLOWED) return;
        String denial = Integration.UNAVAILABLE;
        if (integration != null) {
            try {
                denial = integration.login(event.getUniqueId(), event.getName(), event.getAddress().getHostAddress())
                        .get(7, TimeUnit.SECONDS);
            } catch (InterruptedException e) { Thread.currentThread().interrupt(); }
            catch (Exception ignored) { /* fail closed */ }
        }
        if (denial != null) event.disallow(AsyncPlayerPreLoginEvent.Result.KICK_OTHER, denial);
    }

    @EventHandler(priority = EventPriority.MONITOR)
    public void join(PlayerJoinEvent event) {
        if (integration != null) {
            integration.activity.join(event.getPlayer().getUniqueId(), event.getPlayer().getName());
        }
    }

    @EventHandler(priority = EventPriority.MONITOR)
    public void leave(PlayerQuitEvent event) {
        deniedAt.remove(event.getPlayer().getUniqueId());
        if (flightRevoked.remove(event.getPlayer().getUniqueId())) event.getPlayer().setAllowFlight(true);
        if (api != null) api.forget(event.getPlayer().getUniqueId());
        if (clientLink != null) clientLink.quit(event.getPlayer());
        if (integration != null) integration.activity.leave(event.getPlayer().getUniqueId(), event.getPlayer().getName());
    }

    private void add(Player player, String stat) {
        if (integration != null) {
            integration.activity.add(player.getUniqueId(), player.getName(), stat, 1);
        }
    }

    private boolean claimsEnabled() {
        return integration != null && integration.settings().guildsEnabled() && integration.settings().landClaimingEnabled();
    }

    private final java.util.Map<UUID, Long> deniedAt = new java.util.concurrent.ConcurrentHashMap<>();

    /**
     * Would {@code player} be stopped from changing {@code block}? Answers from the
     * local claim index (no network, and block.getX() >> 4 rather than getChunk()
     * so a check never loads a chunk). Staff with scopenet.claims.bypass are exempt.
     */
    private ChunkCheckResult denial(Player player, Block block) { return denial(player, block, "build"); }

    /** Like {@link #denial(Player, Block)}, but an admin claim that allows {@code flag} (build, interact, containers) lets everyone through. */
    private ChunkCheckResult denial(Player player, Block block, String flag) {
        if (!claimsEnabled() || player.hasPermission("scopenet.claims.bypass")) return null;
        String dim = dimension(block.getWorld());
        ChunkCheckResult check = integration.checkChunk(dim, block.getX() >> 4, block.getZ() >> 4, player.getUniqueId());
        if (!check.claimed() || check.allowed()) return null;
        Boolean allowed = integration.claimFlag(dim, block.getX() >> 4, block.getZ() >> 4, flag);
        return allowed != null && allowed ? null : check;
    }

    /** May this happen at {@code where}? True anywhere an admin claim does not forbid it. Not used for wilderness or guild land. */
    private boolean flagAllows(Location where, String flag) {
        if (integration == null || where.getWorld() == null) return true;
        return integration.adminAllows(dimension(where.getWorld()), where.getBlockX() >> 4, where.getBlockZ() >> 4, flag);
    }

    /** Is {@code player} a member of the guild whose land {@code where} is? False for wilderness and admin claims. */
    private boolean memberOfClaim(Player player, Location where) {
        if (integration == null || where.getWorld() == null) return false;
        ChunkCheckResult check = integration.checkChunk(dimension(where.getWorld()), where.getBlockX() >> 4, where.getBlockZ() >> 4, player.getUniqueId());
        return check.claimed() && check.allowed();
    }

    /** Staff with scopenet.claims.bypass are never held back by a flag. */
    private boolean flagAllows(Player player, Location where, String flag) {
        return player.hasPermission("scopenet.claims.bypass") || flagAllows(where, flag);
    }

    /** Environmental changes: protected as before, unless an admin claim says this kind of change is allowed there. */
    private boolean claimedFor(Block block, String flag) {
        return claimed(block) && !flagAllows(block.getLocation(), flag);
    }

    /** One "claimed" notice per second per player, however many events fire. */
    private void tell(Player player, ChunkCheckResult check) {
        long now = System.currentTimeMillis();
        Long last = deniedAt.get(player.getUniqueId());
        if (last != null && now - last < 1000) return;
        deniedAt.put(player.getUniqueId(), now);
        player.sendMessage(ChatColor.RED + "This chunk is claimed by [" + check.guildTag() + "] " + check.guildName() + "!");
    }

    private boolean protectedFrom(Player player, Block block) { return protectedFrom(player, block, "build"); }

    private boolean protectedFrom(Player player, Block block, String flag) {
        ChunkCheckResult check = denial(player, block, flag);
        if (check == null) return false;
        tell(player, check);
        return true;
    }

    /** Environmental changes (fire, fluids, pistons, explosions) never touch claimed land. */
    private boolean claimed(Block block) {
        return claimsEnabled() && integration.isClaimed(dimension(block.getWorld()), block.getX() >> 4, block.getZ() >> 4);
    }

    @EventHandler(priority = EventPriority.NORMAL, ignoreCancelled = true)
    public void broken(BlockBreakEvent event) {
        if (protectedFrom(event.getPlayer(), event.getBlock())) {
            event.setCancelled(true);
            return;
        }
        add(event.getPlayer(), "blocks_broken");
        if (integration != null) integration.activity.action(event.getPlayer().getUniqueId(), event.getPlayer().getName(),
                "block_broken:" + event.getBlock().getType().name() + "@" + dimension(event.getBlock().getWorld()), 1);
    }

    @EventHandler(priority = EventPriority.NORMAL, ignoreCancelled = true)
    public void placed(BlockPlaceEvent event) {
        if (protectedFrom(event.getPlayer(), event.getBlock())) {
            event.setCancelled(true);
            return;
        }
        add(event.getPlayer(), "blocks_placed");
        if (integration != null) integration.activity.action(event.getPlayer().getUniqueId(), event.getPlayer().getName(),
                "block_placed:" + event.getBlock().getType().name() + "@" + dimension(event.getBlock().getWorld()), 1);
    }

    @EventHandler(priority = EventPriority.HIGHEST, ignoreCancelled = true)
    public void interact(PlayerInteractEvent event) {
        Block clicked = event.getClickedBlock();
        if (clicked == null || event.getAction() != Action.RIGHT_CLICK_BLOCK) return;
        // Admin claims can leave doors and buttons usable while chests stay shut, or the other way round.
        String flag = claimed(clicked) && clicked.getState() instanceof org.bukkit.block.Container ? "containers" : "interact";
        if (protectedFrom(event.getPlayer(), clicked, flag)) event.setCancelled(true);
    }

    @EventHandler(priority = EventPriority.HIGHEST, ignoreCancelled = true)
    public void bucketEmpty(PlayerBucketEmptyEvent event) {
        if (protectedFrom(event.getPlayer(), event.getBlockClicked().getRelative(event.getBlockFace()))) event.setCancelled(true);
    }

    @EventHandler(priority = EventPriority.HIGHEST, ignoreCancelled = true)
    public void bucketFill(PlayerBucketFillEvent event) {
        if (protectedFrom(event.getPlayer(), event.getBlockClicked())) event.setCancelled(true);
    }

    @EventHandler(priority = EventPriority.HIGHEST, ignoreCancelled = true)
    public void entityExplode(EntityExplodeEvent event) {
        event.blockList().removeIf(block -> claimedFor(block, "explosions"));
    }

    @EventHandler(priority = EventPriority.HIGHEST, ignoreCancelled = true)
    public void blockExplode(BlockExplodeEvent event) {
        event.blockList().removeIf(block -> claimedFor(block, "explosions"));
    }

    @EventHandler(priority = EventPriority.HIGHEST, ignoreCancelled = true)
    public void pistonExtend(BlockPistonExtendEvent event) {
        if (event.getBlocks().stream().anyMatch(block -> claimed(block) || claimed(block.getRelative(event.getDirection())))) event.setCancelled(true);
    }

    @EventHandler(priority = EventPriority.HIGHEST, ignoreCancelled = true)
    public void pistonRetract(BlockPistonRetractEvent event) {
        if (event.getBlocks().stream().anyMatch(block -> claimed(block) || claimed(block.getRelative(event.getDirection())))) event.setCancelled(true);
    }

    @EventHandler(priority = EventPriority.HIGHEST, ignoreCancelled = true)
    public void fluid(BlockFromToEvent event) {
        if (claimedFor(event.getToBlock(), "fluid_flow")) event.setCancelled(true);
    }

    @EventHandler(priority = EventPriority.HIGHEST, ignoreCancelled = true)
    public void fire(BlockIgniteEvent event) {
        if (claimedFor(event.getBlock(), "fire_spread")) event.setCancelled(true);
    }

    @EventHandler(priority = EventPriority.HIGHEST, ignoreCancelled = true)
    public void burn(BlockBurnEvent event) {
        if (claimedFor(event.getBlock(), "fire_spread")) event.setCancelled(true);
    }

    @EventHandler(priority = EventPriority.HIGHEST, ignoreCancelled = true)
    public void entityChangeBlock(EntityChangeBlockEvent event) {
        if (claimedFor(event.getBlock(), "mob_griefing")) event.setCancelled(true);
    }

    // ---- admin claim flags --------------------------------------------------------------------------------------
    // Each of these asks the claim index whether the admin claim under the event allows it; anywhere else they do nothing.

    private final java.util.Set<UUID> flightRevoked = java.util.concurrent.ConcurrentHashMap.newKeySet();

    private void tellFlag(Player player, String message) {
        long now = System.currentTimeMillis();
        Long last = deniedAt.get(player.getUniqueId());
        if (last != null && now - last < 1500) return;
        deniedAt.put(player.getUniqueId(), now);
        player.sendMessage(ChatColor.RED + message);
    }

    private static Player playerOf(org.bukkit.entity.Entity entity) {
        if (entity instanceof Player p) return p;
        if (entity instanceof org.bukkit.entity.Projectile shot && shot.getShooter() instanceof Player p) return p;
        return null;
    }

    @EventHandler(priority = EventPriority.HIGHEST, ignoreCancelled = true)
    public void flagCombat(EntityDamageByEntityEvent event) {
        Player attacker = playerOf(event.getDamager());
        if (attacker == null || attacker.hasPermission("scopenet.claims.bypass")) return;
        org.bukkit.entity.Entity victim = event.getEntity();
        if (victim instanceof Player) {
            if (!flagAllows(victim.getLocation(), "pvp")) { event.setCancelled(true); tellFlag(attacker, "PVP is disabled here."); }
        } else if ((victim instanceof org.bukkit.entity.Animals || victim instanceof org.bukkit.entity.AbstractVillager) && !flagAllows(victim.getLocation(), "animal_damage")) {
            event.setCancelled(true);
            tellFlag(attacker, "You can't hurt animals or villagers here.");
        }
    }

    @EventHandler(priority = EventPriority.HIGHEST, ignoreCancelled = true)
    public void flagDamage(EntityDamageEvent event) {
        if (event.getEntity() instanceof Player player && !flagAllows(player.getLocation(), "player_damage")) event.setCancelled(true);
    }

    @EventHandler(priority = EventPriority.HIGHEST, ignoreCancelled = true)
    public void flagHunger(FoodLevelChangeEvent event) {
        if (event.getEntity() instanceof Player player && event.getFoodLevel() < player.getFoodLevel() && !flagAllows(player.getLocation(), "hunger")) event.setCancelled(true);
    }

    @EventHandler(priority = EventPriority.HIGHEST, ignoreCancelled = true)
    public void flagDrop(PlayerDropItemEvent event) {
        if (!flagAllows(event.getPlayer(), event.getPlayer().getLocation(), "item_drop")) {
            event.setCancelled(true);
            tellFlag(event.getPlayer(), "You can't drop items here.");
        }
    }

    @EventHandler(priority = EventPriority.HIGHEST, ignoreCancelled = true)
    public void flagSpawn(CreatureSpawnEvent event) {
        CreatureSpawnEvent.SpawnReason why = event.getSpawnReason();
        if (why == CreatureSpawnEvent.SpawnReason.CUSTOM || why == CreatureSpawnEvent.SpawnReason.COMMAND) return;
        if (!flagAllows(event.getLocation(), "mob_spawning")) event.setCancelled(true);
    }

    @EventHandler(priority = EventPriority.HIGHEST, ignoreCancelled = true)
    public void flagTeleport(PlayerTeleportEvent event) {
        PlayerTeleportEvent.TeleportCause cause = event.getCause();
        if ((cause == PlayerTeleportEvent.TeleportCause.ENDER_PEARL || cause == PlayerTeleportEvent.TeleportCause.CHORUS_FRUIT)
                && event.getTo() != null && !flagAllows(event.getPlayer(), event.getTo(), "ender_pearls")) {
            event.setCancelled(true);
            tellFlag(event.getPlayer(), "You can't teleport in here.");
        }
    }

    @EventHandler(priority = EventPriority.HIGHEST, ignoreCancelled = true)
    public void flagToggleFlight(PlayerToggleFlightEvent event) {
        if (event.isFlying() && !flagAllows(event.getPlayer(), event.getPlayer().getLocation(), "fly")) {
            event.setCancelled(true);
            tellFlag(event.getPlayer(), "Flying isn't allowed here.");
        }
    }

    /** Checked when a player crosses a chunk border: entry, and grounding flight (and giving it back on the way out). */
    @EventHandler(priority = EventPriority.HIGHEST, ignoreCancelled = true)
    public void flagMove(PlayerMoveEvent event) {
        Location from = event.getFrom(), to = event.getTo();
        if (to == null || integration == null || (from.getBlockX() >> 4 == to.getBlockX() >> 4 && from.getBlockZ() >> 4 == to.getBlockZ() >> 4 && from.getWorld() == to.getWorld())) return;
        Player player = event.getPlayer();
        boolean staff = player.hasPermission("scopenet.claims.bypass");
        // The entry rule keeps visitors out; a guild's own members always get onto their land.
        if (!staff && !flagAllows(to, "entry") && flagAllows(from, "entry") && !memberOfClaim(player, to)) {
            event.setTo(from);
            tellFlag(player, "You can't enter this area.");
            return;
        }
        boolean mayFly = staff || flagAllows(to, "fly");
        if (!mayFly) {
            boolean creative = player.getGameMode() == GameMode.CREATIVE;
            if (player.isFlying() || player.isGliding() || (!creative && player.getAllowFlight())) {
                if (!creative && player.getAllowFlight() && player.getGameMode() != GameMode.SPECTATOR) flightRevoked.add(player.getUniqueId());
                player.setFlying(false);
                if (player.isGliding()) player.setGliding(false);
                if (!creative && player.getGameMode() != GameMode.SPECTATOR) player.setAllowFlight(false);
                tellFlag(player, "Flying isn't allowed here.");
            }
        } else if (flightRevoked.remove(player.getUniqueId())) {
            player.setAllowFlight(true);
        }
    }

    @EventHandler(priority = EventPriority.HIGH, ignoreCancelled = true)
    public void chat(AsyncPlayerChatEvent event) {
        if (integration != null && integration.settings().socialEnabled() && integration.settings().chatPrefixesEnabled()
                && chatFormatter != null && chatFormatter.layout().enabled()) {
            event.setFormat(chatFormatter.format(event.getPlayer()));
            event.setMessage(chatFormatter.message(event.getPlayer(), event.getMessage()));
        }
        add(event.getPlayer(), "messages");
    }

    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    public void command(PlayerCommandPreprocessEvent event) {
        audit(event.getPlayer(), "command", event.getMessage().strip().split("\\s+", 2)[0]);
    }

    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    public void statistic(PlayerStatisticIncrementEvent event) {
        if (integration == null) return;
        String item = event.getMaterial() != null ? ":" + event.getMaterial().name()
                : event.getEntityType() != null ? ":" + event.getEntityType().name() : "";
        integration.activity.action(event.getPlayer().getUniqueId(), event.getPlayer().getName(),
                event.getStatistic().name() + item, event.getNewValue() - event.getPreviousValue());
    }

    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    public void inventory(InventoryClickEvent event) {
        if (event.getWhoClicked() instanceof Player player)
            audit(player, "inventory", event.getAction().name() + " slot=" + event.getRawSlot()
                    + (event.getCurrentItem() == null ? "" : " item=" + event.getCurrentItem().getType().name()));
    }

    @EventHandler(priority = EventPriority.MONITOR)
    public void advancement(PlayerAdvancementDoneEvent event) {
        if (integration == null || !integration.settings().achievementsEnabled()) return;
        audit(event.getPlayer(), "advancement", event.getAdvancement().getKey().toString());
    }

    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    public void teleport(PlayerTeleportEvent event) {
        if (event.getTo() != null) audit(event.getPlayer(), "teleport", event.getCause().name() + " → "
                + event.getTo().getWorld().getName() + " " + event.getTo().getBlockX() + ","
                + event.getTo().getBlockY() + "," + event.getTo().getBlockZ());
    }

    private void audit(Player player, String kind, String detail) {
        if (integration != null) integration.activity.event(player.getUniqueId(), player.getName(), kind, detail);
    }

    @EventHandler(priority = EventPriority.MONITOR)
    public void death(EntityDeathEvent event) {
        if (event.getEntity() instanceof Player player) {
            add(player, "deaths");
            if (integration != null) integration.activity.event(player.getUniqueId(), player.getName(), "death",
                    event instanceof PlayerDeathEvent death ? death.getDeathMessage() : null);
        }
        Player killer = event.getEntity().getKiller();
        if (killer != null) {
            add(killer, event.getEntity() instanceof Player ? "player_kills" : "mob_kills");
            // The panel pays out any Casino bounty on the victim to the killer.
            if (integration != null && event.getEntity() instanceof Player victim && !victim.getUniqueId().equals(killer.getUniqueId())) {
                integration.activity.event(killer.getUniqueId(), killer.getName(), "pvp_kill", victim.getUniqueId().toString());
            }
            if (integration != null && !(event.getEntity() instanceof Player)) integration.activity.action(
                    killer.getUniqueId(), killer.getName(), "mob_kills:" + event.getEntityType().name(), 1);
        }
    }
}
