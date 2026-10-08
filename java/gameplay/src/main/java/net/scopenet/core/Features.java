package net.scopenet.core;

/** Which systems are switched on, plus small display settings. */
public record Features(boolean essentials, boolean economy, boolean guilds, boolean social, boolean landClaiming,
                       boolean clientLink, String currencySymbol) {
    public static Features all() { return new Features(true, true, true, true, true, true, "$"); }
}
