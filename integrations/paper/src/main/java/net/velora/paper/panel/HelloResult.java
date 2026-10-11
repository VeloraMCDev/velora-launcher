package net.velora.paper.panel;

public record HelloResult(
        boolean ok,
        long serverId,
        String serverName,
        String brand,
        boolean requireLauncher,
        String errorMessage
) {
    public static HelloResult success(long serverId, String serverName, String brand, boolean requireLauncher) {
        return new HelloResult(true, serverId, serverName, brand, requireLauncher, null);
    }

    public static HelloResult failure(String errorMessage) {
        return new HelloResult(false, -1, null, null, false, errorMessage);
    }
}
