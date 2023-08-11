use crate::api::v3;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InitKeyedDataStream {
    Key(String),
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

super::impl_convert!(InitKeyedDataStream : Option<v3::InitKeyedDataStream>);
