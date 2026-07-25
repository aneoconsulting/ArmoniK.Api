//! Mapping from proto full names to the armonik types implementing them.
//!
//! Grown in lockstep with the annotation of `src/objects/`: registering a
//! type here removes it from `TEMP_UNMAPPED` in `main.rs` (the coverage
//! test enforces both directions).

use armonik::reexports::prost::Message;
use prost_reflect::{DynamicMessage, ReflectMessage, Value};

pub struct Entry {
    pub proto: &'static str,
    /// Decode the bytes as the armonik type and re-encode them.
    pub roundtrip: fn(&[u8]) -> Result<Vec<u8>, armonik::reexports::prost::DecodeError>,
    /// Projection onto the type's documented equivalence classes, applied to
    /// both sides before the semantic comparison. Needed only where an
    /// armonik type has a custom default that differs from the proto zero
    /// value (see `normalize_task_options`).
    pub normalize: Option<fn(&mut DynamicMessage)>,
}

macro_rules! registry {
    ($($proto:literal => $ty:ty $(, normalize = $normalize:expr)?),* $(,)?) => {
        pub fn entries() -> Vec<Entry> {
            vec![$(Entry {
                proto: $proto,
                roundtrip: |bytes| Ok(<$ty as Message>::decode(bytes)?.encode_to_vec()),
                #[allow(unused_mut, unused_assignments)]
                normalize: {
                    let mut normalize: Option<fn(&mut DynamicMessage)> = None;
                    $(normalize = Some($normalize);)?
                    normalize
                },
            }),*]
        }
    };
}

registry! {
    "armonik.api.grpc.v1.Session" => armonik::Session,
    "armonik.api.grpc.v1.TaskOptions" => armonik::TaskOptions,
        normalize = normalize_task_options,
}

/// `TaskOptions::default()` uses `INFINITE_DURATION` for `max_duration`, and
/// the wire mapping folds "absent" and "present but empty" (both encode zero
/// bytes) into that default. Project both representations onto the explicit
/// infinite duration so the comparison reflects the documented semantics.
fn normalize_task_options(message: &mut DynamicMessage) {
    let field = message
        .descriptor()
        .get_field_by_name("max_duration")
        .expect("TaskOptions.max_duration exists");
    let is_empty = match message.get_field(&field).as_ref() {
        Value::Message(duration) => duration.encode_to_vec().is_empty(),
        _ => true,
    };
    if is_empty {
        let duration_desc = match field.kind() {
            prost_reflect::Kind::Message(desc) => desc,
            other => panic!("max_duration should be a message, got {other:?}"),
        };
        let mut infinite = DynamicMessage::new(duration_desc);
        infinite.set_field_by_name("seconds", Value::I64(315576000000));
        message.set_field(&field, Value::Message(infinite));
    }
}
