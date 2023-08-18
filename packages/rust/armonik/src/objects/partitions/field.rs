use crate::api::v3;

/// Represents every available field in a partition.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum Field {
    /// Unspecified.
    Unspecified = 0,
    /// The partition ID.
    #[default]
    Id = 1,
    /// The parent partition IDs.
    ParentPartitionIds = 2,
    /// Whether the partition is reserved for pods.
    PodReserved = 3,
    /// The maximum number of pods that can be used by sessions using the partition.
    PodMax = 4,
    /// The percentage of the partition that can be preempted.
    PreemptionPercentage = 5,
    /// The priority of the partition.
    Priority = 6,
}

impl From<i32> for Field {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Unspecified,
            1 => Self::Id,
            2 => Self::ParentPartitionIds,
            3 => Self::PodReserved,
            4 => Self::PodMax,
            5 => Self::PreemptionPercentage,
            6 => Self::Priority,
            _ => Self::Unspecified,
        }
    }
}

impl From<Field> for v3::partitions::PartitionField {
    fn from(value: Field) -> Self {
        Self {
            field: Some(v3::partitions::partition_field::Field::PartitionRawField(
                v3::partitions::PartitionRawField {
                    field: value as i32,
                },
            )),
        }
    }
}

impl From<v3::partitions::PartitionField> for Field {
    fn from(value: v3::partitions::PartitionField) -> Self {
        match value.field {
            Some(v3::partitions::partition_field::Field::PartitionRawField(field)) => {
                Self::from(field.field)
            }
            None => Self::Unspecified,
        }
    }
}

super::super::impl_convert!(Field : Option<v3::partitions::PartitionField>);
