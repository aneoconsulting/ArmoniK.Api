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
use prost_types::{DescriptorProto, EnumDescriptorProto, FieldDescriptorProto, FileDescriptorSet};

use crate::kind::{Cardinality, FieldKind};

/// A field of a protobuf message, as seen by the derives.
pub(crate) struct FieldMeta {
    pub(crate) name: String,
    pub(crate) tag: u32,
    pub(crate) kind: FieldKind,
    pub(crate) cardinality: Cardinality,
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
}

/// Index of every message and enum in the descriptor set, keyed by full name
/// without leading dot (e.g. `armonik.api.grpc.v1.TaskOptions`).
pub(crate) struct DescriptorIndex {
    pub(crate) fingerprint: u64,
    pub(crate) messages: HashMap<String, MessageMeta>,
    pub(crate) enums: HashMap<String, EnumMeta>,
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
    };
    for file in &fds.file {
        let prefix = file.package();
        let packed_default = file.syntax() != "proto2";
        for message in &file.message_type {
            add_message(prefix, message, packed_default, &map_entries, &mut index)?;
        }
        for enumeration in &file.enum_type {
            add_enum(prefix, enumeration, &mut index);
        }
    }
    Ok(index)
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
    packed_default: bool,
    map_entries: &HashMap<String, (FieldKind, FieldKind)>,
    index: &mut DescriptorIndex,
) -> Result<(), String> {
    let full = full_name(prefix, message.name());
    if message.options.as_ref().is_some_and(|o| o.map_entry()) {
        // Synthetic map entries are folded into their parent's map fields.
        return Ok(());
    }

    for nested in &message.nested_type {
        add_message(&full, nested, packed_default, map_entries, index)?;
    }
    for enumeration in &message.enum_type {
        add_enum(&full, enumeration, index);
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
    for field in &message.field {
        let kind = field_kind(field)
            .map_err(|err| format!("in message `{full}`, field `{}`: {err}", field.name()))?;

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
                    (Cardinality::Repeated { packed: false }, None)
                }
            } else {
                let packed = field
                    .options
                    .as_ref()
                    .and_then(|options| options.packed)
                    .unwrap_or(packed_default && kind.packable());
                (Cardinality::Repeated { packed }, None)
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
            oneof,
        });
    }

    index.messages.insert(full, MessageMeta { fields, oneofs });
    Ok(())
}

fn add_enum(prefix: &str, enumeration: &EnumDescriptorProto, index: &mut DescriptorIndex) {
    let full = full_name(prefix, enumeration.name());
    let values = enumeration
        .value
        .iter()
        .map(|value| (value.name().to_owned(), value.number()))
        .collect();
    index.enums.insert(full, EnumMeta { values });
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
        Type::Sint32 => FieldKind::SInt32,
        Type::Sint64 => FieldKind::SInt64,
        Type::Fixed32 => FieldKind::Fixed32,
        Type::Fixed64 => FieldKind::Fixed64,
        Type::Sfixed32 => FieldKind::SFixed32,
        Type::Sfixed64 => FieldKind::SFixed64,
        Type::Bool => FieldKind::Bool,
        Type::String => FieldKind::String,
        Type::Bytes => FieldKind::Bytes,
        Type::Message => FieldKind::Message(type_name()),
        Type::Enum => FieldKind::Enum(type_name()),
        Type::Group => return Err("proto2 groups are not supported".to_owned()),
    })
}

