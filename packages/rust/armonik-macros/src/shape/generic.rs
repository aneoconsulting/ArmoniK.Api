//! `#[armonik(generic)]`: no descriptor to validate against, so every field carries its own tag and
//! the concrete instantiations are covered through their `#[armonik_macros::alias]` sites.

use crate::attr_site::{field_access, scan_attrs, Allowed, FieldAttrs};
use crate::attrs::Errors;
use crate::descriptor::DescriptorIndex;
use crate::plan::{MessagePlan, Mode, Slot, SlotCodec};

/// Plan for a generic type: no descriptor validation, explicit tags; the concrete instantiations
/// are covered by the differential harness.
pub(crate) fn generic_plan(
    input: &syn::DeriveInput,
    index: &DescriptorIndex,
    mut errors: Errors,
) -> Result<MessagePlan, Errors> {
    let syn::Data::Struct(data) = &input.data else {
        errors.at(input.ident.span(), "#[armonik(generic)] expects a struct");
        return Err(errors);
    };

    let mut fields = Vec::new();
    for (field_index, field) in data.fields.iter().enumerate() {
        let (span, access) = field_access(field, field_index);
        let Some(FieldAttrs { tag, with, .. }) = scan_attrs(
            &field.attrs,
            Allowed {
                tag: true,
                with: true,
                ..Allowed::default()
            },
            "generic-mode fields only take tag = ... and with = ...",
            &mut errors,
        ) else {
            continue;
        };
        let tag = tag.map(|(_, tag)| tag);
        let with = with.map(|(_, ty)| ty);
        let Some(tag) = tag else {
            errors.at(
                span,
                "generic-mode fields need an explicit #[armonik(tag = ...)]",
            );
            continue;
        };

        let proto_path = format!(
            "{}.{}",
            input.ident,
            field
                .ident
                .as_ref()
                .map(|ident| ident.to_string())
                .unwrap_or_else(|| field_index.to_string())
        );
        fields.push(Slot {
            access: Some(access),
            span,
            tag,
            codec: SlotCodec::Field {
                ty: Box::new(field.ty.clone()),
                adapter: with.map(Box::new),
            },
            checks: None,
            proto_path,
            // A generic type names no proto message, so there is nothing to harvest.
            docs: Vec::new(),
        });
    }

    errors.into_result()?;

    fields.sort_by_key(|field| field.tag);
    Ok(MessagePlan {
        ident: input.ident.clone(),
        proto_names: Vec::new(),
        docs: Vec::new(),
        fields,
        generics: input.generics.clone(),
        fingerprint: index.fingerprint,
        mode: Mode::Generic,
        absorbs: Vec::new(),
    })
}
