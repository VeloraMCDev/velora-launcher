package net.scopenet.paper.compat;

import com.google.gson.JsonObject;
import org.bukkit.Bukkit;
import org.bukkit.plugin.RegisteredServiceProvider;
import java.util.Map;

/**
 * Sends Spark's performance numbers (TPS, tick time, CPU) to the panel so admins can watch
 * server health from the dashboard. Spark is read through reflection, so any Spark
 * version that has the statistics API works.
 */
public final class SparkModule implements IntegrationModule {
    private CompatContext context;
    private Object spark;
    private int task = -1;
    private volatile double lastTps = -1;

    @Override public String id() { return "spark"; }
    @Override public String displayName() { return "Spark"; }
    @Override public String pluginName() { return "spark"; }

    @Override @SuppressWarnings({"unchecked", "rawtypes"})
    public void enable(CompatContext context) throws Exception {
        this.context = context;
        Class sparkType = Reflect.load("me.lucko.spark.api.Spark", Bukkit.getPluginManager().getPlugin("spark").getClass().getClassLoader());
        RegisteredServiceProvider<?> provider = Bukkit.getServicesManager().getRegistration(sparkType);
        if (provider == null) throw new IllegalStateException("Spark's API isn't registered");
        spark = provider.getProvider();
        task = context.everyAsync(200, 20L * Math.max(10, context.getInt("spark", "report-interval-seconds", 30)), this::report);
    }

    @Override public void disable() {
        if (context != null && task >= 0) context.cancel(task);
        task = -1;
        spark = null;
    }

    @Override public Map<String, Object> details() {
        return lastTps < 0 ? Map.of() : Map.of("tps", Math.round(lastTps * 100) / 100.0);
    }

    /** Reads one window of a statistic, e.g. poll(TicksPerSecond.MINUTES_1). Returns null if it isn't available. */
    private static Object poll(Object statistic, String windowType, String window) {
        if (statistic == null) return null;
        try {
            Class<?> type = Class.forName("me.lucko.spark.api.statistic.StatisticWindow$" + windowType, true, statistic.getClass().getClassLoader());
            for (Object constant : type.getEnumConstants()) {
                if (constant.toString().equals(window)) return Reflect.call(statistic, "poll", constant);
            }
        } catch (Exception ignored) { /* this Spark doesn't have that window */ }
        return null;
    }

    private static void put(JsonObject to, String key, Object value) {
        if (value instanceof Number n && !Double.isNaN(n.doubleValue())) to.addProperty(key, Math.round(n.doubleValue() * 100) / 100.0);
    }

    private static JsonObject average(Object info) {
        JsonObject o = new JsonObject();
        if (info == null) return o;
        put(o, "mean", Reflect.tryCall(info, "mean"));
        put(o, "median", Reflect.tryCall(info, "median"));
        put(o, "p95", Reflect.tryCall(info, "percentile95th"));
        put(o, "max", Reflect.tryCall(info, "max"));
        return o;
    }

    private void report() {
        try {
            JsonObject data = new JsonObject();
            JsonObject tps = new JsonObject();
            Object tpsStat = Reflect.tryCall(spark, "tps");
            for (String[] w : new String[][]{{"SECONDS_5", "s5"}, {"SECONDS_10", "s10"}, {"MINUTES_1", "m1"}, {"MINUTES_5", "m5"}, {"MINUTES_15", "m15"}}) {
                put(tps, w[1], poll(tpsStat, "TicksPerSecond", w[0]));
            }
            data.add("tps", tps);
            if (tps.has("m1")) lastTps = tps.get("m1").getAsDouble();

            JsonObject mspt = new JsonObject();
            Object msptStat = Reflect.tryCall(spark, "mspt");
            mspt.add("s10", average(poll(msptStat, "MillisPerTick", "SECONDS_10")));
            mspt.add("m1", average(poll(msptStat, "MillisPerTick", "MINUTES_1")));
            data.add("mspt", mspt);

            JsonObject cpu = new JsonObject();
            for (String kind : new String[]{"process", "system"}) {
                JsonObject o = new JsonObject();
                Object stat = Reflect.tryCall(spark, kind.equals("process") ? "cpuProcess" : "cpuSystem");
                put(o, "s10", poll(stat, "CpuUsage", "SECONDS_10"));
                put(o, "m1", poll(stat, "CpuUsage", "MINUTES_1"));
                put(o, "m15", poll(stat, "CpuUsage", "MINUTES_15"));
                cpu.add(kind, o);
            }
            data.add("cpu", cpu);

            Runtime rt = Runtime.getRuntime();
            JsonObject memory = new JsonObject();
            memory.addProperty("used_mb", (rt.totalMemory() - rt.freeMemory()) / (1024 * 1024));
            memory.addProperty("max_mb", rt.maxMemory() / (1024 * 1024));
            data.add("memory", memory);
            context.report("spark", Bukkit.getPluginManager().getPlugin("spark").getDescription().getVersion(), data);
        } catch (Exception e) {
            context.log().fine("Spark report failed: " + e.getMessage());
        }
    }
}
