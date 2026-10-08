package net.scopenet.api;

/** Which level track an XP change applies to. */
public enum XpScope {
    /** The account-wide SCOPENET level, shared by every server. */
    GLOBAL,
    /** The level on the server the plugin is running on. */
    SERVER
}
