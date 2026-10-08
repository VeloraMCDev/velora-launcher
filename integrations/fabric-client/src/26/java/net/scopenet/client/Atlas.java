package net.scopenet.client;

import com.mojang.blaze3d.platform.NativeImage;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.texture.DynamicTexture;
import net.minecraft.resources.Identifier;
import java.net.*;
import java.net.http.*;
import java.io.*;
import java.time.Duration;
import java.util.*;
import java.util.concurrent.*;

/** Bounded, screen-owned tile cache (the panel keeps 900 tiles; a GPU texture each is heavier, so 200 here). HTTP workers never touch GPU resources. */
public final class Atlas implements AutoCloseable {
    private final Map<String,Identifier> tiles=new LinkedHashMap<>();
    private final Set<String> pending=new HashSet<>();
    private final Map<String,Long> failed=new HashMap<>();
    private final ExecutorService worker=Executors.newSingleThreadExecutor(r->{Thread t=new Thread(r,"scopenet-map");t.setDaemon(true);return t;});
    private final HttpClient http=HttpClient.newBuilder().connectTimeout(Duration.ofSeconds(3)).followRedirects(HttpClient.Redirect.NEVER).build();
    private boolean closed;private int sequence;
    private final String namespace=UUID.randomUUID().toString();
    /** A tile that is already loaded; never starts a download. */
    public Identifier peek(String dim,int zoom,int x,int z){return tiles.get(dim+":"+zoom+":"+x+":"+z);}
    /** The server has no tile here (yet): finer or coarser levels may still have one. */
    public boolean missing(String dim,int zoom,int x,int z){return failed.getOrDefault(dim+":"+zoom+":"+x+":"+z,0L)>System.currentTimeMillis();}
    public Identifier tile(String base,long server,String token,String dim,int zoom,int x,int z){
        if(closed||token.isBlank()||!dim.matches("[a-z0-9_.:-]{1,100}"))return null;
        String key=dim+":"+zoom+":"+x+":"+z;Identifier cached=tiles.remove(key);if(cached!=null){tiles.put(key,cached);return cached;}
        if(pending.size()>=10||pending.contains(key)||failed.getOrDefault(key,0L)>System.currentTimeMillis())return null;
        failed.entrySet().removeIf(e->e.getValue()<System.currentTimeMillis());
        if(failed.size()>=128)failed.clear();
        URI root;try{root=URI.create(base);if(!Set.of("http","https").contains(root.getScheme())||root.getHost()==null||root.getUserInfo()!=null)return null;}catch(Exception e){return null;}
        String url=base.replaceAll("/+$","")+"/api/map/"+server+"/"+URLEncoder.encode(dim,java.nio.charset.StandardCharsets.UTF_8)+"/"+zoom+"/"+x+"/"+z+".png?t="+URLEncoder.encode(token,java.nio.charset.StandardCharsets.UTF_8);
        pending.add(key);
        worker.execute(()->{
            byte[] bytes=null;
            try{
                var response=http.send(HttpRequest.newBuilder(URI.create(url)).timeout(Duration.ofSeconds(5)).GET().build(),HttpResponse.BodyHandlers.ofInputStream());
                try(InputStream in=response.body()){if(response.statusCode()==200)bytes=in.readNBytes(200_001);}
                if(bytes==null||bytes.length>200_000||bytes.length<24)throw new IOException();
                // PNG IHDR dimensions are checked before decoder allocation.
                java.nio.ByteBuffer header=java.nio.ByteBuffer.wrap(bytes);if(header.getLong()!=0x89504e470d0a1a0aL||header.getInt(16)!=256||header.getInt(20)!=256)throw new IOException();
                byte[] image=bytes;
                Minecraft.getInstance().execute(()->{
                    pending.remove(key);if(closed)return;
                    try{NativeImage pixels=NativeImage.read(image);Identifier id=Identifier.parse("scopenet:map/"+namespace+"/"+sequence++);
                        Minecraft.getInstance().getTextureManager().register(id,new DynamicTexture(()->"Velora map tile",pixels));tiles.put(key,id);
                        if(tiles.size()>200){String old=tiles.keySet().iterator().next();Minecraft.getInstance().getTextureManager().release(tiles.remove(old));}
                    }catch(Exception e){failed.put(key,System.currentTimeMillis()+20000);}
                });
            }catch(Exception e){Minecraft.getInstance().execute(()->{pending.remove(key);if(!closed)failed.put(key,System.currentTimeMillis()+20000);});}
        });
        return null;
    }
    @Override public void close(){closed=true;worker.shutdownNow();http.shutdownNow();tiles.values().forEach(id->Minecraft.getInstance().getTextureManager().release(id));tiles.clear();pending.clear();failed.clear();}
}
