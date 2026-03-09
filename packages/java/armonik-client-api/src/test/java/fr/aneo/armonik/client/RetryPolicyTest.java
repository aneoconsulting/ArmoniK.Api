package fr.aneo.armonik.client;

import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Test;

import java.time.Duration;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

class RetryPolicyTest {

  @Test
  @DisplayName("defaults should return policy with C# SDK compatible values")
  void defaults_should_return_policy_with_csharp_sdk_compatible_values() {
    // when
    var policy = RetryPolicy.DEFAULT;

    // then
    assertThat(policy.maxAttempts()).isEqualTo(5);
    assertThat(policy.initialBackoff()).isEqualTo(Duration.ofSeconds(1));
    assertThat(policy.maxBackoff()).isEqualTo(Duration.ofSeconds(5));
    assertThat(policy.backoffMultiplier()).isEqualTo(1.5);
  }

  @Test
  @DisplayName("constructor should accept valid parameters")
  void constructor_should_accept_valid_parameters() {
    // when
    var policy = new RetryPolicy(
      3,
      Duration.ofMillis(500),
      Duration.ofSeconds(10),
      2.0
    );

    // then
    assertThat(policy.maxAttempts()).isEqualTo(3);
    assertThat(policy.initialBackoff()).isEqualTo(Duration.ofMillis(500));
    assertThat(policy.maxBackoff()).isEqualTo(Duration.ofSeconds(10));
    assertThat(policy.backoffMultiplier()).isEqualTo(2.0);
  }

  @Test
  @DisplayName("constructor should throw exception when max attempts is less than 1")
  void constructor_should_throw_exception_when_max_attempts_is_less_than_1() {
    assertThatThrownBy(() -> new RetryPolicy(0, Duration.ofSeconds(1), Duration.ofSeconds(5), 1.5))
      .isInstanceOf(IllegalArgumentException.class);
  }

  @Test
  @DisplayName("constructor should throw exception when initial backoff is invalid")
  void constructor_should_throw_exception_when_initial_backoff_is_invalid() {
    assertThatThrownBy(() -> new RetryPolicy(5, Duration.ZERO, Duration.ofSeconds(5), 1.5))
      .isInstanceOf(IllegalArgumentException.class);
  }

  @Test
  @DisplayName("constructor should throw exception when max backoff is less than initial backoff")
  void constructor_should_throw_exception_when_max_backoff_is_less_than_initial_backoff() {
    assertThatThrownBy(() -> new RetryPolicy(5, Duration.ofSeconds(5), Duration.ofSeconds(1), 1.5))
      .isInstanceOf(IllegalArgumentException.class);
  }

  @Test
  @DisplayName("constructor should throw exception when backoff multiplier is not positive")
  void constructor_should_throw_exception_when_backoff_multiplier_is_not_positive() {
    assertThatThrownBy(() -> new RetryPolicy(5, Duration.ofSeconds(1), Duration.ofSeconds(5), 0.0))
      .isInstanceOf(IllegalArgumentException.class);
  }
}
