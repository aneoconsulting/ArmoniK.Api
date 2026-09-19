//! Print the size and field offsets of every ABI struct the C# binding declares.
//!
//! ABI v1 obligation 12.3. See Cargo.toml for why this slice needs it and the
//! earlier ones did not: C# re-declares these types by hand, so the managed
//! `[StructLayout]` is a second declaration and only agreement makes the
//! by-value group work.
//!
//! Output is JSON on stdout, consumed by `gen/cs_abi.py`. Nothing here is
//! measured; it is a build step.

use ak_abi::generated::abi::*;
use ak_abi::{ak_span, ak_str};
use std::mem::{align_of, size_of};

macro_rules! lay {
    ($out:expr, $t:ty, [$($f:ident),* $(,)?]) => {{
        let name = stringify!($t);
        let v = <$t>::ZERO;
        let base = &v as *const $t as usize;
        let mut fields: Vec<String> = Vec::new();
        $(
            let off = (&v.$f as *const _ as usize) - base;
            fields.push(format!("\"{}\": {}", stringify!($f), off));
        )*
        $out.push(format!(
            "    \"{}\": {{ \"size\": {}, \"align\": {}, \"fields\": {{ {} }} }}",
            name, size_of::<$t>(), align_of::<$t>(), fields.join(", ")
        ));
    }};
}

/// A struct with no ZERO const, laid out by a zeroed instance instead.
macro_rules! lay_raw {
    ($out:expr, $t:ty, $v:expr, [$($f:ident),* $(,)?]) => {{
        let v: $t = $v;
        let base = &v as *const $t as usize;
        let mut fields: Vec<String> = Vec::new();
        $(
            let off = (&v.$f as *const _ as usize) - base;
            fields.push(format!("\"{}\": {}", stringify!($f), off));
        )*
        $out.push(format!(
            "    \"{}\": {{ \"size\": {}, \"align\": {}, \"fields\": {{ {} }} }}",
            stringify!($t), size_of::<$t>(), align_of::<$t>(), fields.join(", ")
        ));
    }};
}

fn main() {
    let mut out: Vec<String> = Vec::new();

    lay_raw!(out, ak_str, ak_str::default(), [data, len, tc]);
    lay_raw!(out, ak_span, ak_span::default(), [off, len, coder]);

    // M1's chain: the root group, the element group, and the leaf it embeds.
    lay!(out, ak_efix_Timestamp, [seconds, nanos, presence]);
    lay!(out, ak_efix_ResultRaw, [
        session_id, name, owner_task_id, status, created_at, completed_at,
        result_id, size, created_by, opaque_id, manual_deletion, presence,
    ]);
    lay!(out, ak_efix_ListResultsResponse, [page, total, presence]);

    // The decode side. `ak_dfix_*` carries `ak_span`, an OFFSET into the buffer
    // the host handed in, which is ABI v1 decision 13's borrowed view already
    // present in the interface: eight bytes where a pointer pair was 24, and
    // still meaningful after a host has released a pinned region.
    lay!(out, ak_dfix_Timestamp, [seconds, nanos, presence]);
    lay!(out, ak_dfix_ResultRaw, [
        session_id, name, owner_task_id, status, created_at, completed_at,
        result_id, size, created_by, opaque_id, manual_deletion, presence,
    ]);
    lay!(out, ak_dfix_ListResultsResponse, [page, total, presence]);

    println!("{{");
    println!("  \"pointer_width\": {},", size_of::<usize>() * 8);
    println!("  \"structs\": {{");
    println!("{}", out.join(",\n"));
    println!("  }}");
    println!("}}");
}
