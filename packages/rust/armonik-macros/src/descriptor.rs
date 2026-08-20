//! Loading and indexing of the protobuf descriptor set compiled by the `armonik` build script into
//! `$OUT_DIR/descriptor.bin`.
//!
//! The index is cached per (mtime, len) of the descriptor file so that long-lived proc-macro hosts
//! (rust-analyzer in particular) pick up descriptor changes without a restart, while plain builds
//! decode the file only once.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use prost::Message;
use prost_types::field_descriptor_proto::{Label, Type};
use prost_types::{
    DescriptorProto, EnumDescriptorProto, FieldDescriptorProto, FileDescriptorProto,
    FileDescriptorSet,
};

/// Scalar/wire kind of a protobuf field, mirrored from the descriptor. The `armonik` codec keeps an
/// equivalent runtime classification that the emitted shape asserts are checked against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FieldKind {
    Double,
    Float,
    Int32,
    Int64,
    UInt32,
    UInt64,
    Bool,
    String,
    Bytes,
    /// Full name of the message type, without leading dot.
    Message(String),
    /// Full name of the enum type, without leading dot.
    Enum(String),
    /// A wire kind the codec does not implement (`sint*`, `fixed*`, `sfixed*`; no ArmoniK field
    /// uses them). Resolving a field of this kind is a spanned compile error naming it.
    Unsupported(&'static str),
}

/// Cardinality of a protobuf field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Cardinality {
    /// Singular proto3 field: implicit presence.
    Singular,
    /// `optional` proto3 field: explicit presence.
    Optional,
    /// Repeated field (packedness is decided by the Rust element type's `ProtoField` impl, not
    /// restated from the descriptor).
    Repeated,
    /// Map field, folded from its synthetic `*Entry` message.
    Map { key: FieldKind, value: FieldKind },
}

/// A field of a protobuf message, as seen by the derives.
pub(crate) struct FieldMeta {
    pub(crate) name: String,
    pub(crate) tag: u32,
    pub(crate) kind: FieldKind,
    pub(crate) cardinality: Cardinality,
    /// Leading comment from the proto, cleaned up line by line.
    pub(crate) docs: Vec<String>,
    /// Index into [`MessageMeta::oneofs`] when the field is a member of a real oneof (synthetic
    /// proto3-optional oneofs are folded into [`Cardinality::Optional`] instead).
    pub(crate) oneof: Option<usize>,
}

/// A oneof declaration and the fields belonging to it.
pub(crate) struct OneofMeta {
    pub(crate) name: String,
    /// Indices into [`MessageMeta::fields`].
    pub(crate) fields: Vec<usize>,
}

/// A protobuf message, flattened out of its file/nesting structure.
pub(crate) struct MessageMeta {
    pub(crate) fields: Vec<FieldMeta>,
    pub(crate) oneofs: Vec<OneofMeta>,
    /// Leading comment from the proto, cleaned up line by line.
    pub(crate) docs: Vec<String>,
}

impl MessageMeta {
    pub(crate) fn oneof(&self, name: &str) -> Option<(usize, &OneofMeta)> {
        self.oneofs
            .iter()
            .enumerate()
            .find(|(_, oneof)| oneof.name == name)
    }
}

/// A protobuf enum.
pub(crate) struct EnumMeta {
    /// Pairs of (full proto value name, numeric value), in declaration order.
    pub(crate) values: Vec<(String, i32)>,
    /// Leading comment from the proto, cleaned up line by line.
    pub(crate) docs: Vec<String>,
    /// Per-value leading comments, parallel to `values`.
    pub(crate) value_docs: Vec<Vec<String>>,
}

/// An RPC of a protobuf service, as seen by `service!`.
pub(crate) struct MethodMeta {
    pub(crate) name: String,
    /// Full name of the input message, without leading dot.
    pub(crate) input: String,
    /// Full name of the output message, without leading dot.
    pub(crate) output: String,
    pub(crate) client_streaming: bool,
    pub(crate) server_streaming: bool,
    /// Leading comment from the proto, cleaned up line by line.
    pub(crate) docs: Vec<String>,
}

/// A protobuf service.
pub(crate) struct ServiceMeta {
    pub(crate) methods: Vec<MethodMeta>,
    /// Leading comment from the proto, cleaned up line by line.
    pub(crate) docs: Vec<String>,
}

/// Index of every message, enum and service in the descriptor set, keyed by full name without
/// leading dot (e.g. `armonik.api.grpc.v1.TaskOptions`).
pub(crate) struct DescriptorIndex {
    pub(crate) fingerprint: u64,
    pub(crate) messages: HashMap<String, MessageMeta>,
    pub(crate) enums: HashMap<String, EnumMeta>,
    pub(crate) services: HashMap<String, ServiceMeta>,
}

impl DescriptorIndex {
    /// Leading comment of a message, for the type that stands for it. Empty for a type that names
    /// no message and for a name the descriptor does not have: a plan is built either way, and the
    /// missing name is already an error of its own.
    pub(crate) fn message_docs(&self, name: Option<&str>) -> Vec<String> {
        name.and_then(|name| self.messages.get(name))
            .map(|meta| meta.docs.clone())
            .unwrap_or_default()
    }
}

struct Cached {
    mtime: Option<SystemTime>,
    len: u64,
    index: Arc<DescriptorIndex>,
}

static CACHE: Mutex<Option<Cached>> = Mutex::new(None);

/// Load (or reuse) the descriptor index.
///
/// Errors are strings with remediation guidance; the derive entry points attach them to the span of
/// the derived type.
pub(crate) fn index() -> Result<Arc<DescriptorIndex>, String> {
    let out_dir = std::env::var_os("OUT_DIR").ok_or_else(|| {
        "OUT_DIR is not set: the armonik derives can only be used from within \
         the armonik crate, whose build script produces the protobuf \
         descriptor set they validate against"
            .to_owned()
    })?;
    let path = PathBuf::from(out_dir).join("descriptor.bin");

    let metadata = std::fs::metadata(&path).map_err(|err| {
        format!(
            "`{}` not found ({err}): the armonik build script has not run; \
             build the crate with cargo, and if using rust-analyzer, enable \
             build scripts",
            path.display()
        )
    })?;
    let mtime = metadata.modified().ok();
    let len = metadata.len();

    let mut cache = CACHE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(cached) = cache.as_ref() {
        if cached.mtime == mtime && cached.len == len {
            return Ok(Arc::clone(&cached.index));
        }
    }

    let bytes = std::fs::read(&path)
        .map_err(|err| format!("failed to read `{}`: {err}", path.display()))?;
    let fds = FileDescriptorSet::decode(bytes.as_slice())
        .map_err(|err| format!("failed to decode `{}`: {err}", path.display()))?;
    let fingerprint = {
        use std::hash::Hasher as _;
        let mut hasher = fnv::FnvHasher::default();
        hasher.write(&bytes);
        hasher.finish()
    };
    let index = Arc::new(build_index(fingerprint, &fds)?);

    *cache = Some(Cached {
        mtime,
        len,
        index: Arc::clone(&index),
    });
    Ok(index)
}

fn build_index(fingerprint: u64, fds: &FileDescriptorSet) -> Result<DescriptorIndex, String> {
    // First pass: collect the synthetic map-entry messages so that map fields can be folded into
    // `Cardinality::Map` in the second pass.
    let mut map_entries = HashMap::new();
    for file in &fds.file {
        let prefix = file.package();
        for message in &file.message_type {
            collect_map_entries(prefix, message, &mut map_entries)?;
        }
    }

    let mut index = DescriptorIndex {
        fingerprint,
        messages: HashMap::new(),
        enums: HashMap::new(),
        services: HashMap::new(),
    };
    for file in &fds.file {
        let prefix = file.package();
        let comments = comments(file);
        for (idx, message) in file.message_type.iter().enumerate() {
            let path = vec![FILE_MESSAGE, idx as i32];
            add_message(prefix, message, &map_entries, &comments, path, &mut index)?;
        }
        for (idx, enumeration) in file.enum_type.iter().enumerate() {
            let path = vec![FILE_ENUM, idx as i32];
            add_enum(prefix, enumeration, &comments, path, &mut index);
        }
        add_services(file, &mut index);
    }
    Ok(index)
}

/// `SourceCodeInfo` path components: field numbers of the descriptor protos.
const FILE_MESSAGE: i32 = 4;
const FILE_ENUM: i32 = 5;
const MESSAGE_FIELD: i32 = 2;
const MESSAGE_NESTED: i32 = 3;
const MESSAGE_ENUM: i32 = 4;
const ENUM_VALUE: i32 = 2;

/// The cleaned comment of every location in the file, keyed by path.
///
/// The protos mix styles: a block comment above a message, a `//` or `/** */` comment on the same
/// line as a field or an enum value. `protox` reads the first two the way protoc does, but a
/// same-line `/** */` it records as the *leading* comment of the **next** element rather than as
/// the trailing comment of the one it follows. Taken at face value that shifts a whole enum's
/// prose by one value: on this schema every `TaskStatus` variant documented itself as its
/// predecessor, and the first value of each run lost its comment entirely.
///
/// So a leading comment is re-attributed to the previous sibling whenever there was no line for it
/// to sit on: if the previous element ends on the line before this one starts, nothing can be
/// written above this one, and what protox recorded there was written after that one.
fn comments(file: &FileDescriptorProto) -> HashMap<Vec<i32>, Vec<String>> {
    use prost_types::source_code_info::Location;

    let Some(info) = file.source_code_info.as_ref() else {
        return HashMap::new();
    };

    // `span` is [line, col, end_col] on one line, [line, col, end_line, end_col] across several.
    let start_line = |location: &Location| location.span.first().copied().unwrap_or(0);
    let end_line = |location: &Location| match location.span.len() {
        4 => location.span[2],
        _ => start_line(location),
    };
    let start_col = |location: &Location| location.span.get(1).copied().unwrap_or(0);

    let by_path: HashMap<&[i32], &Location> = info
        .location
        .iter()
        .map(|location| (location.path.as_slice(), location))
        .collect();

    let mut docs: HashMap<Vec<i32>, Vec<String>> = HashMap::new();

    // Trailing comments are recorded against the element they follow, which protox gets right.
    for location in &info.location {
        if let Some(trailing) = location.trailing_comments.as_deref() {
            docs.insert(location.path.clone(), clean_comment(trailing));
        }
    }

    // Leading comments, moved onto the previous sibling where they cannot have been written above
    // this element. `or_insert_with` so a same-line comment never displaces an element's own.
    for location in &info.location {
        let Some(leading) = location.leading_comments.as_deref() else {
            continue;
        };
        let owner = previous_sibling(&location.path)
            .filter(|previous| {
                by_path.get(previous.as_slice()).is_some_and(|previous| {
                    // Two conditions, both of which say there was nowhere to write this comment
                    // above the element it is recorded against: that element starts on the line
                    // right after the previous one, and nothing precedes it on its own line. A
                    // comment written there -- `/** doc */ string y = 2;` -- pushes the element's
                    // column past its sibling's, which is what tells that apart from the schema's
                    // style, `string x = 1; /** doc */` with `y` on the next line.
                    end_line(previous) + 1 == start_line(location)
                        && start_col(location) <= start_col(previous)
                })
            })
            .unwrap_or_else(|| location.path.clone());
        docs.entry(owner).or_insert_with(|| clean_comment(leading));
    }

    docs
}

/// The path of the element declared just before `path` among its siblings, if any.
///
/// Every path this is asked about ends in a repeated-field index (`[4, m, 2, f]` for a field,
/// `[5, e, 2, v]` for an enum value), so the sibling is that index minus one. A path ending in
/// anything else yields at worst an entry nothing looks up.
fn previous_sibling(path: &[i32]) -> Option<Vec<i32>> {
    match path.split_last() {
        Some((&index, head)) if index > 0 => {
            let mut previous = head.to_vec();
            previous.push(index - 1);
            Some(previous)
        }
        _ => None,
    }
}

/// Field numbers of `FileDescriptorProto.service` and `ServiceDescriptorProto.method`, the
/// `SourceCodeInfo` path components for service and method comments.
const FILE_SERVICE: i32 = 6;
const SERVICE_METHOD: i32 = 2;

fn add_services(file: &FileDescriptorProto, index: &mut DescriptorIndex) {
    // Through the same `comments` the messages and enums go through, rather than reading
    // `leading_comments` directly: that read is what shifts a same-line block comment onto the next
    // element, and it drops trailing comments outright. No rpc line carries one today, so this was
    // latent, but it would have gone wrong the first time one did.
    let comments = comments(file);
    let comment = |path: &[i32]| -> Vec<String> { comments.get(path).cloned().unwrap_or_default() };

    let prefix = file.package();
    for (service_idx, service) in file.service.iter().enumerate() {
        let methods = service
            .method
            .iter()
            .enumerate()
            .map(|(method_idx, method)| MethodMeta {
                name: method.name().to_owned(),
                input: method.input_type().trim_start_matches('.').to_owned(),
                output: method.output_type().trim_start_matches('.').to_owned(),
                client_streaming: method.client_streaming(),
                server_streaming: method.server_streaming(),
                docs: comment(&[
                    FILE_SERVICE,
                    service_idx as i32,
                    SERVICE_METHOD,
                    method_idx as i32,
                ]),
            })
            .collect();
        index.services.insert(
            full_name(prefix, service.name()),
            ServiceMeta {
                methods,
                docs: comment(&[FILE_SERVICE, service_idx as i32]),
            },
        );
    }
}

/// Clean a proto leading comment into rustdoc lines: drop the javadoc-style `*` filler lines and
/// the common leading space, trim trailing whitespace.
fn clean_comment(comment: &str) -> Vec<String> {
    let mut lines: Vec<String> = comment
        .lines()
        .map(|line| {
            let line = line.strip_prefix(' ').unwrap_or(line).trim_end();
            // The protos are commented `/** ... */`, and protoc hands the continuation lines over
            // with their leading `*` intact. A bare one is a blank line; one with content is the
            // margin, and leaving it in makes rustdoc read the line as a markdown bullet, which
            // rendered 381 of 3,366 docblocks as one-item lists.
            let line = if line == "*" {
                ""
            } else {
                line.strip_prefix("* ").unwrap_or(line)
            };
            escape_prose(line)
        })
        .collect();
    while lines.first().is_some_and(|line| line.is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    lines
}

/// The most leading indentation CommonMark allows before it opens an indented code block.
const MAX_INDENT: usize = 3;

/// Render one harvested line as inert prose.
///
/// Proto comments are written for javadoc-style tooling, and two of their habits reach further than
/// they look. An indented line becomes a **doctest**: four leading spaces open a CommonMark indented
/// code block, rustdoc reads an unannotated one as Rust, and `cargo test` compiles it, so a comment
/// like `*     GET /tasks?id=<id> HTTP/1.1` fails the Rust build from another package's pull
/// request. And `<...>` in prose is swallowed by the browser while `<div>` is injected raw into the
/// rendered page.
///
/// So indentation is clamped rather than dropped, keeping relative indentation visible without
/// opening a block; a leading fence is escaped; and `[`, `<` and `\` are escaped, `[` because a
/// link to a missing anchor is a rustdoc warning and the build runs with `-Dwarnings`.
///
/// Deliberately *not* `*`, `_`, `#` or `-`: none of them can break a build, and escaping them would
/// change how 3,366 existing docblocks render, which is a diff nobody can review.
///
/// Code spans are copied through untouched. Markdown does not read these characters inside one, so
/// escaping there would show the backslash.
fn escape_prose(line: &str) -> String {
    let body = line.trim_start();
    // Measured the way CommonMark measures it, so a single leading tab counts as the code block it
    // would open rather than as the one column it occupies.
    let indent = line[..line.len() - body.len()]
        .chars()
        .fold(0, |width, ch| match ch {
            '\t' => width + 4 - width % 4,
            _ => width + 1,
        });
    let mut out = " ".repeat(indent.min(MAX_INDENT));
    if body.starts_with("```") || body.starts_with("~~~") {
        out.push('\\');
    }

    let bytes = body.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'`' {
            let run = backtick_run(bytes, at);
            // A run with no matching close opens no code span, so it is literal text; either way
            // the run itself needs no escape.
            let end = closing_run(bytes, at + run, run).map_or(at + run, |close| close + run);
            out.push_str(&body[at..end]);
            at = end;
            continue;
        }
        if matches!(bytes[at], b'[' | b'<' | b'\\') {
            out.push('\\');
        }
        let width = body[at..].chars().next().map_or(1, char::len_utf8);
        out.push_str(&body[at..at + width]);
        at += width;
    }
    out
}

/// Length of the run of backticks starting at `at`.
fn backtick_run(bytes: &[u8], at: usize) -> usize {
    bytes[at..].iter().take_while(|byte| **byte == b'`').count()
}

/// Where the run of exactly `len` backticks that closes this code span starts, if there is one.
fn closing_run(bytes: &[u8], from: usize, len: usize) -> Option<usize> {
    let mut at = from;
    while at < bytes.len() {
        if bytes[at] != b'`' {
            at += 1;
            continue;
        }
        let run = backtick_run(bytes, at);
        if run == len {
            return Some(at);
        }
        at += run;
    }
    None
}

fn full_name(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_owned()
    } else {
        format!("{prefix}.{name}")
    }
}

fn collect_map_entries(
    prefix: &str,
    message: &DescriptorProto,
    map_entries: &mut HashMap<String, (FieldKind, FieldKind)>,
) -> Result<(), String> {
    let full = full_name(prefix, message.name());
    if message.options.as_ref().is_some_and(|o| o.map_entry()) {
        let key = message
            .field
            .iter()
            .find(|f| f.number() == 1)
            .ok_or_else(|| format!("map entry `{full}` has no key field"))?;
        let value = message
            .field
            .iter()
            .find(|f| f.number() == 2)
            .ok_or_else(|| format!("map entry `{full}` has no value field"))?;
        map_entries.insert(full, (field_kind(key)?, field_kind(value)?));
        return Ok(());
    }
    for nested in &message.nested_type {
        collect_map_entries(&full, nested, map_entries)?;
    }
    Ok(())
}

fn add_message(
    prefix: &str,
    message: &DescriptorProto,
    map_entries: &HashMap<String, (FieldKind, FieldKind)>,
    comments: &HashMap<Vec<i32>, Vec<String>>,
    path: Vec<i32>,
    index: &mut DescriptorIndex,
) -> Result<(), String> {
    let full = full_name(prefix, message.name());
    if message.options.as_ref().is_some_and(|o| o.map_entry()) {
        // Synthetic map entries are folded into their parent's map fields.
        return Ok(());
    }

    for (idx, nested) in message.nested_type.iter().enumerate() {
        let mut nested_path = path.clone();
        nested_path.extend([MESSAGE_NESTED, idx as i32]);
        add_message(&full, nested, map_entries, comments, nested_path, index)?;
    }
    for (idx, enumeration) in message.enum_type.iter().enumerate() {
        let mut nested_path = path.clone();
        nested_path.extend([MESSAGE_ENUM, idx as i32]);
        add_enum(&full, enumeration, comments, nested_path, index);
    }

    let mut oneofs = message
        .oneof_decl
        .iter()
        .map(|oneof| OneofMeta {
            name: oneof.name().to_owned(),
            fields: Vec::new(),
        })
        .collect::<Vec<_>>();

    let mut fields = Vec::new();
    for (field_idx, field) in message.field.iter().enumerate() {
        let kind = field_kind(field)
            .map_err(|err| format!("in message `{full}`, field `{}`: {err}", field.name()))?;
        let mut field_path = path.clone();
        field_path.extend([MESSAGE_FIELD, field_idx as i32]);

        let (cardinality, oneof) = if field.proto3_optional() {
            // proto3 `optional` is encoded as a synthetic single-field oneof; fold it into an
            // explicit-presence cardinality instead.
            (Cardinality::Optional, None)
        } else if field.label() == Label::Repeated {
            if let FieldKind::Message(type_name) = &kind {
                if let Some((key, value)) = map_entries.get(type_name) {
                    (
                        Cardinality::Map {
                            key: key.clone(),
                            value: value.clone(),
                        },
                        None,
                    )
                } else {
                    (Cardinality::Repeated, None)
                }
            } else {
                (Cardinality::Repeated, None)
            }
        } else {
            let oneof = field.oneof_index.map(|idx| idx as usize);
            (Cardinality::Singular, oneof)
        };

        if let Some(oneof_index) = oneof {
            let oneof_meta = oneofs.get_mut(oneof_index).ok_or_else(|| {
                format!(
                    "in message `{full}`, field `{}`: oneof index {oneof_index} out of range",
                    field.name()
                )
            })?;
            oneof_meta.fields.push(fields.len());
        }

        fields.push(FieldMeta {
            name: field.name().to_owned(),
            tag: field.number() as u32,
            kind,
            cardinality,
            docs: comments.get(&field_path).cloned().unwrap_or_default(),
            oneof,
        });
    }

    index.messages.insert(
        full,
        MessageMeta {
            fields,
            oneofs,
            docs: comments.get(&path).cloned().unwrap_or_default(),
        },
    );
    Ok(())
}

fn add_enum(
    prefix: &str,
    enumeration: &EnumDescriptorProto,
    comments: &HashMap<Vec<i32>, Vec<String>>,
    path: Vec<i32>,
    index: &mut DescriptorIndex,
) {
    let full = full_name(prefix, enumeration.name());
    let values = enumeration
        .value
        .iter()
        .map(|value| (value.name().to_owned(), value.number()))
        .collect();
    let value_docs = enumeration
        .value
        .iter()
        .enumerate()
        .map(|(idx, _)| {
            let mut value_path = path.clone();
            value_path.extend([ENUM_VALUE, idx as i32]);
            comments.get(&value_path).cloned().unwrap_or_default()
        })
        .collect();
    index.enums.insert(
        full,
        EnumMeta {
            values,
            docs: comments.get(&path).cloned().unwrap_or_default(),
            value_docs,
        },
    );
}

fn field_kind(field: &FieldDescriptorProto) -> Result<FieldKind, String> {
    let type_name = || field.type_name().trim_start_matches('.').to_owned();
    Ok(match field.r#type() {
        Type::Double => FieldKind::Double,
        Type::Float => FieldKind::Float,
        Type::Int32 => FieldKind::Int32,
        Type::Int64 => FieldKind::Int64,
        Type::Uint32 => FieldKind::UInt32,
        Type::Uint64 => FieldKind::UInt64,
        Type::Sint32 => FieldKind::Unsupported("sint32"),
        Type::Sint64 => FieldKind::Unsupported("sint64"),
        Type::Fixed32 => FieldKind::Unsupported("fixed32"),
        Type::Fixed64 => FieldKind::Unsupported("fixed64"),
        Type::Sfixed32 => FieldKind::Unsupported("sfixed32"),
        Type::Sfixed64 => FieldKind::Unsupported("sfixed64"),
        Type::Bool => FieldKind::Bool,
        Type::String => FieldKind::String,
        Type::Bytes => FieldKind::Bytes,
        Type::Message => FieldKind::Message(type_name()),
        Type::Enum => FieldKind::Enum(type_name()),
        Type::Group => return Err("proto2 groups are not supported".to_owned()),
    })
}

#[cfg(test)]
mod tests {
    use super::escape_prose;

    #[test]
    fn indentation_is_clamped_below_a_code_block() {
        assert_eq!(escape_prose("  two"), "  two");
        assert_eq!(escape_prose("   three"), "   three");
        assert_eq!(escape_prose("    four"), "   four");
        assert_eq!(escape_prose("\tGET /tasks"), "   GET /tasks");
    }

    #[test]
    fn a_leading_fence_is_escaped() {
        assert_eq!(escape_prose("```sh"), "\\```sh");
        assert_eq!(escape_prose("~~~"), "\\~~~");
        assert_eq!(escape_prose("a ``` b"), "a ``` b");
    }

    #[test]
    fn the_three_build_breaking_characters_are_escaped() {
        assert_eq!(escape_prose("see [Something]"), "see \\[Something]");
        assert_eq!(escape_prose("a <string> value"), "a \\<string> value");
        assert_eq!(escape_prose("a \\ backslash"), "a \\\\ backslash");
    }

    /// The four that are left alone, because none can break a build and escaping them would
    /// re-render every existing docblock.
    #[test]
    fn markdown_emphasis_is_left_alone() {
        assert_eq!(escape_prose("*bold* _it_ # h - li"), "*bold* _it_ # h - li");
    }

    #[test]
    fn code_spans_are_copied_through() {
        assert_eq!(escape_prose("`Vec<T>` holds"), "`Vec<T>` holds");
        assert_eq!(escape_prose("``a ` b<c>``"), "``a ` b<c>``");
        // An unmatched run opens no span, so what follows is prose again.
        assert_eq!(escape_prose("`unclosed <T>"), "`unclosed \\<T>");
    }

    #[test]
    fn ordinary_prose_is_unchanged() {
        assert_eq!(
            escape_prose("The task creation date."),
            "The task creation date."
        );
        assert_eq!(escape_prose(""), "");
    }
}
