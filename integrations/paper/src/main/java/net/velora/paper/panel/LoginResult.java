package net.velora.paper.panel;

public record LoginResult(
        boolean allowed,
        String message
) {
    public static LoginResult allow() {
        return new LoginResult(true, null);
    }

    public static LoginResult deny(String message) {
        return new LoginResult(false, message);
    }
}
