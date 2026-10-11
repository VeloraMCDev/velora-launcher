package net.velora.gateway;

import java.io.IOException;
import java.net.URI;
import java.nio.file.*;
import java.nio.file.attribute.PosixFilePermissions;
import java.util.*;

/** Operator-owned settings; environment values override the local file. */
final class Settings {
    private final String panel, token, yggdrasil, verified;
    private Settings(String panel, String token, String yggdrasil, String verified) {
        this.panel=panel;this.token=token;this.yggdrasil=yggdrasil;this.verified=verified;
    }
    static Settings load(Path directory, Map<String,String> environment) throws IOException {
        Files.createDirectories(directory);
        Path file=directory.resolve("gateway.properties");
        if (!Files.exists(file)) {
            try {
                if (Files.getFileStore(directory).supportsFileAttributeView("posix"))
                    Files.createFile(file, PosixFilePermissions.asFileAttribute(PosixFilePermissions.fromString("rw-------")));
                else Files.createFile(file);
                Files.writeString(file, """
                    # Environment settings override these values. Keep this file private.
                    # Panel HTTPS base URL, and the same gateway token configured on Panel.
                    panel-url=
                    gateway-token=
                    # Existing Velora Yggdrasil root used in the JVM's authlib-injector argument.
                    yggdrasil-url=
                    # Set to velora-yggdrasil only after custom-account login/UUID acceptance.
                    auth-verified=
                    """);
            } catch (FileAlreadyExistsException ignored) { /* Another startup created it. */ }
        }
        Properties properties=new Properties();
        try (var stream=Files.newInputStream(file)) {properties.load(stream);}
        String panel=required(environment,properties,"VELORA_GATEWAY_PANEL","panel-url");
        String token=required(environment,properties,"VELORA_GATEWAY_TOKEN","gateway-token");
        String yggdrasil=required(environment,properties,"VELORA_GATEWAY_YGGDRASIL","yggdrasil-url");
        String verified=required(environment,properties,"VELORA_GATEWAY_AUTH_VERIFIED","auth-verified");
        https(panel,"VELORA_GATEWAY_PANEL / panel-url");https(yggdrasil,"VELORA_GATEWAY_YGGDRASIL / yggdrasil-url");
        if (token.length()<32 || token.indexOf('\r')>=0 || token.indexOf('\n')>=0)
            throw new IllegalArgumentException("VELORA_GATEWAY_TOKEN / gateway-token must be a 32+ character bearer credential without newlines");
        return new Settings(panel,token,yggdrasil,verified);
    }
    private static String required(Map<String,String> env,Properties properties,String name,String key) {
        String value=env.containsKey(name) ? env.get(name) : properties.getProperty(key);
        if (value==null || value.isBlank()) throw new IllegalArgumentException("Missing "+name+": set that environment variable or '"+key+"' in plugins/velora-gateway/gateway.properties");
        return value;
    }
    private static void https(String value,String name) {
        URI uri;
        try {uri=URI.create(value);} catch (IllegalArgumentException e) {throw new IllegalArgumentException(name+" must be a valid HTTPS URL");}
        if (!"https".equals(uri.getScheme()) || uri.getHost()==null || uri.getUserInfo()!=null || uri.getQuery()!=null || uri.getFragment()!=null)
            throw new IllegalArgumentException(name+" must be an HTTPS URL without credentials, query parameters or fragments");
    }
    void verifyAuthentication(boolean onlineMode,List<String> arguments) {
        if (!onlineMode) throw new IllegalArgumentException("Authenticated online mode required; use verified custom Yggdrasil injection for Velora accounts");
        if (!"velora-yggdrasil".equals(verified)) throw new IllegalArgumentException("Custom identity acceptance gate not confirmed: set VELORA_GATEWAY_AUTH_VERIFIED / auth-verified only after testing Velora accounts");
        if (arguments.stream().noneMatch(this::matchingAgent))
            throw new IllegalArgumentException("Verified custom authentication Java agent required: start the Velocity JVM with -javaagent:/path/to/authlib-injector.jar=<configured Yggdrasil URL>");
    }
    private boolean matchingAgent(String argument) {
        if (!argument.startsWith("-javaagent:")) return false;
        String agent=argument.substring("-javaagent:".length());int options=agent.indexOf('=');
        if (options<0 || !agent.substring(options+1).equals(yggdrasil)) return false;
        String jar=agent.substring(0,options).replace('\\','/');
        return jar.substring(jar.lastIndexOf('/')+1).equals("authlib-injector.jar");
    }
    String panel() {return panel;}
    String token() {return token;}
    @Override public String toString() {return "Gateway settings (credential redacted)";}
}
