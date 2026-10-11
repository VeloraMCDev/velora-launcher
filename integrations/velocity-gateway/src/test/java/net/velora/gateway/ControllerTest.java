package net.velora.gateway;

import com.sun.net.httpserver.HttpServer;
import org.junit.jupiter.api.*;
import java.net.*;
import java.net.http.*;
import java.nio.charset.StandardCharsets;
import java.util.*;
import javax.net.ssl.SSLHandshakeException;
import static org.junit.jupiter.api.Assertions.*;

class ControllerTest {
    private HttpServer server;
    private Controller controller;
    private int status=200;
    private String type="application/json", body="{\"revision\":1,\"routes\":[]}";
    private final List<String> paths=new ArrayList<>(), credentials=new ArrayList<>();
    @BeforeEach void start() throws Exception {
        server=HttpServer.create(new InetSocketAddress("127.0.0.1",0),0);
        server.createContext("/",exchange->{
            paths.add(exchange.getRequestURI().getPath());credentials.add(exchange.getRequestHeaders().getFirst("Authorization"));
            exchange.getRequestBody().readAllBytes();
            exchange.getResponseHeaders().set("Content-Type",type);
            exchange.getResponseHeaders().set("Location","/login?token=synthetic-private-redirect");
            byte[] bytes=body.getBytes(StandardCharsets.UTF_8);
            exchange.sendResponseHeaders(status,bytes.length);exchange.getResponseBody().write(bytes);exchange.close();
        });
        server.start();controller=new Controller(HttpClient.newHttpClient(),"http://127.0.0.1:"+server.getAddress().getPort()+"/","synthetic-private-credential");
    }
    @AfterEach void stop() {server.stop(0);}
    @Test void realRequestsUseExactPathsAndCredentialAndRequireAcknowledgement() throws Exception {
        assertEquals(1,controller.fetch().revision());body="{\"ok\":true}";controller.report(Map.of("revision",1));
        assertEquals(List.of("/api/gateway/v1/routes","/api/gateway/v1/status"),paths);
        assertEquals(List.of("Bearer synthetic-private-credential","Bearer synthetic-private-credential"),credentials);
    }
    @Test void rejectsHttpErrorsForBothEndpointsWithoutLeakingBodiesOrFollowingRedirects() {
        body="synthetic-private-credential";
        for(int code:List.of(302,401,403,404,429,500)) {
            status=code;
            for(var call:List.<org.junit.jupiter.api.function.Executable>of(()->controller.fetch(),()->controller.report(Map.of()))) {
                var error=assertThrows(Controller.Failure.class,call);
                assertTrue(error.getMessage().contains("HTTP "+code));assertFalse(error.getMessage().contains("synthetic-private"));
            }
        }
        assertEquals(12,paths.size());assertTrue(paths.stream().noneMatch(p->p.contains("login")));
    }
    @Test void htmlMalformedJsonAndMissingSnapshotFieldsCannotBecomeEmptyRoutes() {
        type="text/html";body="<html>synthetic-private-credential</html>";
        assertTrue(assertThrows(Controller.Failure.class,()->controller.fetch()).getMessage().contains("application/json"));
        type="application/json; charset=utf-8";
        for(String invalid:List.of("synthetic-private-credential","null","[]","{}","{\"revision\":1}","{\"routes\":[]}","{\"revision\":1,\"routes\":[{}]}")) {
            body=invalid;assertFalse(assertThrows(Controller.Failure.class,()->controller.fetch()).getMessage().contains("synthetic-private"));
        }
        body=" ".repeat(256*1024+1);assertTrue(assertThrows(Controller.Failure.class,()->controller.fetch()).getMessage().contains("size limit"));
    }
    @Test void reportNeedsActualBooleanTrueNotJustHttp200() {
        for(String invalid:List.of("{}","{\"ok\":false}","{\"ok\":\"true\"}","{\"ok\":1}")) {
            body=invalid;assertTrue(assertThrows(Controller.Failure.class,()->controller.report(Map.of())).getMessage().contains("acknowledge"));
        }
        type="text/html";body="login";assertThrows(Controller.Failure.class,()->controller.report(Map.of()));
    }
    @Test void networkErrorsExplainTlsTimeoutDnsAndConnectionWithoutPrivateMessages() {
        assertTrue(Controller.transportError(new SSLHandshakeException("synthetic-private")).contains("TLS"));
        assertTrue(Controller.transportError(new HttpTimeoutException("synthetic-private")).contains("timed out"));
        assertTrue(Controller.transportError(new UnknownHostException("synthetic-private")).contains("resolved"));
        assertTrue(Controller.transportError(new ConnectException("synthetic-private")).contains("connection failed"));
    }
}
