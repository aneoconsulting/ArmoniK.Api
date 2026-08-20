//! TEMPORARY refactor harness: dump the full expansion of one representative input per shape to
//! `$ARMONIK_SNAPSHOT_DIR`, so a before/after diff of the emitter rewrite can be reviewed token by
//! token. Not part of the crate's test contract; delete when the refactor lands.

use quote::ToTokens;

/// Compile the fixture schema once and point the descriptor loader at it (same trick as the oneof
/// emitter tests).
fn fixture_index() -> std::sync::Arc<crate::descriptor::DescriptorIndex> {
    use prost::Message as _;

    static INDEX: std::sync::OnceLock<std::sync::Arc<crate::descriptor::DescriptorIndex>> =
        std::sync::OnceLock::new();
    std::sync::Arc::clone(INDEX.get_or_init(|| {
        let dir = std::env::temp_dir().join("armonik-macros-snapshot-fixture");
        std::fs::create_dir_all(&dir).expect("create the fixture directory");
        let descriptor = protox::compile(["tests/fixture.proto"], ["tests"])
            .expect("compile tests/fixture.proto")
            .encode_to_vec();
        std::fs::write(dir.join("descriptor.bin"), &descriptor).expect("write the descriptor set");
        std::env::set_var("OUT_DIR", &dir);
        crate::descriptor::index().expect("the fixture index loads")
    }))
}

fn pretty(tokens: proc_macro2::TokenStream) -> String {
    match syn::parse2::<syn::File>(tokens.clone()) {
        Ok(file) => prettyplease::unparse(&file),
        Err(_) => tokens.to_string(),
    }
}

/// Mirror of the `message` entry point, minus the proc_macro boundary.
fn expand_message(input: proc_macro2::TokenStream) -> String {
    let mut input: syn::DeriveInput = syn::parse2(input).expect("input parses");
    let ir = match crate::resolve::resolve_message(&input) {
        Ok(ir) => ir,
        Err(errors) => panic!("resolves: {}", errors.into_syn_error()),
    };
    let anchors = crate::item::anchors(&input, crate::item::Kind::Message);
    let absorbed = crate::absorbed(&ir.absorbs);
    crate::item::rewrite(&mut input, &ir);
    pretty(
        [
            input.into_token_stream(),
            anchors,
            crate::emit::message(&ir),
            absorbed,
        ]
        .into_iter()
        .collect(),
    )
}

/// Mirror of the `enumeration` entry point.
fn expand_enumeration(input: proc_macro2::TokenStream) -> String {
    let mut input: syn::DeriveInput = syn::parse2(input).expect("input parses");
    let plan = match crate::enumeration::resolve_enumeration(&input) {
        Ok(plan) => plan,
        Err(errors) => panic!("resolves: {}", errors.into_syn_error()),
    };
    let anchors = crate::item::anchors(&input, crate::item::Kind::Enumeration);
    let absorbed = crate::absorbed(&plan.absorbs);
    let wire = match &plan.mode {
        crate::plan::EnumMode::Plain { names } => crate::enumeration::plain_wire(&plan, names),
        crate::plan::EnumMode::Transparent { names, path } => {
            crate::enumeration::transparent_wire(&plan, names, path)
        }
    };
    crate::item::rewrite_enum(&mut input, &plan);
    pretty(
        [
            input.into_token_stream(),
            anchors,
            wire,
            crate::enumeration::items(&plan),
            absorbed,
        ]
        .into_iter()
        .collect(),
    )
}

#[test]
fn dump_expansions() {
    let Some(dir) = std::env::var_os("ARMONIK_SNAPSHOT_DIR") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    std::fs::create_dir_all(&dir).expect("create the snapshot directory");
    let _ = fixture_index();

    let messages: &[(&str, proc_macro2::TokenStream)] = &[
        (
            "plain_struct",
            quote::quote! {
                /// Hand-written note.
                #[derive(Debug, Clone, Default, PartialEq, Eq)]
                #[armonik(message = "fixture.Simple")]
                pub struct Simple {
                    pub name: String,
                    pub count: i32,
                }
            },
        ),
        (
            "tuple_struct",
            quote::quote! {
                #[derive(Debug, Clone, Default, PartialEq, Eq)]
                #[armonik(message = "fixture.Simple")]
                pub struct Pair(
                    #[armonik(rename = "name")] pub String,
                    #[armonik(rename = "count")] pub i32,
                );
            },
        ),
        (
            "struct_with_adapter_and_map",
            quote::quote! {
                #[derive(Debug, Clone, Default, PartialEq, Eq)]
                #[armonik(message = "fixture.Typed")]
                pub struct Typed {
                    pub colour: Colour,
                    #[armonik(with = "crate::codec::adapters::PairMap", absorbs = "fixture.Fake")]
                    pub simple: Simple,
                    pub tags: Vec<String>,
                    pub labels: std::collections::HashMap<String, String>,
                }
            },
        ),
        (
            "struct_with_oneof_field",
            quote::quote! {
                #[derive(Debug, Clone, Default, PartialEq, Eq)]
                #[armonik(message = "fixture.Choice")]
                pub struct ChoiceMsg {
                    pub shared: String,
                    pub choice: ChoiceOneof,
                }
            },
        ),
        (
            "transparent_struct",
            quote::quote! {
                #[derive(Debug, Clone, Default, PartialEq, Eq)]
                #[armonik(transparent, message = "fixture.ColourWrapper")]
                pub struct Wrap {
                    pub inner: Inner,
                }
            },
        ),
        (
            "generic_struct",
            quote::quote! {
                #[derive(Debug, Clone, Default, PartialEq, Eq)]
                #[armonik(generic)]
                pub struct Sort<T> {
                    #[armonik(tag = 1)]
                    pub field: T,
                    #[armonik(tag = 2, with = "crate::codec::adapters::PairMap")]
                    pub direction: i32,
                }
            },
        ),
        (
            "embedded_oneof",
            quote::quote! {
                #[derive(Debug, Clone, Default, PartialEq, Eq)]
                #[armonik(message = "fixture.Choice", oneof = "choice")]
                pub enum ChoiceOneof {
                    #[default]
                    None,
                    Text(String),
                    #[armonik(rename = "simple", with = "crate::MyAdapter")]
                    Other(Simple),
                    #[armonik(present)]
                    Flag,
                    #[armonik(inline)]
                    Hostile { buf: String, len: i32, value: String, body_len: String },
                }
            },
        ),
        (
            "whole_message_enum_with_sibling",
            quote::quote! {
                #[derive(Debug, Clone, Default, PartialEq, Eq)]
                #[armonik(message = "fixture.Shared")]
                pub enum Shared {
                    #[default]
                    Unset { token: String },
                    Text { token: String, text: String },
                    Flag { token: String, flag: EmptyMsg },
                }
            },
        ),
        (
            "whole_message_enum_straddled",
            quote::quote! {
                #[derive(Debug, Clone, Default, PartialEq, Eq)]
                #[armonik(message = "fixture.Straddled")]
                pub enum Straddled {
                    Text { token: String, text: String },
                    Other { token: String, other: String },
                }
            },
        ),
        (
            "whole_message_enum_tuple",
            quote::quote! {
                #[derive(Debug, Clone, Default, PartialEq, Eq)]
                #[armonik(message = "fixture.OnlyOneof")]
                pub enum OnlyOneof {
                    #[default]
                    Unset,
                    First(String),
                    Second(String),
                }
            },
        ),
    ];
    for (name, input) in messages {
        std::fs::write(
            dir.join(format!("{name}.rs")),
            expand_message(input.clone()),
        )
        .expect("write the snapshot");
    }

    let enumerations: &[(&str, proc_macro2::TokenStream)] = &[
        (
            "plain_enumeration",
            quote::quote! {
                #[derive(Debug, Clone, Copy)]
                #[armonik(enum = "fixture.Colour")]
                pub enum Colour {
                    Red,
                    #[armonik(rename = "COLOUR_GREEN")]
                    Green,
                    /// Unknown values.
                    Unknown(UnknownColour),
                }
            },
        ),
        (
            "plain_enumeration_with_zero",
            quote::quote! {
                #[derive(Debug, Clone, Copy)]
                #[armonik(enum = "fixture.Colour")]
                pub enum Colour {
                    Unspecified,
                    Red,
                    Green,
                    Unknown(UnknownColour),
                }
            },
        ),
        (
            "transparent_enumeration",
            quote::quote! {
                #[derive(Debug, Clone, Copy)]
                #[armonik(transparent, message = "fixture.ColourWrapper")]
                pub enum ColourField {
                    Unspecified,
                    Red,
                    Green,
                    Unknown(UnknownColourField),
                }
            },
        ),
    ];
    for (name, input) in enumerations {
        std::fs::write(
            dir.join(format!("{name}.rs")),
            expand_enumeration(input.clone()),
        )
        .expect("write the snapshot");
    }
}
