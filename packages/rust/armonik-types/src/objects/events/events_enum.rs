/// Represents the events that can be returned in the EventSubscriptionResponse
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Enum)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(enum = "armonik.api.grpc.v1.events.EventsEnum")]
pub enum EventsEnum {
    /// New task
    NewTask,
    /// Task status update
    TaskStatusUpdate,
    /// New result
    NewResult,
    /// Result status update
    ResultStatusUpdate,
    /// Result owner update
    ResultOwnerUpdate,
    /// Unspecified (zero) or an event unknown to this crate version.
    Other(OtherEventsEnum),
}
