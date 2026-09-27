//! U2-stream's byte check (correctness before timing; gate step 11e): every cell of this
//! build, reference and framed, uploads 4 MiB and 16 MiB as a client stream to the
//! server's STREAM_CHECK path, which answers the data byte count and the SHA-256 of every
//! message's bytes as it received them; both must equal the uploaded ones (the messages'
//! wire bytes, identical from every encoder, checked against prost and core-native when the
//! payload is built). Also direction c's upload (U1-unary) through every cell, with the
//! server's status. An in-process server on a Unix socket. Exit 1 on any mismatch.
use campaign::grid::{self, Conn};

fn main() {
    assert!(harness::generated::binding::ak_init_once() >= 0);
    let dir = std::env::temp_dir().join(format!("ak-upload-check-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let sock = dir.join("grid.sock");
    let _server = campaign::server::spawn_in_process(sock.clone(), false);
    let target = format!("unix:{}", sock.display());
    let mut bad = 0;
    for cell in grid::CELLS {
        let conn = Conn::open(cell, &target, false);
        for &(label, chunks) in grid::D_PAYLOADS {
            let call = grid::call_of_d(cell, &conn, chunks, grid::slots(1), (chunks * grid::CHUNK) as u64, true);
            match call.once(0) {
                Ok(()) => println!("{cell:<10} d/{label:<6} {} data bytes and the SHA-256 of every message as received: identical", chunks * grid::CHUNK),
                Err(e) => { bad += 1; println!("{cell:<10} d/{label:<6} MISMATCH: {e}"); }
            }
        }
        for pid in grid::C_PAYLOADS {
            let call = grid::call_of_c(cell, &conn, pid, grid::slots(1), 0);
            match call.once(0) {
                Ok(()) => println!("{cell:<10} c/{pid:<6} upload accepted (prost-decoded by the server, non-empty), empty response"),
                Err(e) => { bad += 1; println!("{cell:<10} c/{pid:<6} FAILED: {e}"); }
            }
        }
    }
    // The control: a planted wrong SHA-256 (one bit) and a planted wrong count must each
    // fail, on a reference cell and a framed one of each transport.
    let mut wrong = grid::stream_payload(2).sha256;
    wrong[0] ^= 1;
    let wrong: &'static [u8; 32] = Box::leak(Box::new(wrong));
    for cell in ["B", "Bf", grid::cell_of("D"), grid::cell_of("Df")] {
        let conn = Conn::open(cell, &target, false);
        let sha_ctl = grid::call_of_d_with(cell, &conn, 2, grid::slots(1), (2 * grid::CHUNK) as u64, Some(wrong)).once(0);
        let len_ctl = grid::call_of_d(cell, &conn, 2, grid::slots(1), (2 * grid::CHUNK) as u64 + 1, true).once(0);
        let ok = sha_ctl.is_err() && len_ctl.is_err();
        if !ok { bad += 1; }
        println!("control {cell:<10} planted wrong SHA-256: {}; planted wrong count: {}  {}",
                 sha_ctl.err().unwrap_or_else(|| "NOT DETECTED".into()), len_ctl.err().unwrap_or_else(|| "NOT DETECTED".into()),
                 if ok { "(both detected)" } else { "CONTROL FAILED" });
    }
    let _ = std::fs::remove_dir_all(&dir);
    println!("{}", if bad == 0 { "UPLOAD CHECK PASSED" } else { "UPLOAD CHECK FAILED" });
    std::process::exit(if bad == 0 { 0 } else { 1 });
}
