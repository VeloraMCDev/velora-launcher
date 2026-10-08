package net.scopenet.api;

/** How many of a player's quests for the current period they have finished. */
public record QuestProgress(int total, int completed, int claimed) {
    /** "3/5", handy for scoreboards. */
    public String display() { return completed + "/" + total; }
}
