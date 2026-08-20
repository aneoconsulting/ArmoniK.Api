//! Semantic equality of dynamic messages.
//!
//! Byte equality is deliberately not required: the armonik encoder writes a
//! message field whatever it holds, so a nested message the reference omitted
//! comes back present and empty (indistinguishable from absent for the
//! "absent = default" fields), and map iteration order is unstable.
//! What must hold exactly:
//!
//! - which member of a real oneof is set (including empty message payloads);
//! - presence of proto3 `optional` fields;
//! - every value, with unset singular fields equal to their defaults and
//!   unset singular messages equal to empty ones.

use prost_reflect::{DynamicMessage, FieldDescriptor, Kind, ReflectMessage, Value};

pub fn messages(a: &DynamicMessage, b: &DynamicMessage) -> bool {
    a.descriptor()
        .fields()
        .all(|field| field_equal(a, b, &field))
}

fn field_equal(a: &DynamicMessage, b: &DynamicMessage, field: &FieldDescriptor) -> bool {
    let in_real_oneof = field
        .containing_oneof()
        .is_some_and(|oneof| !oneof.is_synthetic());
    let proto3_optional = field
        .containing_oneof()
        .is_some_and(|oneof| oneof.is_synthetic());

    if in_real_oneof || proto3_optional {
        if a.has_field(field) != b.has_field(field) {
            return false;
        }
        if !a.has_field(field) {
            return true;
        }
    }

    // For everything else, `get_field` folds absence into the default value,
    // which is exactly the equivalence the encoder is allowed to use.
    value_equal(&a.get_field(field), &b.get_field(field), field)
}

fn value_equal(a: &Value, b: &Value, field: &FieldDescriptor) -> bool {
    match (a, b) {
        (Value::Message(a), Value::Message(b)) => messages(a, b),
        (Value::List(a), Value::List(b)) => {
            a.len() == b.len()
                && a.iter()
                    .zip(b.iter())
                    .all(|(a, b)| element_equal(a, b, &field.kind()))
        }
        (Value::Map(a), Value::Map(b)) => {
            let Kind::Message(entry) = field.kind() else {
                return false;
            };
            let value_field = entry.map_entry_value_field();
            a.len() == b.len()
                && a.iter().all(|(key, a_value)| {
                    b.get(key)
                        .is_some_and(|b_value| element_equal(a_value, b_value, &value_field.kind()))
                })
        }
        (a, b) => a == b,
    }
}

fn element_equal(a: &Value, b: &Value, kind: &Kind) -> bool {
    match (a, b) {
        (Value::Message(a), Value::Message(b)) => {
            debug_assert!(matches!(kind, Kind::Message(_)));
            messages(a, b)
        }
        (a, b) => a == b,
    }
}
