package ak;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * Just enough JSON to read `ffi/schema/generated/manifest.json` at run time.
 *
 * <p>At run time on purpose. The alternative was to bake the expected hashes into a
 * generated Java file, which turns "the manifest moved and this slice did not" from a
 * failed assertion into a silent agreement between two copies of the same stale number.
 * The manifest is the oracle; a slice reads the oracle.
 */
public final class Json {
  private final String s;
  private int i;

  private Json(String s) { this.s = s; }

  public static Object parse(String text) {
    Json p = new Json(text);
    p.ws();
    Object v = p.value();
    p.ws();
    if (p.i != text.length()) throw new IllegalArgumentException("trailing JSON at " + p.i);
    return v;
  }

  private void ws() { while (i < s.length() && Character.isWhitespace(s.charAt(i))) i++; }

  private Object value() {
    char c = s.charAt(i);
    switch (c) {
      case '{': return object();
      case '[': return array();
      case '"': return string();
      case 't': i += 4; return Boolean.TRUE;
      case 'f': i += 5; return Boolean.FALSE;
      case 'n': i += 4; return null;
      default: return number();
    }
  }

  private Map<String, Object> object() {
    Map<String, Object> m = new LinkedHashMap<String, Object>();
    i++;  // {
    ws();
    if (s.charAt(i) == '}') { i++; return m; }
    while (true) {
      ws();
      String k = string();
      ws();
      i++;  // :
      ws();
      m.put(k, value());
      ws();
      char c = s.charAt(i++);
      if (c == '}') return m;
      if (c != ',') throw new IllegalArgumentException("expected , or } at " + i);
    }
  }

  private List<Object> array() {
    List<Object> a = new ArrayList<Object>();
    i++;  // [
    ws();
    if (s.charAt(i) == ']') { i++; return a; }
    while (true) {
      ws();
      a.add(value());
      ws();
      char c = s.charAt(i++);
      if (c == ']') return a;
      if (c != ',') throw new IllegalArgumentException("expected , or ] at " + i);
    }
  }

  private String string() {
    StringBuilder sb = new StringBuilder();
    i++;  // "
    while (true) {
      char c = s.charAt(i++);
      if (c == '"') return sb.toString();
      if (c != '\\') { sb.append(c); continue; }
      char e = s.charAt(i++);
      switch (e) {
        case 'n': sb.append('\n'); break;
        case 't': sb.append('\t'); break;
        case 'r': sb.append('\r'); break;
        case 'b': sb.append('\b'); break;
        case 'f': sb.append('\f'); break;
        case 'u': sb.append((char) Integer.parseInt(s.substring(i, i + 4), 16)); i += 4; break;
        default: sb.append(e);
      }
    }
  }

  private Object number() {
    int st = i;
    while (i < s.length() && "+-.eE0123456789".indexOf(s.charAt(i)) >= 0) i++;
    String t = s.substring(st, i);
    if (t.indexOf('.') >= 0 || t.indexOf('e') >= 0 || t.indexOf('E') >= 0)
      return Double.valueOf(t);
    return Long.valueOf(t);
  }
}
