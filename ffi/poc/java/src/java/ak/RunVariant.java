package ak;

/**
 * FIX-PLAN WP5 step 10: which build this class tree and the loaded core are, checked
 * rather than assumed. Compiled into both trees (full and no-unknown).
 *
 * <pre>
 *   java -Dak.lib=&lt;shim&gt; ak.RunVariant shapes|corpus
 * </pre>
 *
 * Prints the tree's build ({@code ak.Variant}), the number of layout facts the host
 * reproduces and the number the loaded core exports ({@code ak_layout_facts}), and runs the
 * section 10 agreement check (every fact equal). For {@code corpus}, also rule 6 on the
 * build's root-bound contexts: a context bound to ListResultsResponse refuses to parse
 * ListTasksDetailedResponse with AK_ERR_INVALID_STATE (-8) and still parses its own root.
 * Exit 1 on any disagreement.
 */
public final class RunVariant {
  static byte[] row(String id) {   // CampaignCodec is not in the floor tree
    return Payloads.readFile(new java.io.File(Payloads.SCHEMA_DIR, "../../corpus/generated/vectors/" + id + ".bin"));
  }

  public static void main(String[] args) {
    String which = args.length > 0 ? args[0] : "shapes";
    boolean ok = true;
    int host;
    if (which.equals("corpus")) {
      ak.corpus.Binding b = new ak.corpus.Binding();   // loads and binds the shim
      host = ak.corpus.Layout.HOST.length;
      int core = Native.layoutFacts(null);
      String agree;
      try { ak.corpus.Layout.assertAgreement(); agree = "agree"; }
      catch (IllegalStateException e) { agree = "DISAGREE: " + e.getMessage(); ok = false; }
      System.out.println("  " + Variant.NAME + " tree (corpus): host reproduces " + host
          + " layout facts, the core exports " + core + "; " + agree);
      long ctx = b.contextOf("ListResultsResponse");
      byte[] other = row("U-deep-all");
      byte[] own = row("U-leaf-all");
      int wrong = ak.corpus.NativeEntry.parseListTasksDetailedResponse(b, ctx, other, 0, other.length);
      int mine = ak.corpus.NativeEntry.parseListResultsResponse(b, ctx, own, 0, own.length);
      boolean r6 = wrong == -8 && mine >= 0;
      ok &= r6;
      System.out.println("  rule 6: a ListResultsResponse context parsing ListTasksDetailedResponse rc "
          + wrong + " (AK_ERR_INVALID_STATE = -8); its own root rc " + mine + "  " + (r6 ? "PASS" : "FAIL"));
      b.close();
    } else {
      ak.shapes.Binding b = new ak.shapes.Binding();
      host = ak.shapes.Layout.HOST.length;
      int core = Native.layoutFacts(null);
      String agree;
      try { ak.shapes.Layout.assertAgreement(); agree = "agree"; }
      catch (IllegalStateException e) { agree = "DISAGREE: " + e.getMessage(); ok = false; }
      System.out.println("  " + Variant.NAME + " tree (shapes): host reproduces " + host
          + " layout facts, the core exports " + core + "; " + agree);
      b.close();
    }
    System.out.println(ok ? "VARIANT CHECK PASSED" : "VARIANT CHECK FAILED");
    System.exit(ok ? 0 : 1);
  }
}
