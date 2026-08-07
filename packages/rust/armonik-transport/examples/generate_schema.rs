//! Prints the JSON schema of the flat option vocabulary to stdout.
//!
//! For a consumer that generates an options class in another language:
//!
//! ```sh
//! cargo run -p armonik-transport --features schema --example generate_schema
//! ```

use armonik_transport::reexports::schemars;

fn main() {
    let schema = schemars::schema_for!(armonik_transport::HttpConfig);
    println!(
        "{}",
        serde_json::to_string_pretty(&schema).expect("a schema serialises to JSON")
    );
}
