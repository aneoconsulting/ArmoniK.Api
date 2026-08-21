#[armonik_macros::enumeration("armonik.api.grpc.v1.events.EventsEnum")]
#[derive(Debug, Clone, Copy)]
pub enum EventsEnum {
    NewTask,
    TaskStatusUpdate,
    NewResult,
    ResultStatusUpdate,
    ResultOwnerUpdate,
    /// Unspecified (zero) or an event unknown to this crate version.
    Unknown(UnknownEventsEnum),
}
