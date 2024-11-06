use snafu::Snafu;

pub(crate) trait IntoCollection<T> {
    fn into_collect(self) -> T;
}

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

pub(crate) fn read_env(name: &str) -> Result<String, ReadEnvError> {
    match std::env::var(name) {
        Ok(value) => Ok(value),
        Err(std::env::VarError::NotPresent) => Ok(String::new()),
        Err(std::env::VarError::NotUnicode(value)) => NotUnicodeSnafu {
            name: name.to_owned(),
            value,
        }
        .fail(),
    }
}

pub(crate) fn read_env_bool(name: &str) -> Result<bool, ReadEnvError> {
    let value = read_env(name)?;
    match value.as_ref() {
        "0" | "false" | "no" | "disable" | "disallow" | "forbid" | "" => Ok(false),
        "1" | "true" | "yes" | "enable" | "allow" | "authorize" => Ok(true),
        _ => NotBooleanSnafu {
            name: name.to_owned(),
            value,
        }
        .fail(),
    }
}

#[derive(Debug, Snafu)]
#[non_exhaustive]
pub enum ReadEnvError {
    #[snafu(display(
        "Environment variable `{name}={value:?}` is not a valid unicode string [{location}]"
    ))]
    #[non_exhaustive]
    NotUnicode {
        name: String,
        value: std::ffi::OsString,
        backtrace: snafu::Backtrace,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("Environment variable `{name}={value}` is not a valid boolean [{location}]"))]
    #[non_exhaustive]
    NotBoolean {
        name: String,
        value: String,
        backtrace: snafu::Backtrace,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}

#[derive(Debug)]
pub(crate) struct InsecureCertVerifier;

#[cfg(feature = "_gen-client")]
impl rustls::client::danger::ServerCertVerifier for InsecureCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA1,
            rustls::SignatureScheme::ECDSA_SHA1_Legacy,
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::ED448,
        ]
    }
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
/// ```ignore
/// struct Foo();
/// struct Bar(Vec<Foo>);
///
/// crate::utils::impl_vec_wrapper!(Bar(Foo));
/// ```
///
/// ```ignore
/// struct Foo();
/// struct Bar{ bar: Vec<Foo>};
///
/// crate::utils::impl_vec_wrapper!(Bar{bar: Foo});
/// ```
///
/// # Examples without FromIterator
///
/// ```ignore
/// struct Foo();
/// struct Bar(Vec<Foo>, i64);
///
/// crate::utils::impl_vec_wrapper!(Bar[0: Foo]);
/// ```
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
    ($wrapper:ident($inner_type:ty)) => {
        crate::utils::impl_vec_wrapper!($wrapper[0: $inner_type]);

        impl FromIterator<$inner_type> for $wrapper {
            fn from_iter<T: IntoIterator<Item = $inner_type>>(iter: T) -> Self {
                Self(iter.into_iter().collect())
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
