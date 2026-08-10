//! Discovery of the proto-to-type mapping, and the projection of messages onto the armonik types'
//! documented equivalence classes.
//!
//! The mapping is self-registering: every derived (and hand-written) message type pushes a
//! [`Registration`] into [`registrations::REGISTRY`], so new messages are covered without touching
//! the harness. Each registration carries the type's own
//! `Normalize` projection, generated from the same constructs that shape its codec (adapters,
//! markers, wrapper chains) or hand-written next to the hand-written impls. The generic
//! instantiations (`Sort`, `FilterStatus`) self-register the same way through
//! `#[armonik_macros::alias(...)]` at their alias sites.

use std::collections::HashMap;
use std::sync::OnceLock;

use prost_reflect::{DynamicMessage, ReflectMessage, Value};

pub(crate) use super::registrations::Hooks;

/// Every proto name a Rust type implements, with that type's hooks.
pub fn entries() -> impl Iterator<Item = (&'static str, Hooks)> {
    super::registrations::typed()
}

/// Project a message (recursively) onto the equivalence classes of its armonik type, so that the
/// semantic comparison reflects the documented semantics. Applied to both sides of every
/// round-trip.
pub fn normalize(message: &mut DynamicMessage) {
    let descriptor = message.descriptor();
    for field in descriptor.fields() {
        if !message.has_field(&field) {
            continue;
        }
        let mut value = message.get_field(&field).into_owned();
        if normalize_value(&mut value) {
            message.set_field(&field, value);
        }
    }
    apply_rules(message);
}

fn normalize_value(value: &mut Value) -> bool {
    match value {
        Value::Message(inner) => {
            normalize(inner);
            true
        }
        Value::List(items) => {
            let mut changed = false;
            for item in items {
                changed |= normalize_value(item);
            }
            changed
        }
        Value::Map(map) => {
            let mut changed = false;
            for item in map.values_mut() {
                changed |= normalize_value(item);
            }
            changed
        }
        _ => false,
    }
}

/// Canonical "absent" form per proto name: what each type emits for `Default::default()`, decoded
/// back dynamically. This is the harness's only own contribution to the quotient, derived from the
/// implementation itself instead of restated: whatever a type emits for "nothing" is what an absent
/// field or member is equivalent to.
fn canonicals() -> &'static HashMap<&'static str, (DynamicMessage, Hooks)> {
    static CANONICALS: OnceLock<HashMap<&'static str, (DynamicMessage, Hooks)>> = OnceLock::new();
    CANONICALS.get_or_init(|| {
        let pool = super::harness::pool();
        let mut map = HashMap::new();
        for (proto, hooks) in entries() {
            let Some(desc) = pool.get_message_by_name(proto) else {
                continue;
            };
            let canonical = DynamicMessage::decode(desc, (hooks.default_encoding)().as_slice())
                .expect("the default encoding decodes");
            map.entry(proto).or_insert((canonical, hooks));
        }
        map
    })
}

fn apply_rules(message: &mut DynamicMessage) {
    let name = message.descriptor().full_name().to_owned();
    let Some((canonical, hooks)) = canonicals().get(name.as_str()) else {
        return;
    };
    // The value projections declared by the type itself.
    (hooks.normalize)(message);
    // The canonical-absence fold: whatever the type emits for "nothing" (default oneof members, and
    // the zero fields the encoder writes rather than skips) materializes where it is absent, except
    // into a oneof that already carries another member.
    for member in canonical.descriptor().fields() {
        if !canonical.has_field(&member) || message.has_field(&member) {
            continue;
        }
        if let Some(oneof) = member.containing_oneof() {
            if oneof.fields().any(|other| message.has_field(&other)) {
                continue;
            }
        }
        message.set_field(&member, canonical.get_field(&member).into_owned());
    }
}
