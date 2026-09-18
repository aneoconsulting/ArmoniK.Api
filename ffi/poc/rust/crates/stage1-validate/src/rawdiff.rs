//! A protobuf-aware diff, so a mismatch names a field rather than a byte offset.
//!
//! Flattens a buffer into (tag path, wire type, body) in wire order, descending into any
//! length-delimited field that parses as a message -- the same heuristic
//! `ffi/schema/emit/check.py` uses, and it can be wrong the same way, which is why the
//! byte offset is reported too.

pub struct Item {
    pub path: String,
    pub wire: u8,
    pub body: Vec<u8>,
    pub off: usize,
}

fn varint(b: &[u8], i: &mut usize) -> Option<u64> {
    let (mut n, mut shift) = (0u64, 0u32);
    loop {
        let c = *b.get(*i)?;
        *i += 1;
        n |= ((c & 0x7f) as u64) << shift;
        if c & 0x80 == 0 {
            return Some(n);
        }
        shift += 7;
        if shift > 63 {
            return None;
        }
    }
}

pub fn flatten(b: &[u8], prefix: &str, base: usize, out: &mut Vec<Item>) -> bool {
    let mut i = 0usize;
    while i < b.len() {
        let start = i;
        let Some(k) = varint(b, &mut i) else { return false };
        let (tag, wire) = (k >> 3, (k & 7) as u8);
        if tag == 0 {
            return false;
        }
        let path = if prefix.is_empty() {
            format!("{tag}")
        } else {
            format!("{prefix}.{tag}")
        };
        match wire {
            0 => {
                let s = i;
                if varint(b, &mut i).is_none() {
                    return false;
                }
                out.push(Item { path, wire, body: b[s..i].to_vec(), off: base + start });
            }
            1 | 5 => {
                let n = if wire == 1 { 8 } else { 4 };
                if i + n > b.len() {
                    return false;
                }
                out.push(Item { path, wire, body: b[i..i + n].to_vec(), off: base + start });
                i += n;
            }
            2 => {
                let Some(n) = varint(b, &mut i) else { return false };
                let n = n as usize;
                if i + n > b.len() {
                    return false;
                }
                let body = &b[i..i + n];
                let mut nested = Vec::new();
                let looks_msg = !body.is_empty()
                    && flatten(body, &path, base + i, &mut nested)
                    && !nested.is_empty();
                if looks_msg {
                    out.push(Item { path: format!("{path}{{"), wire, body: vec![], off: base + start });
                    out.append(&mut nested);
                } else {
                    out.push(Item { path, wire, body: body.to_vec(), off: base + start });
                }
                i += n;
            }
            _ => return false,
        }
    }
    true
}

fn show(b: &[u8]) -> String {
    let hex: String = b.iter().take(24).map(|c| format!("{c:02x}")).collect();
    let ascii: String = b
        .iter()
        .take(24)
        .map(|&c| if (0x20..0x7f).contains(&c) { c as char } else { '.' })
        .collect();
    let more = if b.len() > 24 { format!(" (+{} B)", b.len() - 24) } else { String::new() };
    format!("{hex} |{ascii}|{more}")
}

/// The first place the two buffers disagree, described as a field.
pub fn first_difference(a: &[u8], b: &[u8]) -> String {
    let (mut ia, mut ib) = (Vec::new(), Vec::new());
    let oka = flatten(a, "", 0, &mut ia);
    let okb = flatten(b, "", 0, &mut ib);
    let mut lines = vec![format!(
        "  walked: manifest {} ({} items), prost {} ({} items)",
        if oka { "ok" } else { "UNPARSEABLE" },
        ia.len(),
        if okb { "ok" } else { "UNPARSEABLE" },
        ib.len()
    )];
    for n in 0..ia.len().max(ib.len()) {
        match (ia.get(n), ib.get(n)) {
            (Some(x), Some(y)) if x.path == y.path && x.wire == y.wire && x.body == y.body => {}
            (x, y) => {
                lines.push(format!("  first divergence at item {n}:"));
                lines.push(match x {
                    Some(x) => format!(
                        "    manifest: tag path {} wt{} @0x{:x}  {}",
                        x.path,
                        x.wire,
                        x.off,
                        show(&x.body)
                    ),
                    None => "    manifest: <end of buffer>".into(),
                });
                lines.push(match y {
                    Some(y) => format!(
                        "    prost   : tag path {} wt{} @0x{:x}  {}",
                        y.path,
                        y.wire,
                        y.off,
                        show(&y.body)
                    ),
                    None => "    prost   : <end of buffer>".into(),
                });
                return lines.join("\n");
            }
        }
    }
    lines.push("  no field-level divergence; the buffers differ only in framing".into());
    lines.join("\n")
}

/// Every (tag path, wire type, body) triple in the buffer, as comparable strings. Used to
/// show that two encodings of the same value are permutations of each other.
pub fn flatten_sorted(b: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    flatten(b, "", 0, &mut out);
    out.into_iter()
        .map(|i| format!("{}/{}/{}", i.path, i.wire, i.body.iter().map(|c| format!("{c:02x}")).collect::<String>()))
        .collect()
}
