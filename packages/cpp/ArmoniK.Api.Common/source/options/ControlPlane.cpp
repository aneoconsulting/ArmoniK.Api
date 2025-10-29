#include "options/ControlPlane.h"
#include "utils/Configuration.h"
#include "utils/Utils.h"

armonik::api::common::options::ControlPlane::ControlPlane(const utils::Configuration &config) {
  endpoint_ = config.get(EndpointKey);
  user_cert_pem_path_ = config.get(UserCertKey);
  user_key_pem_path_ = config.get(UserKeyKey);
  user_p12_path_ = config.get(UserP12Key);
  ca_cert_pem_path_ = config.get(CaCertKey);
  sslValidation_ = config.get(AllowUnsafeConnectionKey) != "true";

  keep_alive_time_ = config.get(KeepAliveTimeKey).empty() ? utils::duration_from_values(0, 0, 0, 30)
                                                          : utils::duration_from_timespan(config.get(KeepAliveTimeKey));
  keep_alive_time_interval_ = config.get(KeepAliveTimeIntervalKey).empty()
                                  ? utils::duration_from_values(0, 0, 0, 30)
                                  : utils::duration_from_timespan(config.get(KeepAliveTimeIntervalKey));
  max_idle_time_ = config.get(MaxIdleTimeKey).empty() ? utils::duration_from_values(0, 0, 5)
                                                      : utils::duration_from_timespan(config.get(MaxIdleTimeKey));
  long attempts = std::strtol(config.get(MaxAttemptsKey).c_str(), nullptr, 10);
  max_attempts_ = (attempts <= 0 || attempts >= INT_MAX) ? 5 : (int)attempts;
  backoff_multiplier_ = strtod(config.get(BackoffMultiplierKey).c_str(), nullptr);
  backoff_multiplier_ = backoff_multiplier_ == 0 || backoff_multiplier_ == HUGE_VAL ? 1.5 : backoff_multiplier_;
  initial_backoff_ = config.get(InitialBackOffKey).empty()
                         ? utils::duration_from_values(0, 0, 0, 1)
                         : utils::duration_from_timespan(config.get(InitialBackOffKey));
  max_backoff_ = config.get(MaxBackOffKey).empty() ? utils::duration_from_values(0, 0, 0, 5)
                                                   : utils::duration_from_timespan(config.get(MaxBackOffKey));
  request_timeout_ = config.get(RequestTimeoutKey).empty()
                         ? utils::duration_from_values(366)
                         : utils::duration_from_timespan(config.get(RequestTimeoutKey));

  partition_id_ = config.get(PartitionIdKey);
}
