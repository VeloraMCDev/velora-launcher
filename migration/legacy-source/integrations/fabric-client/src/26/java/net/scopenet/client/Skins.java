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

/** Player heads from the panel's avatar endpoint, by UUID: the same skins the launcher and the web panel show. Bounded and screen-owned. */
public final class Skins implements AutoCloseable {
    private final Map<String,Identifier> heads=new LinkedHashMap<>();
    private final Set<String> pending=new HashSet<>();
    private final Map<String,Long> failed=new HashMap<>();
    private final ExecutorService worker=Executors.newSingleThreadExecutor(r->{Thread t=new Thread(r,"scopenet-skins");t.setDaemon(true);return t;});
    private final HttpClient http=HttpClient.newBuilder().connectTimeout(Duration.ofSeconds(3)).followRedirects(HttpClient.Redirect.NEVER).build();
    private boolean closed;private int sequence;
    private final String namespace=UUID.randomUUID().toString();
    /** The head texture, or null while it loads (a request starts) or when the panel has no skin for this player. */
    public Identifier head(String base,String uuid){
        if(closed||uuid==null||base==null||base.isBlank())return null;
        // The panel's avatar endpoint takes a UUID or an account name.
        String id=uuid.toLowerCase(Locale.ROOT).replace("-","");
        if(!id.matches("[0-9a-f]{32}")&&!uuid.matches("[A-Za-z0-9_]{1,16}"))return null;
        if(!id.matches("[0-9a-f]{32}"))id=uuid.toLowerCase(Locale.ROOT);
        final String key=id;
        Identifier cached=heads.remove(key);if(cached!=null){heads.put(key,cached);return cached;}
        if(pending.size()>=4||pending.contains(key)||failed.getOrDefault(key,0L)>System.currentTimeMillis())return null;
        failed.entrySet().removeIf(e->e.getValue()<System.currentTimeMillis());
        if(failed.size()>=256)failed.clear();
        URI root;try{root=URI.create(base);if(!Set.of("http","https").contains(root.getScheme())||root.getHost()==null||root.getUserInfo()!=null)return null;}catch(Exception e){return null;}
        String url=base.replaceAll("/+$","")+"/api/v1/avatar/"+key+"?size=64";
        pending.add(key);
        worker.execute(()->{
            try{
                var response=http.send(HttpRequest.newBuilder(URI.create(url)).timeout(Duration.ofSeconds(5)).GET().build(),HttpResponse.BodyHandlers.ofInputStream());
                byte[] bytes=null;
                try(InputStream in=response.body()){if(response.statusCode()==200)bytes=in.readNBytes(60_001);}
                if(bytes==null||bytes.length>60_000||bytes.length<24)throw new IOException();
                java.nio.ByteBuffer header=java.nio.ByteBuffer.wrap(bytes);
                if(header.getLong()!=0x89504e470d0a1a0aL||header.getInt(16)<8||header.getInt(16)>128||header.getInt(20)<8||header.getInt(20)>128)throw new IOException();
                byte[] image=bytes;
                Minecraft.getInstance().execute(()->{
                    pending.remove(key);if(closed)return;
                    try{NativeImage pixels=NativeImage.read(image);Identifier tex=Identifier.parse("scopenet:skin/"+namespace+"/"+sequence++);
                        Minecraft.getInstance().getTextureManager().register(tex,new DynamicTexture(()->"SCOPENET player head",pixels));heads.put(key,tex);
                        if(heads.size()>96){String old=heads.keySet().iterator().next();Minecraft.getInstance().getTextureManager().release(heads.remove(old));}
                    }catch(Exception e){failed.put(key,System.currentTimeMillis()+60000);}
                });
            }catch(Exception e){Minecraft.getInstance().execute(()->{pending.remove(key);if(!closed)failed.put(key,System.currentTimeMillis()+60000);});}
        });
        return null;
    }
    @Override public void close(){closed=true;worker.shutdownNow();http.shutdownNow();heads.values().forEach(t->Minecraft.getInstance().getTextureManager().release(t));heads.clear();pending.clear();failed.clear();}
}
