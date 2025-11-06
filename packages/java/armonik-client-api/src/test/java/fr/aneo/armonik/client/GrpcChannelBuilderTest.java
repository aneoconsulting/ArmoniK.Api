package fr.aneo.armonik.client;

import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Test;

import java.time.Duration;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

class GrpcChannelBuilderTest {

  @Test
  @DisplayName("build should create channel with all features configured")
  void build_should_create_channel_with_all_features_configured() {
    // when
    var channel = GrpcChannelBuilder.forEndpoint("http://localhost:4312")
                                    .withUnsecureConnection()
                                    .withRetry(RetryPolicy.DEFAULT)
                                    .withKeepAlive(Duration.ofSeconds(30), Duration.ofSeconds(10))
                                    .withIdleTimeout(Duration.ofMinutes(5))
                                    .build();

    // then
    assertThat(channel).isNotNull();
    assertThat(channel.isShutdown()).isFalse();
    channel.shutdown();
  }

  @Test
  @DisplayName("build should create channel with minimal configuration")
  void build_should_create_channel_with_minimal_configuration() {
    // when
    var channel = GrpcChannelBuilder.forEndpoint("http://localhost:4312")
                                    .withUnsecureConnection()
                                    .build();

    // then
    assertThat(channel).isNotNull();
    assertThat(channel.isShutdown()).isFalse();
    channel.shutdown();
  }

  @Test
  @DisplayName("with keep alive should throw exception when keep alive time is null")
  void with_keep_alive_should_throw_exception_when_keep_alive_time_is_null() {
    assertThatThrownBy(() ->
      GrpcChannelBuilder.forEndpoint("https://localhost:4312")
                        .withKeepAlive(null)
    ).isInstanceOf(IllegalArgumentException.class);
  }

  @Test
  @DisplayName("with keep alive should throw exception when keep alive time is zero")
  void with_keep_alive_should_throw_exception_when_keep_alive_time_is_zero() {
    assertThatThrownBy(() ->
      GrpcChannelBuilder.forEndpoint("https://localhost:4312")
                        .withKeepAlive(Duration.ZERO)
    ).isInstanceOf(IllegalArgumentException.class);
  }

  @Test
  @DisplayName("with keep alive should throw exception when keep alive time is negative")
  void with_keep_alive_should_throw_exception_when_keep_alive_time_is_negative() {
    assertThatThrownBy(() ->
      GrpcChannelBuilder.forEndpoint("https://localhost:4312")
                        .withKeepAlive(Duration.ofSeconds(-1))
    ).isInstanceOf(IllegalArgumentException.class);
  }

  @Test
  @DisplayName("with keep alive should throw exception when keep alive timeout is zero")
  void with_keep_alive_should_throw_exception_when_keep_alive_timeout_is_zero() {
    assertThatThrownBy(() ->
      GrpcChannelBuilder.forEndpoint("https://localhost:4312")
                        .withKeepAlive(Duration.ofSeconds(30), Duration.ZERO)
    ).isInstanceOf(IllegalArgumentException.class);
  }

  @Test
  @DisplayName("with keep alive should throw exception when keep alive timeout is negative")
  void with_keep_alive_should_throw_exception_when_keep_alive_timeout_is_negative() {
    assertThatThrownBy(() ->
      GrpcChannelBuilder.forEndpoint("https://localhost:4312")
                        .withKeepAlive(Duration.ofSeconds(30), Duration.ofSeconds(-1))
    ).isInstanceOf(IllegalArgumentException.class);
  }

  @Test
  @DisplayName("with idle timeout should throw exception when idle timeout is zero")
  void with_idle_timeout_should_throw_exception_when_idle_timeout_is_zero() {
    assertThatThrownBy(() ->
      GrpcChannelBuilder.forEndpoint("https://localhost:4312")
                        .withIdleTimeout(Duration.ZERO)
    ).isInstanceOf(IllegalArgumentException.class);
  }

  @Test
  @DisplayName("with idle timeout should throw exception when idle timeout is negative")
  void with_idle_timeout_should_throw_exception_when_idle_timeout_is_negative() {
    assertThatThrownBy(() ->
      GrpcChannelBuilder.forEndpoint("https://localhost:4312")
                        .withIdleTimeout(Duration.ofSeconds(-1))
    ).isInstanceOf(IllegalArgumentException.class);
  }

  @Test
  @DisplayName("build should throw exception when unsecure connection is combined with CA certificate")
  void build_should_throw_exception_when_unsecure_connection_is_combined_with_ca_certificate() {
    assertThatThrownBy(() ->
      GrpcChannelBuilder.forEndpoint("http://localhost:4312")
                        .withUnsecureConnection()
                        .withCaPem("/path/to/ca.pem")
                        .build()
    ).isInstanceOf(IllegalStateException.class);
  }

  @Test
  @DisplayName("build should throw exception when unsecure connection is combined with client certificate")
  void build_should_throw_exception_when_unsecure_connection_is_combined_with_client_certificate() {
    assertThatThrownBy(() ->
      GrpcChannelBuilder.forEndpoint("http://localhost:4312")
                        .withUnsecureConnection()
                        .withClientCertificate(PemClientCertificate.of("/cert.pem", "/key.pem"))
                        .build()
    ).isInstanceOf(IllegalStateException.class);
  }
}
