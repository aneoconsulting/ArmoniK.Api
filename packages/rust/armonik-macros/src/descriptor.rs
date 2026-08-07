//! Loading and indexing of the protobuf descriptor set compiled by the
//! `armonik` build script into `$OUT_DIR/descriptor.bin`.
//!
//! The index is cached per (mtime, len) of the descriptor file so that
//! long-lived proc-macro hosts (rust-analyzer in particular) pick up
//! descriptor changes without a restart, while plain builds decode the file
//! only once.

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

/// Scalar/wire kind of a protobuf field, mirrored from the descriptor. The
/// `armonik` codec keeps an equivalent runtime classification that the
/// emitted shape asserts are checked against.
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
    /// A wire kind the codec does not implement (`sint*`/`fixed*`/`sfixed*`
    /// — no ArmoniK field uses them); resolving a field of this kind is a
    /// spanned compile error naming it.
    Unsupported(&'static str),
}

/// Cardinality of a protobuf field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Cardinality {
    /// Singular proto3 field: implicit presence.
    Singular,
    /// `optional` proto3 field: explicit presence.
    Optional,
    /// Repeated field (packedness is decided by the Rust element type's
    /// `ProtoField` impl, not restated from the descriptor).
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
    /// Index into [`MessageMeta::oneofs`] when the field is a member of a
    /// real oneof (synthetic proto3-optional oneofs are folded into
    /// [`Cardinality::Optional`] instead).
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

/// Index of every message, enum and service in the descriptor set, keyed by
/// full name without leading dot (e.g. `armonik.api.grpc.v1.TaskOptions`).
pub(crate) struct DescriptorIndex {
    pub(crate) fingerprint: u64,
    pub(crate) messages: HashMap<String, MessageMeta>,
    pub(crate) enums: HashMap<String, EnumMeta>,
    pub(crate) services: HashMap<String, ServiceMeta>,
}

struct Cached {
    mtime: Option<SystemTime>,
    len: u64,
    index: Arc<DescriptorIndex>,
}

static CACHE: Mutex<Option<Cached>> = Mutex::new(None);

/// Load (or reuse) the descriptor index.
///
/// Errors are strings with remediation guidance; the derive entry points
/// attach them to the span of the derived type.
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
    // First pass: collect the synthetic map-entry messages so that map fields
    // can be folded into `Cardinality::Map` in the second pass.
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

/// The cleaned comment of every location in the file, keyed by path. The
/// protos mix styles: block comments above messages, trailing comments on
/// fields — take the leading one, else the trailing one.
fn comments(file: &FileDescriptorProto) -> HashMap<Vec<i32>, Vec<String>> {
    file.source_code_info
        .as_ref()
        .map(|info| {
            info.location
                .iter()
                .filter_map(|location| {
                    let comment = location
                        .leading_comments
                        .as_deref()
                        .or(location.trailing_comments.as_deref())?;
                    Some((location.path.clone(), clean_comment(comment)))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Field numbers of `FileDescriptorProto.service` and
/// `ServiceDescriptorProto.method`, the `SourceCodeInfo` path components for
/// service and method comments.
const FILE_SERVICE: i32 = 6;
const SERVICE_METHOD: i32 = 2;

fn add_services(file: &FileDescriptorProto, index: &mut DescriptorIndex) {
    let comment = |path: &[i32]| -> Vec<String> {
        file.source_code_info
            .as_ref()
            .and_then(|info| info.location.iter().find(|loc| loc.path == path))
            .map(|loc| clean_comment(loc.leading_comments()))
            .unwrap_or_default()
    };

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

/// Clean a proto leading comment into rustdoc lines: drop the javadoc-style
/// `*` filler lines and the common leading space, trim trailing whitespace.
fn clean_comment(comment: &str) -> Vec<String> {
    let mut lines: Vec<String> = comment
        .lines()
        .map(|line| {
            let line = line.strip_prefix(' ').unwrap_or(line).trim_end();
            if line == "*" { "" } else { line }.to_owned()
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
            // proto3 `optional` is encoded as a synthetic single-field oneof;
            // fold it into an explicit-presence cardinality instead.
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
