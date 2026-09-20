// A minimal JSON reader, because the corpus has to run on the FLOOR too.
//
// `System.Text.Json` is the obvious answer and it is the wrong one here:
// .NET Framework 4.8 does not have it in the box, so taking it would either
// restrict corpus conformance to arm a or add a package to the floor arm and
// change what the floor arm is. README 5.2's arms are only comparable if the
// floor build's dependency set is the floor's.
//
// So: objects, arrays, strings, numbers, true/false/null, and nothing else. No
// writer, no streaming, no comments, no big-number handling beyond keeping the
// literal text -- which is what the projections need anyway, since
// `ffi/corpus/CONTRACT.md` section 3 encodes every integer AS a decimal string
// and only `bool` as a JSON literal.
//
// The existing `Manifest.cs` hand-scans its file with IndexOf instead. That was
// fine for a flat table of six fields per row and is not fine for the corpus
// manifest, which nests four levels.

using System;
using System.Collections.Generic;
using System.Globalization;
using System.Text;

namespace Armonik.Ffi.Harness;

/// A parsed JSON value. `Obj`, `Arr` and `Str` are the shapes the corpus uses;
/// `Num` keeps the literal text so that a 64-bit integer does not become a
/// double on the way through.
public sealed class Json
{
    public enum K { Null, Bool, Num, Str, Arr, Obj }

    public K Kind;
    public bool Bool;
    public string Text;                       // Num (literal) and Str (decoded)
    public List<Json> Arr;
    public Dictionary<string, Json> Obj;

    public static Json Parse(string s)
    {
        int i = 0;
        var v = ParseValue(s, ref i);
        SkipWs(s, ref i);
        if (i != s.Length) throw new FormatException("trailing input at " + i);
        return v;
    }

    public Json this[string key]
    {
        get
        {
            Json v;
            return Obj != null && Obj.TryGetValue(key, out v) ? v : null;
        }
    }

    public string AsString => Kind == K.Str || Kind == K.Num ? Text : null;

    public int AsInt => Kind == K.Num || Kind == K.Str
        ? int.Parse(Text, CultureInfo.InvariantCulture) : 0;

    public IEnumerable<string> Keys => Obj != null ? (IEnumerable<string>)Obj.Keys : new string[0];

    private static void SkipWs(string s, ref int i)
    {
        while (i < s.Length && (s[i] == ' ' || s[i] == '\t' || s[i] == '\n' || s[i] == '\r')) i++;
    }

    private static Json ParseValue(string s, ref int i)
    {
        SkipWs(s, ref i);
        if (i >= s.Length) throw new FormatException("unexpected end");
        char c = s[i];
        switch (c)
        {
            case '{': return ParseObj(s, ref i);
            case '[': return ParseArr(s, ref i);
            case '"': return new Json { Kind = K.Str, Text = ParseStr(s, ref i) };
            case 't': Expect(s, ref i, "true"); return new Json { Kind = K.Bool, Bool = true };
            case 'f': Expect(s, ref i, "false"); return new Json { Kind = K.Bool, Bool = false };
            case 'n': Expect(s, ref i, "null"); return new Json { Kind = K.Null };
            default: return ParseNum(s, ref i);
        }
    }

    private static void Expect(string s, ref int i, string lit)
    {
        if (i + lit.Length > s.Length || string.CompareOrdinal(s, i, lit, 0, lit.Length) != 0)
            throw new FormatException("expected " + lit + " at " + i);
        i += lit.Length;
    }

    private static Json ParseObj(string s, ref int i)
    {
        var o = new Dictionary<string, Json>(StringComparer.Ordinal);
        i++;                                     // '{'
        SkipWs(s, ref i);
        if (i < s.Length && s[i] == '}') { i++; return new Json { Kind = K.Obj, Obj = o }; }
        while (true)
        {
            SkipWs(s, ref i);
            string k = ParseStr(s, ref i);
            SkipWs(s, ref i);
            if (s[i] != ':') throw new FormatException("expected : at " + i);
            i++;
            o[k] = ParseValue(s, ref i);
            SkipWs(s, ref i);
            if (i >= s.Length) throw new FormatException("unterminated object");
            if (s[i] == ',') { i++; continue; }
            if (s[i] == '}') { i++; return new Json { Kind = K.Obj, Obj = o }; }
            throw new FormatException("expected , or } at " + i);
        }
    }

    private static Json ParseArr(string s, ref int i)
    {
        var a = new List<Json>();
        i++;                                     // '['
        SkipWs(s, ref i);
        if (i < s.Length && s[i] == ']') { i++; return new Json { Kind = K.Arr, Arr = a }; }
        while (true)
        {
            a.Add(ParseValue(s, ref i));
            SkipWs(s, ref i);
            if (i >= s.Length) throw new FormatException("unterminated array");
            if (s[i] == ',') { i++; continue; }
            if (s[i] == ']') { i++; return new Json { Kind = K.Arr, Arr = a }; }
            throw new FormatException("expected , or ] at " + i);
        }
    }

    private static string ParseStr(string s, ref int i)
    {
        if (s[i] != '"') throw new FormatException("expected string at " + i);
        i++;
        var sb = new StringBuilder();
        while (true)
        {
            if (i >= s.Length) throw new FormatException("unterminated string");
            char c = s[i++];
            if (c == '"') return sb.ToString();
            if (c != '\\') { sb.Append(c); continue; }
            char e = s[i++];
            switch (e)
            {
                case '"': sb.Append('"'); break;
                case '\\': sb.Append('\\'); break;
                case '/': sb.Append('/'); break;
                case 'b': sb.Append('\b'); break;
                case 'f': sb.Append('\f'); break;
                case 'n': sb.Append('\n'); break;
                case 'r': sb.Append('\r'); break;
                case 't': sb.Append('\t'); break;
                case 'u':
                    // Kept as the raw code unit, surrogate or not. The transcode
                    // class turns on lone surrogates surviving this step.
                    sb.Append((char)ushort.Parse(s.Substring(i, 4), NumberStyles.HexNumber,
                                                 CultureInfo.InvariantCulture));
                    i += 4;
                    break;
                default: throw new FormatException("bad escape \\" + e);
            }
        }
    }

    private static Json ParseNum(string s, ref int i)
    {
        int start = i;
        if (i < s.Length && (s[i] == '-' || s[i] == '+')) i++;
        while (i < s.Length && (char.IsDigit(s[i]) || s[i] == '.' || s[i] == 'e' || s[i] == 'E'
                                || s[i] == '+' || s[i] == '-')) i++;
        if (i == start) throw new FormatException("bad value at " + i);
        return new Json { Kind = K.Num, Text = s.Substring(start, i - start) };
    }
}
