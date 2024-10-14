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
pub enum ReadEnvError {
    #[snafu(display(
        "Environment variable `{name}={value:?}` is not a valid unicode string [{location}]"
    ))]
    NotUnicode {
        name: String,
        value: std::ffi::OsString,
        backtrace: snafu::Backtrace,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("Environment variable `{name}={value}` is not a valid boolean [{location}]"))]
    NotBoolean {
        name: String,
        value: String,
        backtrace: snafu::Backtrace,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}
