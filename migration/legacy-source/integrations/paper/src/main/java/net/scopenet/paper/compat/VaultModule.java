package net.scopenet.paper.compat;

import net.milkbowl.vault.economy.Economy;
import net.scopenet.integration.OperationLedger;
import org.bukkit.Bukkit;
import org.bukkit.plugin.ServicePriority;
import java.util.Map;

/** Registers the SCOPENET economy as Vault's economy provider. */
public final class VaultModule implements IntegrationModule {
    private ScopenetEconomy economy;
    private OperationLedger ledger;
    private CompatContext context;

    @Override public String id() { return "vault"; }
    @Override public String displayName() { return "Vault"; }
    @Override public String pluginName() { return "Vault"; }

    @Override public void enable(CompatContext context) throws Exception {
        this.context = context;
        String symbol = context.config().getString("economy.currency-symbol", "$");
        // The economy needs the ledger's callback and the ledger needs the economy: break the cycle with a holder.
        ScopenetEconomy[] holder = new ScopenetEconomy[1];
        ledger = new OperationLedger(context.dataFolder().resolve("vault-pending.json"), context.integration().client(),
                context.log()::warning, (id, result) -> {
                    if (holder[0] != null) holder[0].settled(id, result);
                });
        economy = new ScopenetEconomy(context, ledger, symbol);
        holder[0] = economy;
        // High priority: if another economy is installed too, admins can still pick with Vault's own config.
        Bukkit.getServicesManager().register(Economy.class, economy, context.plugin(), ServicePriority.High);
    }

    @Override public void disable() {
        if (economy != null) {
            Bukkit.getServicesManager().unregister(Economy.class, economy);
            economy = null;
        }
        if (ledger != null) {
            ledger.close();
            ledger = null;
        }
    }

    @Override public Map<String, Object> details() {
        return Map.of("unsent_transactions", ledger == null ? 0 : ledger.size());
    }
}
