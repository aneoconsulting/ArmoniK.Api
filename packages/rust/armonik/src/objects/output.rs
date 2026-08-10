#[armonik_macros::message]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[armonik(message = "armonik.api.grpc.v1.Output")]
pub enum Output {
    #[default]
    #[armonik(present)]
    Ok,
    Error {
        details: String,
    },
}
