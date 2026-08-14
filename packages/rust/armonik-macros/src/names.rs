//! The naming rules the expansions match Rust identifiers to proto names by.
//!
//! One home, because the two callers disagreeing is invisible: a value prefix stripped by the Rust
//! type's name instead of the proto enum's still resolves, and silently harvests no comments.
//! `health_checks::Status` (proto `HealthStatusEnum`) is a live case of the two differing.

/// `HealthChecks` -> `health_checks`.
pub(crate) fn snake(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() {
            if i != 0 {
                out.push('_');
            }
            out.extend(ch.to_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

/// `TASK_STATUS_CREATING` -> `TaskStatusCreating`.
pub(crate) fn upper_camel(screaming_snake: &str) -> String {
    screaming_snake
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            let first = chars.next().map(|c| c.to_ascii_uppercase());
            first
                .into_iter()
                .chain(chars.map(|c| c.to_ascii_lowercase()))
                .collect::<String>()
        })
        .collect()
}

/// The prost-style Rust variant name for a proto enum value: upper-camel the value name, then strip
/// the enum's own name where it prefixes it, so `TASK_STATUS_CREATING` of `TaskStatus` is
/// `Creating`.
///
/// `enum_simple_name` is the *proto* enum's simple name, never the Rust type's: they are free to
/// differ, and the proto side is what the value names are actually prefixed with.
///
/// The prefix is kept when stripping it would leave nothing, or would leave a name starting with a
/// digit, neither of which is an identifier.
pub(crate) fn variant_name(enum_simple_name: &str, value_name: &str) -> String {
    let camel = upper_camel(value_name);
    match camel.strip_prefix(enum_simple_name) {
        Some(stripped)
            if !stripped.is_empty() && !stripped.starts_with(|c: char| c.is_ascii_digit()) =>
        {
            stripped.to_owned()
        }
        _ => camel,
    }
}

/// `TaskDetailed` -> `task_detailed`: the proto field name a Rust field or variant matches by
/// default.
///
/// Distinct from [`snake`] only in that it is ASCII-only, which is what a proto identifier is.
pub(crate) fn snake_case(camel: &str) -> String {
    let mut out = String::with_capacity(camel.len() + 4);
    for (i, c) in camel.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i != 0 {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The prefix on the values is the *proto enum's* name, which need not be the Rust type's.
    #[test]
    fn values_are_stripped_of_the_proto_enum_name() {
        assert_eq!(
            variant_name("TaskStatus", "TASK_STATUS_CREATING"),
            "Creating"
        );
        // `health_checks::Status`, whose proto enum is `HealthStatusEnum`.
        assert_eq!(
            variant_name("HealthStatusEnum", "HEALTH_STATUS_ENUM_HEALTHY"),
            "Healthy"
        );
        // Stripping the Rust name instead would have left the value unmatched.
        assert_ne!(
            variant_name("Status", "HEALTH_STATUS_ENUM_HEALTHY"),
            "Healthy"
        );
    }

    #[test]
    fn a_prefix_is_kept_when_stripping_it_leaves_no_identifier() {
        assert_eq!(variant_name("Colour", "COLOUR"), "Colour");
        assert_eq!(variant_name("Version", "VERSION_2"), "Version2");
    }
}
