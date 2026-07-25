use crate::api::v3;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.InitKeyedDataStream", oneof = "type")]
pub enum InitKeyedDataStream {
    Key(String),
    #[armonik(present)]
    LastResult,
}

impl Default for InitKeyedDataStream {
    fn default() -> Self {
        Self::Key(Default::default())
    }
}

impl From<InitKeyedDataStream> for v3::InitKeyedDataStream {
    fn from(value: InitKeyedDataStream) -> Self {
        match value {
            InitKeyedDataStream::Key(key) => Self {
                r#type: Some(v3::init_keyed_data_stream::Type::Key(key)),
            },
            InitKeyedDataStream::LastResult => Self {
                r#type: Some(v3::init_keyed_data_stream::Type::LastResult(true)),
            },
        }
    }
}

impl From<v3::InitKeyedDataStream> for InitKeyedDataStream {
    fn from(value: v3::InitKeyedDataStream) -> Self {
        match value.r#type {
            Some(v3::init_keyed_data_stream::Type::Key(key)) => Self::Key(key),
            Some(v3::init_keyed_data_stream::Type::LastResult(_)) => Self::LastResult,
            None => Default::default(),
        }
    }
}

super::impl_convert!(req InitKeyedDataStream : v3::InitKeyedDataStream);
