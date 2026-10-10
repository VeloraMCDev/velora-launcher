package net.velora.core;

import com.google.gson.JsonParser;
import java.util.Properties;
import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

class CoreModulesTest {
    @Test void localSettingsCannotOverridePanelDenial() {
        var response = JsonParser.parseString("{\"velora_core\":{\"schema\":1,\"modules\":{\"map\":true,\"economy\":false,\"vaults\":true,\"casino\":false,\"analytics\":true,\"factions\":false,\"permissions_chat\":true}}}").getAsJsonObject();
        var policy = CoreModules.all().intersect(CoreModules.fromPanel(response));
        assertFalse(policy.commandEnabled("market")); assertFalse(policy.commandEnabled("guild")); assertTrue(policy.commandEnabled("vault"));
        var local = new Properties(); local.setProperty("modules.vaults.enabled", "false");
        assertFalse(CoreModules.local(local).intersect(policy).commandEnabled("vault"));
    }
    @Test void malformedAndUnknownSchemasFailClosedButLegacyRemainsCompatible() {
        for (String raw : new String[]{"{\"velora_core\":{\"schema\":2}}", "{\"velora_core\":{\"schema\":1,\"modules\":{}}}"}) {
            assertFalse(CoreModules.fromPanel(JsonParser.parseString(raw).getAsJsonObject()).enabled("economy"));
        }
        assertTrue(CoreModules.fromPanel(JsonParser.parseString("{}").getAsJsonObject()).enabled("economy"));
        assertFalse(CoreModules.pending().enabled("vaults"));
        var local = new Properties(); local.setProperty("modules.map.enabled", "typo");
        assertThrows(IllegalArgumentException.class, () -> CoreModules.local(local));
    }
}
