package net.scopenet.paper.compat;

import java.lang.reflect.Method;

/**
 * Small reflection helpers for integrations that talk to plugins without compiling against
 * them (WorldGuard, CoreProtect, Spark): these plugins change their APIs between versions,
 * and a missing method should degrade one integration, not break SCOPENET.
 */
final class Reflect {
    private Reflect() {}

    static Object call(Object target, String name, Object... args) throws Exception {
        Method m = find(target.getClass(), name, args.length);
        if (m == null) throw new NoSuchMethodException(target.getClass().getName() + "." + name);
        m.setAccessible(true);
        return m.invoke(target, args);
    }

    /** Calls the public method whose single parameter accepts {@code arg} (for overloaded one-argument methods). */
    static Object callCompatible(Object target, String name, Object arg) throws Exception {
        for (Method m : target.getClass().getMethods()) {
            if (m.getName().equals(name) && m.getParameterCount() == 1 && m.getParameterTypes()[0].isInstance(arg)) {
                m.setAccessible(true);
                return m.invoke(target, arg);
            }
        }
        throw new NoSuchMethodException(target.getClass().getName() + "." + name + "(" + arg.getClass().getName() + ")");
    }

    /** Like {@link #call} but returns {@code null} instead of throwing. */
    static Object tryCall(Object target, String name, Object... args) {
        try { return target == null ? null : call(target, name, args); }
        catch (Exception e) { return null; }
    }

    /** First public method with this name and arity, searching interfaces too (implementation classes may be non-public). */
    static Method find(Class<?> type, String name, int arity) {
        for (Method m : type.getMethods()) {
            if (m.getName().equals(name) && m.getParameterCount() == arity) return m;
        }
        for (Class<?> c = type; c != null; c = c.getSuperclass()) {
            for (Method m : c.getDeclaredMethods()) {
                if (m.getName().equals(name) && m.getParameterCount() == arity) return m;
            }
        }
        return null;
    }

    static double number(Object value, double fallback) {
        return value instanceof Number n ? n.doubleValue() : fallback;
    }

    static Class<?> load(String name, ClassLoader loader) throws ClassNotFoundException {
        return Class.forName(name, true, loader);
    }
}
