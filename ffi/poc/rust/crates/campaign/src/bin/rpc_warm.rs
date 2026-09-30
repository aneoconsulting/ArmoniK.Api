//! FIX-PLAN WP10: `serve.sh warm N`. N CHECKED calls per direction (a, b, c, d) from a tonic
//! client (cell A: prost over tonic) and from the core's client (cell B: prost over the core's
//! transport), on one socket and configuration. Exit 3 on the first failed check.
//!
//!   rpc_warm --socket PATH --transport shipped|pinned --n N
//!   rpc_warm --target URI --transport pinned --n N     (e.g. http://127.0.0.1:PORT, the TCP listener)
//!
//! Direction d (a 4 MiB client stream) gets ceil(N / 4) calls, as the grid's own warm-up.

use campaign::grid::{self, Conn};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let arg = |k: &str| a.iter().position(|x| x == k).map(|i| a[i + 1].clone());
    let pinned = arg("--transport").as_deref() == Some("pinned");
    let n: usize = arg("--n").map(|v| v.parse().unwrap()).unwrap_or(64);
    let target = arg("--target").unwrap_or_else(|| format!("unix:{}", arg("--socket").expect("--socket or --target")));
    assert!(harness::generated::binding::ak_init_once() >= 0);
    let p22 = campaign::server::p22_response().len() as u64;
    let run = || -> Result<(), String> {
        grid::warm_with(&["A", "B"], &target, pinned, n, p22)?;
        for cell in ["A", "B"] {
            let conn = Conn::open(cell, &target, pinned);
            let call = grid::call_of(cell, &conn, "b", grid::slots(1), p22);
            for _ in 0..n {
                call.once(0)?;
            }
        }
        grid::warm_with_c(&["A", "B"], &target, pinned, n, 0)?;
        grid::warm_with_d(&["A", "B"], &target, pinned, n.div_ceil(4), 0)
    };
    match run() {
        Ok(()) => println!("warm: {n} checked calls per direction (a, b, c; d: {}) from tonic (A) and the core (B) on {target}", n.div_ceil(4)),
        Err(e) => {
            eprintln!("ABORT (requirement 18): warm-up: {e}");
            std::process::exit(3);
        }
    }
}
