package ak;

import java.io.File;
import java.util.Arrays;
import java.util.Map;
import java.util.TreeMap;

/**
 * Decision 11 on the JVM (FIX-PLAN WP5 step 9): the controls the corpus cannot express.
 * Over the corpus description ({@code ak.corpus}), against the corpus core ({@code -Dak.lib}).
 *
 * <ol>
 *   <li><b>per-position discard</b>: for every accept row of class unknown whose root crosses
 *       the C ABI, and every position of that root, decode with that one position's entry
 *       left all zero and re-encode. Checked: every position zeroed gives exactly the drop
 *       arm's bytes; no position zeroed gives the full-retain bytes; one position zeroed
 *       gives bytes between the two (never longer than full retain, never shorter than
 *       drop); the push and pull deliveries agree byte for byte under every mask. The rows
 *       and positions where zeroing changed the output are counted.</li>
 *   <li><b>wrong root</b> (rule 6): a context bound to one root, used to reset, parse or decode
 *       another root, is refused with AK_ERR_INVALID_STATE (-8), and still decodes its own.</li>
 * </ol>
 * {@code -Dak.unk.plant=1} is a planted defect for the gate: the mask is ignored (every
 * position stays armed), so the all-zero check must fail.
 */
public final class RunUnkControls {
  public static void main(String[] args) throws Exception {
    boolean plant = "1".equals(System.getProperty("ak.unk.plant"));
    File dir = new File(CampaignCodec.CORPUS_DIR);
    @SuppressWarnings("unchecked")
    Map<String, Object> vs = (Map<String, Object>) ((Map<String, Object>) Json.parse(
        new String(Payloads.readFile(new File(dir, "manifest.json")), "UTF-8"))).get("vectors");
    java.util.Set<String> abi = new java.util.HashSet<String>(Arrays.asList(ak.corpus.Dispatch.ABI));
    ak.corpus.Binding drop = new ak.corpus.Binding();
    int rows = 0, pairs = 0, changedRows = 0, changedPairs = 0, bad = 0, pullBad = 0;
    for (Map.Entry<String, Object> e : new TreeMap<String, Object>(vs).entrySet()) {
      @SuppressWarnings("unchecked")
      Map<String, Object> r = (Map<String, Object>) e.getValue();
      String root = (String) r.get("root");
      if (!e.getKey().startsWith("U-") || !"unknown".equals(r.get("class"))
          || !"accept".equals(r.get("expect")) || "disputed".equals(r.get("verdict"))
          || !abi.contains(root)) continue;
      byte[] w = CampaignCodec.corpusRow(e.getKey());
      byte[] dropped = ak.corpus.Dispatch.encFfi(drop, root, ak.corpus.Dispatch.decFfi(drop, root, w));
      int ri = Arrays.asList(ak.corpus.Binding.ROOTS).indexOf(root);
      int npos = ak.corpus.Binding.UNK_POSITIONS[ri].length;
      byte[] full = run(root, w, -1L, false, plant);
      rows++;
      boolean rowChanged = false;
      long[] masks = new long[npos + 2];
      masks[0] = 0L;
      masks[1] = -1L;
      for (int i = 0; i < npos; i++) masks[i + 2] = ~(1L << i);
      for (int k = 0; k < masks.length; k++) {
        byte[] got = run(root, w, masks[k], false, plant);
        byte[] gotPull = run(root, w, masks[k], true, plant);
        if (!Arrays.equals(got, gotPull)) {
          pullBad++;
          if (pullBad <= 5) System.out.println("  PULL != PUSH " + e.getKey() + " mask " + Long.toHexString(masks[k]));
        }
        String why = null;
        if (k == 0 && !Arrays.equals(got, dropped)) why = "every position zeroed != drop";
        if (k == 1 && !Arrays.equals(got, full)) why = "no position zeroed != full retain";
        if (k >= 2) {
          pairs++;
          if (got.length > full.length || got.length < dropped.length) why = "outside [drop, full]";
          if (!Arrays.equals(got, full)) { changedPairs++; rowChanged = true; }
        }
        if (why != null) {
          bad++;
          if (bad <= 8) System.out.println("  MISMATCH " + e.getKey() + " (" + root + ") mask "
              + Long.toHexString(masks[k]) + ": " + why);
        }
      }
      if (rowChanged) changedRows++;
    }
    System.out.println("per-position discard: rows " + rows + ", (row, position) pairs " + pairs
        + ", changed by zeroing: " + changedPairs + " pair(s) in " + changedRows + " row(s)");
    System.out.println("discard mismatches: " + bad + "; pull != push: " + pullBad
        + (plant ? "   [PLANTED: the mask is ignored]" : ""));

    // ---- wrong root (rule 6)
    ak.corpus.Binding b = new ak.corpus.Binding();
    long ctx = b.contextOf("ListResultsResponse");
    byte[] w = CampaignCodec.corpusRow("U-deep-all");
    int reset = ak.corpus.NativeEntry.decResetListTasksDetailedResponse(ctx, 0L);
    int parse = ak.corpus.NativeEntry.parseListTasksDetailedResponse(b, ctx, w, 0, w.length);
    byte[] own = CampaignCodec.corpusRow("U-leaf-all");
    int ownParse = ak.corpus.NativeEntry.parseListResultsResponse(b, ctx, own, 0, own.length);
    boolean wrongOk = reset == -8 && parse == -8 && ownParse >= 0;
    System.out.println("wrong root: reset rc " + reset + ", parse rc " + parse
        + " (AK_ERR_INVALID_STATE = -8); own root still parses: rc " + ownParse
        + "  " + (wrongOk ? "PASS" : "FAIL"));
    boolean ok = bad == 0 && pullBad == 0 && wrongOk && changedPairs > 0;
    System.out.println(ok ? "UNKNOWN-FIELD CONTROLS PASSED" : "UNKNOWN-FIELD CONTROLS FAILED");
    System.exit(ok ? 0 : 1);
  }

  static byte[] run(String root, byte[] w, long mask, boolean pull, boolean plant) {
    ak.corpus.Binding b = new ak.corpus.Binding();
    b.retain = true;
    if (!plant) b.setUnkPositions(root, mask);
    Object o = pull ? ak.corpus.Dispatch.parseFfi(b, root, w) : ak.corpus.Dispatch.decFfi(b, root, w);
    byte[] out = ak.corpus.Dispatch.encFfi(b, root, o);
    b.close();
    return out;
  }
}
