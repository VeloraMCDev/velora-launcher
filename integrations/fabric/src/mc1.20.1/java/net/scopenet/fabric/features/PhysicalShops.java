package net.scopenet.fabric.features;

import com.google.gson.*;
import net.minecraft.core.BlockPos;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.ChestBlock;
import net.minecraft.world.level.block.state.properties.ChestType;
import net.minecraft.world.level.block.entity.ChestBlockEntity;
import net.minecraft.world.phys.BlockHitResult;
import net.minecraft.world.phys.HitResult;
import net.scopenet.core.*;
import java.util.*;

/** A pedestal displays a sample; its empty vanilla chest opens panel-backed stock custody. */
public final class PhysicalShops {
    private final Env env;
    private final CommandSet commands;
    private final FabricPlatform platform;
    private final net.minecraft.server.MinecraftServer server;
    private ClientLink link;
    private List<JsonObject> shops=List.of();
    private final Map<UUID,JsonObject> offers=new HashMap<>();
    private final Set<UUID> buying=new HashSet<>();
    private final Map<UUID,JsonObject> promotions=new HashMap<>();
    private long promotionPrice=5000;
    private boolean requiresClaim=true;
    private boolean polling;
    private long nextPoll;
    static volatile Set<String> stockChests=Set.of();
    PhysicalShops(Env env,CommandSet commands,FabricPlatform platform,net.minecraft.server.MinecraftServer server){this.env=env;this.commands=commands;this.platform=platform;this.server=server;}
    static String location(String dimension,BlockPos p){return dimension+":"+p.getX()+":"+p.getY()+":"+p.getZ();}
    public static boolean isStockChest(String dimension,BlockPos p){return stockChests.contains(location(dimension,p));}
    private boolean enabled(){return env.modules.get().enabled("economy")&&env.modules.get().enabled("vaults")&&(!requiresClaim||env.modules.get().enabled("factions"));}
    void tick(){
        if(polling||env.clock.getAsLong()<nextPoll)return;
        polling=true;nextPoll=env.clock.getAsLong()+5000;
        env.io(()->env.panel.call("economy/physical-shops",new JsonObject()).getAsJsonObject(),result->{
            polling=false;List<JsonObject> next=new ArrayList<>();Set<String> chests=new HashSet<>();
            promotionPrice=result.get("promotion_cents_per_day").getAsLong();
            requiresClaim=result.get("requires_claim").getAsBoolean();
            for(var row:result.getAsJsonArray("shops")){JsonObject s=row.getAsJsonObject();next.add(s);chests.add(location(s.get("dimension").getAsString(),pos(s,true)));}
            shops=List.copyOf(next);stockChests=Set.copyOf(chests);renderDisplays();
        },error->polling=false);
    }
    private void renderDisplays(){
        for(JsonObject s:shops){
            var level=server.getLevel(net.minecraft.resources.ResourceKey.create(net.minecraft.core.registries.Registries.DIMENSION,new net.minecraft.resources.ResourceLocation(s.get("dimension").getAsString())));
            if(level==null||!level.hasChunkAt(pos(s,false)))continue;
            BlockPos at=pos(s,false);String tag="velora_shop_"+s.get("id").getAsString();
            var displays=level.getEntitiesOfClass(net.minecraft.world.entity.Display.ItemDisplay.class,new net.minecraft.world.phys.AABB(at).inflate(2),entity->entity.getTags().contains(tag));
            boolean valid=s.get("enabled").getAsBoolean()&&level.getBlockState(at).is(Blocks.POLISHED_BLACKSTONE_SLAB);
            if(!valid){displays.forEach(net.minecraft.world.entity.Entity::discard);continue;}
            if(!displays.isEmpty()){for(int i=1;i<displays.size();i++)displays.get(i).discard();continue;}
            var display=net.minecraft.world.entity.EntityType.ITEM_DISPLAY.create(level);if(display==null)continue;
            var item=FabricItems.fromCore(new Item(s.get("item_id").getAsString(),s.get("item_name").getAsString(),s.get("quantity").getAsInt(),s.get("display_data").getAsString()));
            display.getSlot(0).set(item);display.setPos(at.getX()+.5,at.getY()+1,at.getZ()+.5);display.setInvulnerable(true);display.setNoGravity(true);display.addTag(tag);level.addFreshEntity(display);
        }
    }
    private static BlockPos pos(JsonObject s,boolean chest){String prefix=chest?"chest_":"";return new BlockPos(s.get(prefix+"x").getAsInt(),s.get(prefix+"y").getAsInt(),s.get(prefix+"z").getAsInt());}
    private static boolean valid(ServerPlayer player,JsonObject s){
        var level=player.serverLevel();BlockPos chest=pos(s,true);
        var state=level.getBlockState(chest);
        return s.get("enabled").getAsBoolean()&&level.dimension().location().toString().equals(s.get("dimension").getAsString())&&level.getBlockState(pos(s,false)).is(Blocks.POLISHED_BLACKSTONE_SLAB)
            &&state.is(Blocks.CHEST)&&state.getValue(ChestBlock.TYPE)==ChestType.SINGLE&&level.getBlockEntity(chest) instanceof ChestBlockEntity entity&&entity.isEmpty();
    }
    boolean use(ServerPlayer player,BlockPos at){
        String dimension=player.level().dimension().location().toString();
        for(JsonObject s:shops){if(!dimension.equals(s.get("dimension").getAsString()))continue;
            if((at.equals(pos(s,true))||at.equals(pos(s,false)))&&!platform.wrap(player).hasPermission("scopenet.command.pshop")){message(player,"You do not have permission to use physical shops.");return true;}
            if(at.equals(pos(s,true))){
                if(!s.get("owner_uuid").getAsString().equals(player.getUUID().toString()))message(player,"Only the seller can access this stock chest.");
                else if(!enabled()||InventoryLocks.locked(player.getUUID()))message(player,"Shop stock is unavailable while modules or inventory recovery are pending.");
                else commands.utilityHub.cloudVaults.open(platform.wrap(player),"shop:"+s.get("id").getAsString(),1,"Shop stock");
                return true;
            }
            if(at.equals(pos(s,false))){
                if(!enabled()||!valid(player,s)){message(player,"This shop is paused; its pedestal and empty single stock chest must be restored.");return true;}
                JsonObject offer=s.deepCopy();offer.addProperty("operation_id",UUID.randomUUID().toString());offers.put(player.getUUID(),offer);
                message(player,s.get("quantity").getAsInt()+"x "+s.get("item_name").getAsString()+" for "+env.money(s.get("price_cents").getAsLong()/100.0)+". Run /pshop buy to confirm. Delivery goes to your vault.");
                if(link!=null)link.open(platform.wrap(player),"physical_shop");
                return true;
            }
        }return false;
    }
    void request(CorePlayer player,JsonObject message,ClientLink link){
        this.link=link;String id=message.get("id").getAsString();ServerPlayer p=server.getPlayerList().getPlayer(player.uuid());
        if(!message.get("operation").getAsString().equals("physical_shop")||message.has("args")&&!message.get("args").isJsonObject()){link.result(player,id,null,"Unsupported shop request.");return;}
        if(p==null||!enabled()||!player.hasPermission("scopenet.command.pshop")){link.result(player,id,null,"Physical shops are unavailable.");return;}
        JsonObject offer=offers.get(player.uuid());
        if(offer==null||!valid(p,offer)||p.distanceToSqr(pos(offer,false).getX()+.5,pos(offer,false).getY()+.5,pos(offer,false).getZ()+.5)>36){link.result(player,id,null,"Review a nearby shop pedestal first.");return;}
        String action=message.has("args")&&message.getAsJsonObject("args").has("action")?message.getAsJsonObject("args").get("action").getAsString():"view";
        if(action.equals("buy")){buy(p,data->link.result(player,id,data,null),error->link.result(player,id,null,error));return;}
        JsonObject view=new JsonObject();for(String key:List.of("item_name","quantity","price_cents"))view.add(key,offer.get(key));link.result(player,id,view,null);
    }
    void link(ClientLink link){this.link=link;}
    void command(ServerPlayer player,String text){
        if(InventoryLocks.locked(player.getUUID())){message(player,"Wait for inventory recovery.");return;}
        if(!platform.wrap(player).hasPermission("scopenet.command.pshop")){message(player,"You do not have permission to use physical shops.");return;}
        String[] args=text.trim().split("\\s+");String sub=args[0].toLowerCase(Locale.ROOT);
        if(sub.equals("stock")&&args.length==2){
            var shop=shops.stream().filter(s->s.get("id").getAsString().equals(args[1])&&s.get("owner_uuid").getAsString().equals(player.getUUID().toString())).findFirst();
            if(shop.isPresent())commands.utilityHub.cloudVaults.open(platform.wrap(player),"shop:"+args[1],1,"Shop stock");else message(player,"Your shop was not found.");return;
        }
        if(!enabled()){message(player,"Physical shops require economy, vaults and factions.");return;}
        if(sub.equals("buy")){buy(player);return;}
        if((sub.equals("close")||sub.equals("reopen"))&&args.length==2){
            var shop=shops.stream().filter(s->s.get("id").getAsString().equals(args[1])&&s.get("owner_uuid").getAsString().equals(player.getUUID().toString())).findFirst();
            if(shop.isEmpty()){message(player,"Your shop was not found.");return;}
            JsonObject body=new JsonObject();body.addProperty("uuid",player.getUUID().toString());body.addProperty("id",args[1]);body.add("price_cents",shop.get().get("price_cents"));body.addProperty("enabled",sub.equals("reopen"));
            env.io(()->env.panel.call("economy/physical-shops/update",body),ok->{nextPoll=0;tick();message(player,"Shop "+(sub.equals("close")?"closed. Stock remains recoverable.":"reopened."));},error->message(player,error));return;
        }
        if(sub.equals("visit")&&args.length==2){
            var shop=shops.stream().filter(s->s.get("id").getAsString().equals(args[1])&&s.get("enabled").getAsBoolean()&&s.get("warp_enabled").getAsBoolean()&&s.has("promoted_until")&&!s.get("promoted_until").isJsonNull()&&java.time.Instant.parse(s.get("promoted_until").getAsString()).isAfter(java.time.Instant.now())).findFirst();
            if(shop.isEmpty()){message(player,"This shop has no active warp listing.");return;}
            JsonObject target=shop.get();var safe=platform.safeSurface(target.get("dimension").getAsString(),target.get("x").getAsInt(),target.get("z").getAsInt());
            if(safe.isPresent())platform.wrap(player).teleport(safe.get());else message(player,"No safe destination was found.");return;
        }
        if(sub.equals("price")&&args.length==3){
            try{long cents=new java.math.BigDecimal(args[2]).movePointRight(2).longValueExact();if(cents<1||cents>100_000_000_000L)throw new IllegalArgumentException();
                var shop=shops.stream().filter(s->s.get("id").getAsString().equals(args[1])&&s.get("owner_uuid").getAsString().equals(player.getUUID().toString())).findFirst();if(shop.isEmpty())throw new IllegalArgumentException();
                JsonObject body=new JsonObject();body.addProperty("uuid",player.getUUID().toString());body.addProperty("id",args[1]);body.addProperty("price_cents",cents);body.add("enabled",shop.get().get("enabled"));
                env.io(()->env.panel.call("economy/physical-shops/update",body),ok->{nextPoll=0;tick();message(player,"Shop price updated.");},error->message(player,error));
            }catch(RuntimeException invalid){message(player,"Use /pshop price <your shop UUID> <positive dollars>.");}return;
        }
        if(sub.equals("promote")&&args.length>=3){promote(player,args);return;}
        if(!sub.equals("create")||args.length<2||args.length>3){message(player,"Hold your product, look at a polished blackstone slab beside an empty single chest, then /pshop create <price> [quantity]. Right-click a pedestal, then /pshop buy. Recover stock with /pshop stock <shop UUID>.");return;}
        try{
            long cents=new java.math.BigDecimal(args[1]).movePointRight(2).longValueExact();
            var item=player.getMainHandItem();int quantity=args.length==3?Integer.parseInt(args[2]):1;
            if(item.isEmpty()||quantity<1||quantity>item.getMaxStackSize()||cents<1||cents>100_000_000_000L)throw new IllegalArgumentException();
            HitResult hit=player.pick(6,0,false);if(!(hit instanceof BlockHitResult block)||hit.getType()!=HitResult.Type.BLOCK)throw new IllegalArgumentException();
            BlockPos at=block.getBlockPos();if(!player.level().getBlockState(at).is(Blocks.POLISHED_BLACKSTONE_SLAB))throw new IllegalArgumentException();
            BlockPos chest=null;
            for(BlockPos candidate:BlockPos.betweenClosed(at.offset(-2,-1,-2),at.offset(2,1,2))){
                if(candidate.distManhattan(at)>2)continue;var state=player.level().getBlockState(candidate);
                if(state.is(Blocks.CHEST)&&state.getValue(ChestBlock.TYPE)==ChestType.SINGLE&&player.level().getBlockEntity(candidate) instanceof ChestBlockEntity entity&&entity.isEmpty()&&!isStockChest(player.level().dimension().location().toString(),candidate)){
                    if(chest!=null){message(player,"Place exactly one eligible stock chest within two blocks of the pedestal.");return;}chest=candidate.immutable();
                }
            }
            if(chest==null)throw new IllegalArgumentException();
            JsonObject body=new JsonObject();body.addProperty("id",UUID.randomUUID().toString());body.addProperty("uuid",player.getUUID().toString());body.addProperty("dimension",player.level().dimension().location().toString());
            body.addProperty("x",at.getX());body.addProperty("y",at.getY());body.addProperty("z",at.getZ());body.addProperty("chest_x",chest.getX());body.addProperty("chest_y",chest.getY());body.addProperty("chest_z",chest.getZ());
            var sample=item.copy();sample.setCount(quantity);body.addProperty("item_id",net.minecraft.core.registries.BuiltInRegistries.ITEM.getKey(item.getItem()).toString());
            String name=item.getHoverName().getString();body.addProperty("item_name",name.length()>128?name.substring(0,128):name);body.addProperty("fingerprint",FabricPlatform.itemFingerprint(item));body.addProperty("display_data",sample.save(new net.minecraft.nbt.CompoundTag()).toString());
            body.addProperty("quantity",quantity);body.addProperty("price_cents",cents);
            env.io(()->env.panel.call("economy/physical-shops/create",body),result->{nextPoll=0;tick();message(player,"Shop created: "+body.get("id").getAsString()+". Right-click the linked chest to deposit stock. Your sample stays in your hand.");},error->message(player,error));
        }catch(RuntimeException invalid){message(player,"Use a positive dollar price (at most two decimals), a valid quantity, and a slab beside one empty single chest.");}
    }
    private void promote(ServerPlayer player,String[] args){
        try{
            int days=Integer.parseInt(args[2]);boolean warp=args.length>3&&args[3].equals("warp");if(days<1||days>30)throw new IllegalArgumentException();
            boolean confirmed=args[args.length-1].equals("confirm");
            if(!confirmed){
                JsonObject body=new JsonObject();body.addProperty("uuid",player.getUUID().toString());body.addProperty("username",player.getGameProfile().getName());body.addProperty("id",args[1]);body.addProperty("days",days);body.addProperty("warp",warp);body.addProperty("expected_price_cents",Math.multiplyExact(promotionPrice,days));body.addProperty("operation_id",UUID.randomUUID().toString());promotions.put(player.getUUID(),body);
                message(player,"Map listing costs "+env.money(promotionPrice*days/100.0)+" for "+days+" days. Confirm with /pshop promote "+args[1]+" "+days+(warp?" warp":"")+" confirm");return;
            }
            JsonObject body=promotions.get(player.getUUID());if(body==null||!body.get("id").getAsString().equals(args[1])||body.get("days").getAsInt()!=days||body.get("warp").getAsBoolean()!=warp){message(player,"Review this promotion without 'confirm' first.");return;}
            if(!buying.add(player.getUUID()))return;
            env.io(()->env.panel.call("economy/physical-shops/promote",body),result->{buying.remove(player.getUUID());promotions.remove(player.getUUID());nextPoll=0;tick();message(player,"Shop promotion purchased.");},error->{buying.remove(player.getUUID());message(player,error+". Repeat the same confirmation to retry safely.");});
        }catch(RuntimeException invalid){message(player,"Use /pshop promote <your shop UUID> <1-30 days> [warp], then confirm the quote.");}
    }
    private void buy(ServerPlayer player){
        buy(player,result->message(player,"Purchased. Your items are queued for your vault; no shop fee was charged."),error->message(player,error+". Use /pshop buy to retry this same purchase safely."));
    }
    private void buy(ServerPlayer player,java.util.function.Consumer<JsonElement> success,java.util.function.Consumer<String> failure){
        JsonObject offer=offers.get(player.getUUID());
        if(offer==null||!valid(player,offer)||player.distanceToSqr(pos(offer,false).getX()+.5,pos(offer,false).getY()+.5,pos(offer,false).getZ()+.5)>36){failure.accept("Right-click a nearby shop pedestal and review its price first.");return;}
        if(!buying.add(player.getUUID())){failure.accept("A shop purchase is already running.");return;}
        JsonObject body=new JsonObject();body.addProperty("uuid",player.getUUID().toString());body.addProperty("username",player.getGameProfile().getName());body.add("id",offer.get("id"));body.add("operation_id",offer.get("operation_id"));body.add("expected_price_cents",offer.get("price_cents"));
        env.io(()->env.panel.call("economy/physical-shops/buy",body),result->{buying.remove(player.getUUID());offers.remove(player.getUUID());success.accept(result);},error->{buying.remove(player.getUUID());failure.accept(error);});
    }
    private static void message(ServerPlayer p,String text){p.sendSystemMessage(net.minecraft.network.chat.Component.literal(text));}
}
