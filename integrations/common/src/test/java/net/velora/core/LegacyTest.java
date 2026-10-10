package net.velora.core;

import net.scopenet.integration.Settings;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;

import static org.junit.jupiter.api.Assertions.*;

class LegacyTest {
    @TempDir Path config;

    @Test void copiesTheOldServerConfigWithoutRemovingIt() throws IOException {
        Path old = config.resolve("scopenet.properties");
        Files.writeString(old, "panel-url=https://panel.example.com\ntoken=sn_" + "a".repeat(40) + "\n");
        Path next = config.resolve("velora-core.properties");
        Settings settings = Settings.load(next);
        assertEquals("https://panel.example.com", settings.panel().toString());
        assertTrue(Files.exists(old), "the old file stays so an older jar still works");
        assertEquals(Files.readString(old), Files.readString(next));
    }

    @Test void neverOverwritesAnExistingNewConfig() throws IOException {
        Files.writeString(config.resolve("scopenet.properties"), "token=old\n");
        Path next = config.resolve("velora-core.properties");
        Files.writeString(next, "token=new\n");
        assertFalse(Legacy.copyServerConfig(next));
        assertEquals("token=new\n", Files.readString(next));
    }

    @Test void writesTheDocumentedDefaultWhenNothingExists() throws IOException {
        Path next = config.resolve("velora-core.properties");
        Settings.ensureDefault(next);
        assertTrue(Files.readString(next).contains("modules.vaults.enabled=true"));
        IllegalArgumentException missingToken = assertThrows(IllegalArgumentException.class, () -> Settings.load(next));
        assertTrue(missingToken.getMessage().contains("config/velora-core.properties"));
    }

    @Test void movesTheDataDirectoryOnce() throws IOException {
        Path old = Files.createDirectories(config.resolve("scopenet").resolve("vaults"));
        Files.writeString(config.resolve("scopenet").resolve("essentials.json"), "{}");
        Legacy.moveDataDirectory(config);
        assertFalse(Files.exists(old));
        assertTrue(Files.exists(config.resolve("velora-core").resolve("essentials.json")));
        assertTrue(Files.isDirectory(config.resolve("velora-core").resolve("vaults")));
        Files.createDirectories(config.resolve("scopenet"));
        Legacy.moveDataDirectory(config);
        assertTrue(Files.isDirectory(config.resolve("scopenet")), "an existing new directory is never merged or replaced");
    }
}
