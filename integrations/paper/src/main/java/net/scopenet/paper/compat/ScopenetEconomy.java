package net.scopenet.paper.compat;

import com.google.gson.JsonObject;
import net.milkbowl.vault.economy.Economy;
import net.milkbowl.vault.economy.EconomyResponse;
import net.milkbowl.vault.economy.EconomyResponse.ResponseType;
import net.scopenet.integration.OperationLedger;
import net.scopenet.integration.PanelClient;
import org.bukkit.Bukkit;
import org.bukkit.OfflinePlayer;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;

/**
 * Velora's server economy as a Vault provider, so shops, jobs, auction and every other
 * Vault plugin use (and pay into) the same balances as /balance and the launcher.
 *
 * Vault calls are synchronous and arrive on the server thread, so balances are answered from a
 * short-lived cache, and changes are applied to the cache immediately and sent to the panel
 * through a durable {@link OperationLedger} (saved first, retried until accepted, applied once).
 */
public final class ScopenetEconomy implements Economy {
    private record Balance(double amount, long at) {}

    private static final long REFRESH_MS = 10_000;
    private final CompatContext context;
    private final PanelClient client;
    private final OperationLedger ledger;
    private final String symbol;
    private final Map<UUID, Balance> cache = new ConcurrentHashMap<>();
    private final Map<String, UUID> inFlight = new ConcurrentHashMap<>();
    private final Map<UUID, Boolean> refreshing = new ConcurrentHashMap<>();

    public ScopenetEconomy(CompatContext context, OperationLedger ledger, String symbol) {
        this.context = context;
        this.client = context.integration().client();
        this.ledger = ledger;
        this.symbol = symbol;
    }

    /** Ledger callback: the panel settled an operation, so trust its balance again. */
    public void settled(String operationId, JsonObject result) {
        UUID uuid = inFlight.remove(operationId);
        if (uuid == null) return;
        if (result.has("error")) {
            context.log().warning("The panel refused a Vault transaction for " + uuid + ": " + result.get("error").getAsString());
            cache.remove(uuid);
        } else if (result.has("balance") && result.get("balance").isJsonPrimitive()) {
            // Only adopt the panel's number when nothing else is still in flight for this player.
            if (!inFlight.containsValue(uuid)) cache.put(uuid, new Balance(result.get("balance").getAsDouble(), System.currentTimeMillis()));
        }
    }

    private double balanceOf(UUID uuid) {
        Balance cached = cache.get(uuid);
        if (cached == null) {
            // First sight of this player: Vault needs an answer now, so ask the panel directly (briefly).
            try {
                double fetched = client.getBalance(uuid, "");
                cache.put(uuid, new Balance(fetched, System.currentTimeMillis()));
                return fetched;
            } catch (Exception e) {
                return 0.0;
            }
        }
        if (System.currentTimeMillis() - cached.at() > REFRESH_MS && !inFlight.containsValue(uuid) && refreshing.putIfAbsent(uuid, true) == null) {
            context.async(() -> {
                try { cache.put(uuid, new Balance(client.getBalance(uuid, ""), System.currentTimeMillis())); }
                catch (Exception ignored) { /* keep serving the cached value */ }
                finally { refreshing.remove(uuid); }
            });
        }
        return cached.amount();
    }

    private static double round(double amount) {
        return Math.round(amount * 100.0) / 100.0;
    }

    private EconomyResponse change(OfflinePlayer player, double amount, boolean deposit) {
        if (Double.isNaN(amount) || Double.isInfinite(amount) || amount < 0) {
            return new EconomyResponse(0, balanceOf(player.getUniqueId()), ResponseType.FAILURE, "Invalid amount");
        }
        amount = round(amount);
        UUID uuid = player.getUniqueId();
        double before = balanceOf(uuid);
        if (!deposit && before + 1e-9 < amount) {
            return new EconomyResponse(0, before, ResponseType.FAILURE, "Insufficient funds");
        }
        if (amount == 0) return new EconomyResponse(0, before, ResponseType.SUCCESS, "");
        double after = round(before + (deposit ? amount : -amount));
        JsonObject payload = new JsonObject();
        payload.addProperty("uuid", uuid.toString());
        payload.addProperty("username", player.getName() == null ? uuid.toString() : player.getName());
        payload.addProperty("delta", deposit ? amount : -amount);
        payload.addProperty("description", "Vault: " + (deposit ? "deposit" : "withdrawal"));
        try {
            String id = ledger.submit("economy/adjust", payload);
            inFlight.put(id, uuid);
        } catch (Exception e) {
            return new EconomyResponse(0, before, ResponseType.FAILURE, "Could not save the transaction: " + e.getMessage());
        }
        cache.put(uuid, new Balance(after, System.currentTimeMillis()));
        return new EconomyResponse(amount, after, ResponseType.SUCCESS, "");
    }

    @SuppressWarnings("deprecation")
    private static OfflinePlayer named(String name) {
        return Bukkit.getOfflinePlayer(name);
    }

    // ---- Economy ----

    @Override public boolean isEnabled() { return context.plugin().isEnabled(); }
    @Override public String getName() { return "Velora"; }
    @Override public boolean hasBankSupport() { return false; }
    @Override public int fractionalDigits() { return 2; }
    @Override public String format(double amount) { return symbol + String.format(Locale.US, "%,.2f", amount); }
    @Override public String currencyNamePlural() { return "dollars"; }
    @Override public String currencyNameSingular() { return "dollar"; }

    @Override public boolean hasAccount(String playerName) { return true; }
    @Override public boolean hasAccount(OfflinePlayer player) { return true; }
    @Override public boolean hasAccount(String playerName, String worldName) { return true; }
    @Override public boolean hasAccount(OfflinePlayer player, String worldName) { return true; }

    @Override public double getBalance(String playerName) { return balanceOf(named(playerName).getUniqueId()); }
    @Override public double getBalance(OfflinePlayer player) { return balanceOf(player.getUniqueId()); }
    @Override public double getBalance(String playerName, String world) { return getBalance(playerName); }
    @Override public double getBalance(OfflinePlayer player, String world) { return getBalance(player); }

    @Override public boolean has(String playerName, double amount) { return getBalance(playerName) + 1e-9 >= amount; }
    @Override public boolean has(OfflinePlayer player, double amount) { return getBalance(player) + 1e-9 >= amount; }
    @Override public boolean has(String playerName, String worldName, double amount) { return has(playerName, amount); }
    @Override public boolean has(OfflinePlayer player, String worldName, double amount) { return has(player, amount); }

    @Override public EconomyResponse withdrawPlayer(String playerName, double amount) { return change(named(playerName), amount, false); }
    @Override public EconomyResponse withdrawPlayer(OfflinePlayer player, double amount) { return change(player, amount, false); }
    @Override public EconomyResponse withdrawPlayer(String playerName, String worldName, double amount) { return withdrawPlayer(playerName, amount); }
    @Override public EconomyResponse withdrawPlayer(OfflinePlayer player, String worldName, double amount) { return withdrawPlayer(player, amount); }

    @Override public EconomyResponse depositPlayer(String playerName, double amount) { return change(named(playerName), amount, true); }
    @Override public EconomyResponse depositPlayer(OfflinePlayer player, double amount) { return change(player, amount, true); }
    @Override public EconomyResponse depositPlayer(String playerName, String worldName, double amount) { return depositPlayer(playerName, amount); }
    @Override public EconomyResponse depositPlayer(OfflinePlayer player, String worldName, double amount) { return depositPlayer(player, amount); }

    // Guild banks are managed with /guild bank, not through Vault's bank API.
    private static EconomyResponse noBanks() {
        return new EconomyResponse(0, 0, ResponseType.NOT_IMPLEMENTED, "Velora guild banks are managed with /guild bank");
    }

    @Override public EconomyResponse createBank(String name, String player) { return noBanks(); }
    @Override public EconomyResponse createBank(String name, OfflinePlayer player) { return noBanks(); }
    @Override public EconomyResponse deleteBank(String name) { return noBanks(); }
    @Override public EconomyResponse bankBalance(String name) { return noBanks(); }
    @Override public EconomyResponse bankHas(String name, double amount) { return noBanks(); }
    @Override public EconomyResponse bankWithdraw(String name, double amount) { return noBanks(); }
    @Override public EconomyResponse bankDeposit(String name, double amount) { return noBanks(); }
    @Override public EconomyResponse isBankOwner(String name, String playerName) { return noBanks(); }
    @Override public EconomyResponse isBankOwner(String name, OfflinePlayer player) { return noBanks(); }
    @Override public EconomyResponse isBankMember(String name, String playerName) { return noBanks(); }
    @Override public EconomyResponse isBankMember(String name, OfflinePlayer player) { return noBanks(); }
    @Override public List<String> getBanks() { return List.of(); }

    // Accounts are created on first use by the panel.
    @Override public boolean createPlayerAccount(String playerName) { return true; }
    @Override public boolean createPlayerAccount(OfflinePlayer player) { return true; }
    @Override public boolean createPlayerAccount(String playerName, String worldName) { return true; }
    @Override public boolean createPlayerAccount(OfflinePlayer player, String worldName) { return true; }
}
