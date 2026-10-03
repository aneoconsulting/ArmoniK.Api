package ak;

/** perf task-clock of the whole process (CAMPAIGN req 21 as amended 2026-10-01), opened by the
 *  JVM agent native/taskclock.c (-agentpath) before the JVM's threads exist; see that file. */
public final class TaskClock {
  private TaskClock() {}
  private static boolean loaded;

  /** Loads the agent library (-Dak.taskclock.lib, the same path as -agentpath) and checks
   *  that the agent opened the counter; throws otherwise. */
  public static synchronized void ensureOpen() {
    if (!loaded) {
      String p = System.getProperty("ak.taskclock.lib");
      if (p == null) throw new IllegalStateException("-Dak.taskclock.lib is not set");
      System.load(p);
      loaded = true;
    }
    int s = status();
    if (s == -1000) throw new IllegalStateException("task-clock: the agent did not run (-agentpath:" + System.getProperty("ak.taskclock.lib") + " missing)");
    if (s < 0) throw new IllegalStateException("task-clock: perf_event_open failed, errno " + (-s));
  }

  public static native long ns();
  public static native int status();
}
