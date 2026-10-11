package net.velora.gateway;
import org.junit.jupiter.api.Test;
import java.util.List;
import static org.junit.jupiter.api.Assertions.*;
class RoutesTest {
    @Test void normalization() {
        assertEquals("smp.velora.scopedd.lol", Routes.hostname("SMP.VELORA.SCOPEDD.LOL.:25565"));
        for (String h : List.of("", "*.example.com", "a..com", "a.com:0", "a.com:65536", "a.com\u0000forge")) assertThrows(IllegalArgumentException.class, () -> Routes.hostname(h));
    }
    @Test void atomicValidation() {
        var r = new Routes.Route("smp", "SMP", "smp.velora.scopedd.lol", "100.64.0.2", 25565, true);
        assertEquals(1, Routes.validate(new Routes.Snapshot(1, List.of(r))).size());
        assertThrows(IllegalArgumentException.class, () -> Routes.validate(new Routes.Snapshot(1, List.of(r,r))));
        var publicRoute = new Routes.Route("smp", "SMP", r.hostname(), "8.8.8.8", 25565, true);
        assertThrows(IllegalArgumentException.class, () -> Routes.validate(new Routes.Snapshot(1, List.of(publicRoute))));
    }
}
