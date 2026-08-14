#[armonik_macros::enumeration]
#[derive(Debug, Clone, Copy)]
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
