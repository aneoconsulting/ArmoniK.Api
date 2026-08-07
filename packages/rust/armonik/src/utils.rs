/// Used by the client conveniences, which widen their arguments to `impl IntoIterator`.
#[cfg(feature = "_gen-client")]
pub(crate) trait IntoCollection<T> {
    fn into_collect(self) -> T;
}

#[cfg(feature = "_gen-client")]
impl<X, Y, TX, TY> IntoCollection<TY> for TX
where
    X: Into<Y>,
    TX: IntoIterator<Item = X>,
    TY: IntoIterator<Item = Y>,
    TY: std::iter::FromIterator<Y>,
{
    fn into_collect(self) -> TY {
        self.into_iter().map(Into::into).collect()
    }
}

/// The nested-filter collect shared by the `list`/`subscribe` convenience
/// methods: two levels of `impl IntoIterator` into the service's
/// `filter::Or { or: Vec<filter::And> }` shape.
#[cfg(feature = "_gen-client")]
pub(crate) fn into_filters<Field, And, Or>(
    filters: impl IntoIterator<Item = impl IntoIterator<Item = Field>>,
) -> Or
where
    And: FromIterator<Field>,
    Or: FromIterator<And>,
{
    filters
        .into_iter()
        .map(|fields| fields.into_iter().collect())
        .collect()
}

#[cfg(feature = "serde")]
pub(crate) mod serde_timestamp {
    pub(crate) fn serialize<S: serde::Serializer>(
        value: &prost_types::Timestamp,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(&(value.seconds, value.nanos), serializer)
    }

    pub(crate) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<prost_types::Timestamp, D::Error> {
        let (seconds, nanos): (i64, i32) = serde::Deserialize::deserialize(deserializer)?;
        Ok(prost_types::Timestamp { seconds, nanos })
    }
}
#[cfg(feature = "serde")]
pub(crate) mod serde_option_timestamp {
    pub(crate) fn serialize<S: serde::Serializer>(
        value: &Option<prost_types::Timestamp>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(
            &value.as_ref().map(|value| (value.seconds, value.nanos)),
            serializer,
        )
    }

    pub(crate) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<prost_types::Timestamp>, D::Error> {
        Ok(
            <Option<(i64, i32)> as serde::Deserialize>::deserialize(deserializer)?
                .map(|(seconds, nanos)| prost_types::Timestamp { seconds, nanos }),
        )
    }
}

#[cfg(feature = "serde")]
pub(crate) mod serde_duration {
    pub(crate) fn serialize<S: serde::Serializer>(
        value: &prost_types::Duration,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(&(value.seconds, value.nanos), serializer)
    }

    pub(crate) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<prost_types::Duration, D::Error> {
        let (seconds, nanos): (i64, i32) = serde::Deserialize::deserialize(deserializer)?;
        Ok(prost_types::Duration { seconds, nanos })
    }
}
#[cfg(feature = "serde")]
pub(crate) mod serde_option_duration {
    pub(crate) fn serialize<S: serde::Serializer>(
        value: &Option<prost_types::Duration>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(
            &value.as_ref().map(|value| (value.seconds, value.nanos)),
            serializer,
        )
    }

    pub(crate) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<prost_types::Duration>, D::Error> {
        Ok(
            <Option<(i64, i32)> as serde::Deserialize>::deserialize(deserializer)?
                .map(|(seconds, nanos)| prost_types::Duration { seconds, nanos }),
        )
    }
}

/// Implement all traits and functions to define a wrapper around a [`Vec`]
///
/// # Examples
///
/// The `{field: Type}` form (a named-field wrapper) implements everything,
/// including `FromIterator`:
///
/// ```ignore
/// struct Foo();
/// struct Bar{ bar: Vec<Foo>};
///
/// crate::utils::impl_vec_wrapper!(Bar{bar: Foo});
/// ```
///
/// The `[field: Type]` form skips `FromIterator` (for a wrapper that carries
/// other fields too):
///
/// ```ignore
/// struct Foo();
/// struct Bar{ bar: Vec<Foo>, dummy: i64};
///
/// crate::utils::impl_vec_wrapper!(Bar[bar: Foo]);
/// ```
macro_rules! impl_vec_wrapper {
    ($wrapper:ident{$inner:ident: $inner_type:ty}) => {
        crate::utils::impl_vec_wrapper!($wrapper[$inner: $inner_type]);

        impl FromIterator<$inner_type> for $wrapper {
            fn from_iter<T: IntoIterator<Item = $inner_type>>(iter: T) -> Self {
                Self{$inner: iter.into_iter().collect()}
            }
        }
    };
    ($wrapper:ident[$inner:tt: $inner_type:ty]) => {
        impl $wrapper {
            pub fn iter(&self) -> std::slice::Iter<'_, $inner_type> {
                self.$inner.iter()
            }
            pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, $inner_type> {
                self.$inner.iter_mut()
            }
        }

        impl IntoIterator for $wrapper {
            type Item = $inner_type;

            type IntoIter = std::vec::IntoIter<$inner_type>;

            fn into_iter(self) -> Self::IntoIter {
                self.$inner.into_iter()
            }
        }

        impl<'a> IntoIterator for &'a $wrapper {
            type Item = &'a $inner_type;

            type IntoIter = std::slice::Iter<'a, $inner_type>;

            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }

        impl<'a> IntoIterator for &'a mut $wrapper {
            type Item = &'a mut $inner_type;

            type IntoIter = std::slice::IterMut<'a, $inner_type>;

            fn into_iter(self) -> Self::IntoIter {
                self.iter_mut()
            }
        }

        impl AsRef<[$inner_type]> for $wrapper {
            fn as_ref(&self) -> &[$inner_type] {
                &self.$inner
            }
        }

        impl AsMut<[$inner_type]> for $wrapper {
            fn as_mut(&mut self) -> &mut [$inner_type] {
                &mut self.$inner
            }
        }

        impl AsRef<Vec<$inner_type>> for $wrapper {
            fn as_ref(&self) -> &Vec<$inner_type> {
                &self.$inner
            }
        }

        impl AsMut<Vec<$inner_type>> for $wrapper {
            fn as_mut(&mut self) -> &mut Vec<$inner_type> {
                &mut self.$inner
            }
        }

        impl std::borrow::Borrow<[$inner_type]> for $wrapper {
            fn borrow(&self) -> &[$inner_type] {
                &self.$inner
            }
        }

        impl std::borrow::BorrowMut<[$inner_type]> for $wrapper {
            fn borrow_mut(&mut self) -> &mut [$inner_type] {
                &mut self.$inner
            }
        }

        impl std::borrow::Borrow<Vec<$inner_type>> for $wrapper {
            fn borrow(&self) -> &Vec<$inner_type> {
                &self.$inner
            }
        }

        impl std::borrow::BorrowMut<Vec<$inner_type>> for $wrapper {
            fn borrow_mut(&mut self) -> &mut Vec<$inner_type> {
                &mut self.$inner
            }
        }

        impl std::ops::Deref for $wrapper {
            type Target = Vec<$inner_type>;

            fn deref(&self) -> &Self::Target {
                &self.$inner
            }
        }

        impl std::ops::DerefMut for $wrapper {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.$inner
            }
        }
    };
}

pub(crate) use impl_vec_wrapper;
