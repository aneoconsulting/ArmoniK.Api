use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(i32)]
pub enum EventsEnum {
    /// Unspecified
    #[default]
    Unspecified = 0,
    /// New task
    NewTask = 1,
    /// Task status update
    TaskStatusUpdate = 2,
    /// New result
    NewResult = 3,
    /// Result status update
    ResultStatusUpdate = 4,
    /// Result owner update
    ResultOwnerUpdate = 5,
}

impl From<i32> for EventsEnum {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Unspecified,
            1 => Self::NewTask,
            2 => Self::TaskStatusUpdate,
            3 => Self::NewResult,
            4 => Self::ResultStatusUpdate,
            5 => Self::ResultOwnerUpdate,
            _ => Default::default(),
        }
    }
}

impl From<EventsEnum> for v3::events::EventsEnum {
    fn from(value: EventsEnum) -> Self {
        match value {
            EventsEnum::Unspecified => Self::Unspecified,
            EventsEnum::NewTask => Self::NewTask,
            EventsEnum::TaskStatusUpdate => Self::TaskStatusUpdate,
            EventsEnum::NewResult => Self::NewResult,
            EventsEnum::ResultStatusUpdate => Self::ResultStatusUpdate,
            EventsEnum::ResultOwnerUpdate => Self::ResultOwnerUpdate,
        }
    }
}

impl From<v3::events::EventsEnum> for EventsEnum {
    fn from(value: v3::events::EventsEnum) -> Self {
        match value {
            v3::events::EventsEnum::Unspecified => Self::Unspecified,
            v3::events::EventsEnum::NewTask => Self::NewTask,
            v3::events::EventsEnum::TaskStatusUpdate => Self::TaskStatusUpdate,
            v3::events::EventsEnum::NewResult => Self::NewResult,
            v3::events::EventsEnum::ResultStatusUpdate => Self::ResultStatusUpdate,
            v3::events::EventsEnum::ResultOwnerUpdate => Self::ResultOwnerUpdate,
        }
    }
}

super::super::impl_convert!(req EventsEnum : v3::events::EventsEnum);
