#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.Output")]
pub enum Output {
    #[default]
    #[armonik(present)]
    Ok,
    Error {
        details: String,
    },
}
