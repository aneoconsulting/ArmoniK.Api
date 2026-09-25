package ak;

import java.io.File;
import java.util.Arrays;
import java.util.Map;
import java.util.TreeMap;

/**
 * Decision 11 rule 3 on the JVM: a decode that fails leaves every buffer the core grew with
 * the host, so the host must reclaim them. The shim's grow links each buffer it hands out
 * into the arming options' list (`host`); delivery unlinks it; after the decode the binding
 * frees what is still linked ({@code Binding.unkSettle}). This control decodes every corpus
 * row whose root crosses the C ABI -- the refused rows are the point -- in retain mode on
 * the four retain arms (push, pull drain, pull walk, borrowed facade), and after EVERY row
 * reads the shim's count of buffers alive ({@code Native.unkLive}). Checked: 0 alive after
 * every row, on every arm, before any binding is closed (closing frees the lists, which
 * would hide a leak); 0 buffers left undelivered after a decode that succeeded; and some
 * buffers were in fact reclaimed after refused decodes (otherwise the control reaches
 * nothing). The corpus's own refused rows are refused before any unknown field is taken
 * (measured: 0 reclaimed over them), so the control adds DERIVED refused rows: every
 * accept row of class unknown, with an invalid tag appended (0x0F: field 1, wire type 7)
 * and truncated at every length -- decodes that grow buffers and then fail.
 * {@code -Dak.unk.leakplant=1} (read by the generated binding) skips the reclaim,
 * so the 0-alive check must fail.
 */
public final class RunUnkLeak {
  public static void main(String[] args) throws Exception {
    boolean plant = "1".equals(System.getProperty("ak.unk.leakplant"));
    File dir = new File(CampaignCodec.CORPUS_DIR);
    @SuppressWarnings("unchecked")
    Map<String, Object> vs = (Map<String, Object>) ((Map<String, Object>) Json.parse(
        new String(Payloads.readFile(new File(dir, "manifest.json")), "UTF-8"))).get("vectors");
    java.util.Set<String> abi = new java.util.HashSet<String>(Arrays.asList(ak.corpus.Dispatch.ABI));
    String[] arms = {"ffi-retain", "ffi-pull-retain", "ffi-pull-walk-retain", "ffi-borrow-retain"};
    java.util.List<Object[]> cases = new java.util.ArrayList<Object[]>();   // {label, root, bytes, derived}
    for (Map.Entry<String, Object> e : new TreeMap<String, Object>(vs).entrySet()) {
      @SuppressWarnings("unchecked")
      Map<String, Object> r = (Map<String, Object>) e.getValue();
      String root = (String) r.get("root");
      if (!abi.contains(root)) continue;
      Object f = r.get("file");   // a baseline row points at schema/generated
      byte[] w = f != null ? Payloads.readFile(new File(dir, (String) f)) : CampaignCodec.corpusRow(e.getKey());
      cases.add(new Object[] {e.getKey(), root, w, Boolean.FALSE});
    }
    int corpusCases = cases.size();
    for (int i = 0; i < corpusCases; i++) {
      Object[] c = cases.get(i);
      @SuppressWarnings("unchecked")
      Map<String, Object> r = (Map<String, Object>) vs.get(c[0]);
      if (!((String) c[0]).startsWith("U-") || !"unknown".equals(r.get("class"))
          || !"accept".equals(r.get("expect"))) continue;
      byte[] w = (byte[]) c[2];
      byte[] bad = Arrays.copyOf(w, w.length + 1);
      bad[w.length] = 0x0F;
      cases.add(new Object[] {c[0] + "+bad-tag", c[1], bad, Boolean.TRUE});
      for (int n = 1; n < w.length; n++)
        cases.add(new Object[] {c[0] + "[:" + n + "]", c[1], Arrays.copyOf(w, n), Boolean.TRUE});
    }
    boolean ok = true;
    new ak.corpus.Binding().close();   // loads and binds the shim
    long before = Native.unkLive();
    for (String arm : arms) {
      ak.corpus.Binding b = null;
      ak.corpus.borrow.Binding bb = null;
      if (arm.equals("ffi-borrow-retain")) {
        bb = new ak.corpus.borrow.Binding();
        bb.retain = true;
      } else {
        b = new ak.corpus.Binding();
        b.retain = true;
        b.pullWalk = arm.startsWith("ffi-pull-walk");
      }
      boolean pull = arm.startsWith("ffi-pull");
      before = Native.unkLive();   // closing the previous arm's binding freed its lists
      int[] rows = new int[2], refused = new int[2];
      long[] recl = new long[2];
      int leakRows = 0;
      for (Object[] c : cases) {
        String root = (String) c[1];
        byte[] w = (byte[]) c[2];
        int g = ((Boolean) c[3]) ? 1 : 0;
        long r0 = bb != null ? bb.unkReclaimed : b.unkReclaimed;
        rows[g]++;
        try {
          if (bb != null) ak.corpus.borrow.Dispatch.decFfi(bb, root, w);
          else if (pull) ak.corpus.Dispatch.parseFfi(b, root, w);
          else ak.corpus.Dispatch.decFfi(b, root, w);
        } catch (RuntimeException x) {
          refused[g]++;
        }
        recl[g] += (bb != null ? bb.unkReclaimed : b.unkReclaimed) - r0;
        long live = Native.unkLive() - before;
        if (live != 0) {
          leakRows++;
          if (leakRows <= 3) System.out.println("  LEFT BEHIND " + arm + " " + c[0] + ": " + live + " buffer(s) alive");
          before = Native.unkLive();   // count each row's own leak, not the running total
        }
      }
      long leftOk = bb != null ? bb.unkLeftAfterSuccess : b.unkLeftAfterSuccess;
      boolean armOk = leakRows == 0 && leftOk == 0 && (recl[1] > 0 || plant);
      ok &= armOk;
      System.out.println(String.format("%-22s corpus rows %d (refused %d, buffers reclaimed %d); derived rows %d"
          + " (refused %d, buffers reclaimed %d); left undelivered after accepted decodes %d;"
          + " rows leaving buffers alive %d  %s", arm, rows[0], refused[0], recl[0], rows[1], refused[1],
          recl[1], leftOk, leakRows, armOk ? "PASS" : "FAIL"));
      if (bb != null) bb.close(); else b.close();
    }
    System.out.println("buffers alive at exit (after close): " + Native.unkLive()
        + (plant ? "   [PLANTED: nothing is reclaimed]" : ""));
    ok &= Native.unkLive() == 0 || plant;
    System.out.println(ok ? "LEAK CONTROL PASSED" : "LEAK CONTROL FAILED");
    System.exit(ok ? 0 : 1);
  }
}
