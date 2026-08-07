#[armonik_macros::enumeration]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(enum = "armonik.api.grpc.v1.events.EventsEnum")]
pub enum EventsEnum {
    NewTask,
    TaskStatusUpdate,
    NewResult,
    ResultStatusUpdate,
    ResultOwnerUpdate,
    /// Unspecified (zero) or an event unknown to this crate version.
    Unknown(UnknownEventsEnum),
}
