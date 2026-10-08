package net.scopenet.client;

import com.google.gson.*;
import java.util.*;

/** Client-thread-only state, cleared fully at every disconnect. */
public final class Model {
    public boolean connected, modern;
    public String server="", currency="$", error="", panel="";
    public JsonObject state = new JsonObject(), claims = new JsonObject(), features = new JsonObject();
    public JsonArray market = new JsonArray();
    public String toast=""; public long toastUntil;
    public static String text(JsonObject o,String key) { try {return o.get(key).isJsonPrimitive()?o.get(key).getAsString():"";}catch(Exception e){return "";} }
    public static double number(JsonObject o,String key) {try{return o.get(key).getAsDouble();}catch(Exception e){return 0;} }
    public static boolean flag(JsonObject o,String key) {try{return o.get(key).getAsBoolean();}catch(Exception e){return false;} }
    public static JsonObject object(JsonObject o,String key) {try{JsonObject v=o.getAsJsonObject(key);return v==null?new JsonObject():v;}catch(Exception e){return new JsonObject();} }
    public static JsonArray array(JsonObject o,String key) {try{JsonArray a=o.getAsJsonArray(key);return a==null?new JsonArray():a;}catch(Exception e){return new JsonArray();} }
    public void reset() { connected=modern=false;server="";currency="$";error="";panel="";state=new JsonObject();claims=new JsonObject();features=new JsonObject();market=new JsonArray();toast="";toastUntil=0; }
    public void handle(JsonObject m) {
        switch(text(m,"t")) {
            case "hello" -> {connected=number(m,"v")==1;modern=number(m,"companion")==2;server=text(m,"server");panel=text(m,"panel");currency=text(m,"currency");features=object(m,"features");}
            case "state" -> state=m;
            case "claims" -> claims=m;
            case "market" -> market=array(m,"listings");
            case "notify" -> {toast=text(m,"title")+": "+text(m,"text");toastUntil=System.currentTimeMillis()+6000;}
        }
    }
    public String money(double n) {return currency+String.format(Locale.ROOT,"%,.2f",n);}
}
