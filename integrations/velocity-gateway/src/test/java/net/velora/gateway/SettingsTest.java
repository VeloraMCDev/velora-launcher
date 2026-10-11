package net.velora.gateway;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;
import java.nio.file.*;
import java.util.*;
import static org.junit.jupiter.api.Assertions.*;

class SettingsTest {
    @TempDir Path dir;
    private Map<String,String> environment() {
        return new HashMap<>(Map.of("VELORA_GATEWAY_PANEL","https://panel.example.org",
            "VELORA_GATEWAY_TOKEN","synthetic-gateway-test-credential-000000",
            "VELORA_GATEWAY_YGGDRASIL","https://auth.example.org/api/yggdrasil",
            "VELORA_GATEWAY_AUTH_VERIFIED","velora-yggdrasil"));
    }
    @Test void missingAndBlankSettingsNameTheProblemWithoutNullPointerExceptions() throws Exception {
        for (String name:environment().keySet()) {
            var env=environment();env.remove(name);
            var absent=assertThrows(IllegalArgumentException.class,()->Settings.load(dir,env));
            assertTrue(absent.getMessage().contains("Missing "+name));
            env.put(name," ");assertTrue(assertThrows(IllegalArgumentException.class,()->Settings.load(dir,env)).getMessage().contains(name));
        }
        assertTrue(Files.exists(dir.resolve("gateway.properties")));
        if (Files.getFileStore(dir).supportsFileAttributeView("posix"))
            assertEquals(java.nio.file.attribute.PosixFilePermissions.fromString("rw-------"),Files.getPosixFilePermissions(dir.resolve("gateway.properties")));
    }
    @Test void localFileSupportsPanelLaunchersAndEnvironmentOverridesRetainCredentials() throws Exception {
        var env=environment();Properties properties=new Properties();
        properties.setProperty("panel-url",env.get("VELORA_GATEWAY_PANEL"));properties.setProperty("gateway-token",env.get("VELORA_GATEWAY_TOKEN"));
        properties.setProperty("yggdrasil-url",env.get("VELORA_GATEWAY_YGGDRASIL"));properties.setProperty("auth-verified","velora-yggdrasil");
        try(var out=Files.newOutputStream(dir.resolve("gateway.properties"))){properties.store(out,"Synthetic settings");}
        Settings file=Settings.load(dir,Map.of());assertEquals(env.get("VELORA_GATEWAY_TOKEN"),file.token());
        Settings overridden=Settings.load(dir,Map.of("VELORA_GATEWAY_PANEL","https://override.example.org"));
        assertEquals("https://override.example.org",overridden.panel());assertEquals(file.token(),overridden.token());
        assertFalse(file.toString().contains(file.token()));
        assertThrows(IllegalArgumentException.class,()->Settings.load(dir,Map.of("VELORA_GATEWAY_TOKEN","")));
    }
    @Test void authenticationStillRequiresOnlineModeAttestationAndExactAgentUrl() throws Exception {
        Settings settings=Settings.load(dir,environment());String url=environment().get("VELORA_GATEWAY_YGGDRASIL");
        for(String jar:List.of("authlib-injector.jar","/srv/authlib-injector.jar","C:\\proxy\\authlib-injector.jar"))
            assertDoesNotThrow(()->settings.verifyAuthentication(true,List.of("-javaagent:"+jar+"="+url)));
        assertThrows(IllegalArgumentException.class,()->settings.verifyAuthentication(false,List.of("-javaagent:authlib-injector.jar="+url)));
        for(String argument:List.of("-javaagent:authlib-injector.jar=https://wrong.example.org","-javaagent:other.jar="+url,
            "-javaagent:other.jar=/authlib-injector.jar="+url,"-Dspoof=-javaagent:authlib-injector.jar="+url))
            assertThrows(IllegalArgumentException.class,()->settings.verifyAuthentication(true,List.of(argument)));
        var env=environment();env.put("VELORA_GATEWAY_AUTH_VERIFIED","not-verified");
        assertThrows(IllegalArgumentException.class,()->Settings.load(dir,env).verifyAuthentication(true,List.of("-javaagent:authlib-injector.jar="+url)));
    }
    @Test void invalidSettingsDoNotEchoCredentialValues() {
        var withCredentials=environment();withCredentials.put("VELORA_GATEWAY_PANEL","https://user:synthetic-private-password@example.org");
        assertFalse(assertThrows(IllegalArgumentException.class,()->Settings.load(dir,withCredentials)).getMessage().contains("synthetic-private-password"));
        var invalid=environment();invalid.put("VELORA_GATEWAY_TOKEN","short");
        assertTrue(assertThrows(IllegalArgumentException.class,()->Settings.load(dir,invalid)).getMessage().contains("VELORA_GATEWAY_TOKEN"));
    }
}
