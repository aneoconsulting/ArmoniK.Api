//! When a failed request is worth sending again, and how long to wait first.
//!
//! A unit of fields, not a naming scheme: the embedding composes them with `#[serde(flatten)]`
//! under a prefix of its own, so the same unit serves however many embeddings read retry options,
//! and grouping these fields changes no environment variable a deployment already sets.
//!
//! The schedule follows the gRPC retry specification: the wait before replay `n` is bounded by
//! `min(initial * multiplier^n, max)`. The defaults are the ones ArmoniK's other clients hand
//! grpc-dotnet, so a deployment behaves the same whichever client talks to it.

use std::time::Duration;

#[cfg(feature = "serde")]
use crate::config::{ConfigError, IncompatibleOptionsSnafu};

/// Attempts in all, first try included, when the option is left unset.
const DEFAULT_MAX_ATTEMPTS: u32 = 5;
/// Wait before the second attempt, when the option is left unset.
const DEFAULT_INITIAL_BACK_OFF: Duration = Duration::from_secs(1);
/// Ceiling the wait grows to, when the option is left unset.
const DEFAULT_MAX_BACK_OFF: Duration = Duration::from_secs(5);
/// What each wait is multiplied by, when the option is left unset.
const DEFAULT_BACK_OFF_MULTIPLIER: f64 = 1.5;
/// gRPC status codes, as the wire numbers them.
///
/// Spelled out rather than taken from a gRPC crate: this crate reads options and builds connectors,
/// and naming three constants is cheaper than depending on a whole gRPC stack for them.
mod code {
    pub const UNKNOWN: i32 = 2;
    pub const ABORTED: i32 = 10;
    pub const UNAVAILABLE: i32 = 14;
}

/// Failures worth sending the request again for, the same three ArmoniK's other clients fix.
const DEFAULT_RETRYABLE_STATUS_CODES: [i32; 3] = [code::UNAVAILABLE, code::ABORTED, code::UNKNOWN];

/// Replaying a failed request: how many attempts, how long between them, and which failures are
/// worth another try.
///
/// Each option is a field of its own name; the full name a source spells is the embedding's prefix
/// followed by that name, so no field doc spells it: the same field is a different option under a
/// different prefix. A schema generated for this type on its own describes the unprefixed names it
/// declares.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize),
    serde(try_from = "RawRetry")
)]
#[non_exhaustive]
pub struct RetryConfig {
    /// Attempts in all for one request, first try included; `1` never replays.
    pub max_attempts: u32,
    /// Wait before the second attempt.
    pub initial_back_off: Duration,
    /// Ceiling the wait grows to, never below [`Self::initial_back_off`].
    pub max_back_off: Duration,
    /// What each wait is multiplied by.
    pub back_off_multiplier: f64,
    /// Failures worth sending the request again for, as gRPC status codes: 14 `UNAVAILABLE`, 10
    /// `ABORTED`, 2 `UNKNOWN`.
    ///
    /// The wire numbers, so that naming them costs no gRPC dependency here; whoever makes the calls
    /// converts from the type its own stack reports.
    ///
    /// Set programmatically: no option reads it, as in ArmoniK's other clients, which fix the same
    /// three codes.
    pub retryable_status_codes: Vec<i32>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: DEFAULT_MAX_ATTEMPTS,
            initial_back_off: DEFAULT_INITIAL_BACK_OFF,
            max_back_off: DEFAULT_MAX_BACK_OFF,
            back_off_multiplier: DEFAULT_BACK_OFF_MULTIPLIER,
            retryable_status_codes: DEFAULT_RETRYABLE_STATUS_CODES.to_vec(),
        }
    }
}

impl RetryConfig {
    /// Whether a request that failed with gRPC status `code` is worth sending again.
    pub fn is_retryable(&self, code: i32) -> bool {
        self.retryable_status_codes.contains(&code)
    }

    /// The longest each replay may wait, one item per replay and nothing past the last attempt:
    /// `initial_back_off`, each following one multiplied by `back_off_multiplier`, never above
    /// `max_back_off`.
    ///
    /// A bound rather than the wait itself, because the specification draws the wait uniformly
    /// below it so that clients which failed together do not come back together.
    pub fn bounds(&self) -> impl Iterator<Item = Duration> + use<> {
        let ceiling = self.max_back_off;
        let multiplier = self.back_off_multiplier;
        // A ceiling below the first wait is refused while reading the options, but the fields are
        // public and a caller may set either, so the first bound is held to the ceiling too.
        let mut bound = self.initial_back_off.min(ceiling);

        (0..self.max_attempts.saturating_sub(1)).map(move |_| {
            let current = bound;
            // `Duration::mul_f64` panics on a multiplier that is negative, infinite or not a
            // number, so the growth goes through the fallible constructor and settles on the
            // ceiling for anything it refuses.
            bound = Duration::try_from_secs_f64(bound.as_secs_f64() * multiplier)
                .unwrap_or(ceiling)
                .min(ceiling);
            current
        })
    }
}

/// The flat string options [`RetryConfig`] is read from, one per field, all optional.
///
/// Every field tolerates the eager typing a `serde` source may apply (a bare number arriving as
/// one), and an empty string means unset, the same as an absent key: a deployment that declares a
/// variable with an empty default must not differ from one that leaves it out.
///
/// Each value is held in the type that states what it accepts, so a refusal is raised while the
/// source's own key is still known and the message needs no option name written into it.
#[cfg(feature = "serde")]
#[derive(Debug, Default, serde::Deserialize)]
#[cfg_attr(
    feature = "schema",
    derive(schemars::JsonSchema),
    schemars(transform = crate::config::strip_rust_details)
)]
#[serde(rename_all = "PascalCase", default)]
pub(crate) struct RawRetry {
    /// Attempts in all for one request, first try included; `1` never replays, `0` is refused, and
    /// empty is the default.
    #[serde(deserialize_with = "crate::config_utils::optional_parsed")]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    max_attempts: Option<std::num::NonZeroU32>,
    /// Wait before the second attempt (e.g. `1s`); empty for the default.
    #[serde(deserialize_with = "crate::config_utils::optional_duration")]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    initial_back_off: Option<Duration>,
    /// Ceiling the wait grows to (e.g. `5s`); empty for the default.
    #[serde(deserialize_with = "crate::config_utils::optional_duration")]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    max_back_off: Option<Duration>,
    /// What each wait is multiplied by; empty for the default.
    #[serde(deserialize_with = "crate::config_utils::optional_parsed")]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    back_off_multiplier: Option<f64>,
}

#[cfg(feature = "serde")]
impl TryFrom<RawRetry> for RetryConfig {
    type Error = ConfigError;

    fn try_from(raw: RawRetry) -> Result<Self, Self::Error> {
        let RawRetry {
            max_attempts,
            initial_back_off,
            max_back_off,
            back_off_multiplier,
        } = raw;

        let initial_back_off = initial_back_off.unwrap_or(DEFAULT_INITIAL_BACK_OFF);
        let max_back_off = max_back_off.unwrap_or(DEFAULT_MAX_BACK_OFF);

        // The two are named here because the mistake belongs to neither key alone: a ceiling below
        // the first wait holds every wait down to it, so one of the two options does nothing and
        // the source cannot say which was meant. Setting only one is enough to reach this, since
        // the other keeps its default.
        snafu::ensure!(
            max_back_off >= initial_back_off,
            IncompatibleOptionsSnafu {
                msg: format!(
                    "`MaxBackOff` ({}) is below `InitialBackOff` ({}), which would hold every \
                     wait down to the ceiling",
                    humantime::format_duration(max_back_off),
                    humantime::format_duration(initial_back_off),
                ),
            }
        );

        let back_off_multiplier = back_off_multiplier.unwrap_or(DEFAULT_BACK_OFF_MULTIPLIER);
        // `f64` parses `nan`, `inf` and every negative as happily as a real multiplier, and none of
        // them backs anything off: below 1 each wait is shorter than the last, and a value that is
        // not a finite number makes the schedule meaningless rather than long.
        snafu::ensure!(
            back_off_multiplier.is_finite() && back_off_multiplier >= 1.0,
            IncompatibleOptionsSnafu {
                msg: format!(
                    "`BackOffMultiplier` ({back_off_multiplier}) is not a finite number of at least 1"
                ),
            }
        );

        Ok(Self {
            max_attempts: max_attempts.map_or(DEFAULT_MAX_ATTEMPTS, std::num::NonZeroU32::get),
            initial_back_off,
            max_back_off,
            back_off_multiplier,
            retryable_status_codes: DEFAULT_RETRYABLE_STATUS_CODES.to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The default policy with `max_attempts` replaced, the only field most tests here vary.
    fn policy(max_attempts: u32) -> RetryConfig {
        RetryConfig {
            max_attempts,
            ..RetryConfig::default()
        }
    }

    /// The policy `BackOffMultiplier=value` produces, whichever way it turns out.
    #[cfg(feature = "serde")]
    fn from_multiplier(value: &str) -> Result<RetryConfig, ConfigError> {
        use serde::Deserialize as _;
        RetryConfig::deserialize(serde::de::value::MapDeserializer::<
            _,
            serde::de::value::Error,
        >::new([("BackOffMultiplier", value)].into_iter()))
        .map_err(|error| {
            IncompatibleOptionsSnafu {
                msg: error.to_string(),
            }
            .build()
        })
    }

    #[cfg(feature = "serde")]
    #[test]
    fn a_multiplier_that_backs_nothing_off_is_refused() {
        // `f64` parses every one of these, and each would make the schedule something other than a
        // backoff: no wait at all, waits that shrink, or a number that is not one.
        for value in ["0", "-1", "0.5", "nan", "inf", "-inf"] {
            let rendered = from_multiplier(value)
                .err()
                .map(|error| error.to_string())
                .unwrap_or_else(|| panic!("`{value}` should be refused"));

            assert!(
                rendered.contains("BackOffMultiplier"),
                "{value}: {rendered}"
            );
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn a_multiplier_of_one_is_a_constant_wait_and_allowed() {
        // The edge of the rule, and a legitimate policy: retry at a fixed interval.
        let policy = from_multiplier("1").expect("a constant wait is a policy");

        assert_eq!(policy.back_off_multiplier, 1.0);
    }

    #[test]
    fn the_defaults_are_the_ones_the_other_clients_use() {
        // A deployment that sets nothing has to behave the same whichever client talks to it.
        let policy = RetryConfig::default();

        assert_eq!(policy.max_attempts, 5);
        assert_eq!(policy.initial_back_off, Duration::from_secs(1));
        assert_eq!(policy.max_back_off, Duration::from_secs(5));
        assert_eq!(policy.back_off_multiplier, 1.5);
        // The literals rather than the constants: what has to hold is that these are the numbers
        // gRPC gives `UNAVAILABLE`, `ABORTED` and `UNKNOWN`, and 3 is `INVALID_ARGUMENT`.
        assert!(policy.is_retryable(14));
        assert!(policy.is_retryable(10));
        assert!(policy.is_retryable(2));
        assert!(!policy.is_retryable(3));
    }

    #[test]
    fn the_bounds_grow_by_the_multiplier_and_stop_at_the_ceiling() {
        let policy = RetryConfig {
            initial_back_off: Duration::from_secs(1),
            max_back_off: Duration::from_secs(4),
            back_off_multiplier: 2.0,
            ..policy(6)
        };

        assert_eq!(
            policy.bounds().collect::<Vec<_>>(),
            [1, 2, 4, 4, 4].map(Duration::from_secs),
            "1, 2, 4, then the ceiling holds"
        );
    }

    #[test]
    fn there_is_one_bound_per_replay_and_none_beyond() {
        assert_eq!(policy(3).bounds().count(), 2, "three attempts, two waits");
        assert_eq!(policy(1).bounds().count(), 0, "one attempt is no replay");
    }

    #[test]
    fn a_multiplier_no_duration_can_hold_settles_on_the_ceiling() {
        // The fields are public, so a caller can set a multiplier `Duration::mul_f64` would panic
        // on. Every such value has to come out as a bounded wait instead.
        for multiplier in [f64::NAN, f64::INFINITY, -1.0, f64::MAX] {
            let policy = RetryConfig {
                initial_back_off: Duration::from_secs(1),
                max_back_off: Duration::from_secs(4),
                back_off_multiplier: multiplier,
                ..policy(4)
            };

            assert_eq!(
                policy.bounds().collect::<Vec<_>>(),
                [1, 4, 4].map(Duration::from_secs),
                "{multiplier} should be held to the ceiling"
            );
        }
    }
}
