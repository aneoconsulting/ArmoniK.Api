package fr.aneo.armonik.client;

import io.grpc.ManagedChannel;
import io.grpc.netty.shaded.io.grpc.netty.GrpcSslContexts;
import io.grpc.netty.shaded.io.grpc.netty.NettyChannelBuilder;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.io.File;
import java.net.URI;
import java.time.Duration;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.concurrent.TimeUnit;

/*
 * This file is part of the ArmoniK project
 *
 * Copyright (C) ANEO, 2025-2025. All rights reserved.
 *   C. Amory          <camory@ext.aneo.fr>
 *
 * Licensed under the Apache License, Version 2.0 (the "License")
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *         http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

/**
 * Fluent builder for creating gRPC ManagedChannel instances with customizable SSL/TLS configuration,
 * retry policies, and connection management.
 *
 * <h3>Certificate Format Support</h3>
 * <ul>
 *   <li><strong>PEM Format</strong>: Use {@link PemClientCertificate} for separate certificate and key files</li>
 *   <li><strong>PKCS#12 Format</strong>: Use {@link Pkcs12ClientCertificate} for bundled certificate/key files (.p12/.pfx)</li>
 *   <li><strong>Custom Formats</strong>: Implement {@link GrpcClientCertificate} interface for other certificate formats</li>
 * </ul>
 *
 * <h3>Configuration Validation</h3>
 * <ul>
 *   <li><strong>Insecure Connection Conflict</strong>: Throws {@link IllegalStateException} if {@code withUnsecureConnection()}
 *       is used together with any certificate configuration ({@code withCaPem()} or {@code withClientCertificate()})</li>
 * </ul>
 */
public class GrpcChannelBuilder {

  private static final Logger logger = LoggerFactory.getLogger(GrpcChannelBuilder.class);

  private static final int MAX_INBOUND_METADATA_SIZE = 1024 * 1024;  // 1 MB
  private static final int MAX_INBOUND_MESSAGE_SIZE = 8 * 1024 * 1024;  // 8 MB
  private static final List<String> RETRYABLE_STATUS_CODES = List.of(
    "UNAVAILABLE",
    "DEADLINE_EXCEEDED",
    "RESOURCE_EXHAUSTED",
    "ABORTED"
  );

  private final URI endpointUri;
  private String caPem = null;
  private GrpcClientCertificate clientCertificate = null;
  private boolean useUnsecureConnection = false;
  private RetryPolicy retryPolicy = null;
  private Duration keepAliveTime = null;
  private Duration keepAliveTimeout = null;
  private Duration idleTimeout = null;

  /**
   * @param endpointUri The parsed gRPC server endpoint URI
   */
  private GrpcChannelBuilder(URI endpointUri) {
    this.endpointUri = endpointUri;
  }

  /**
   * Creates a new builder for the specified endpoint.
   *
   * @param endpoint The gRPC server endpoint (e.g., "https://localhost:4312" or "http://localhost:4312")
   * @return A new {@link GrpcChannelBuilder} instance
   * @throws IllegalArgumentException if endpoint is null, blank, or has invalid format
   */
  public static GrpcChannelBuilder forEndpoint(String endpoint) {
    if (endpoint == null || endpoint.isBlank()) {
      throw new IllegalArgumentException("Endpoint must be provided and cannot be blank");
    }

    URI uri;
    try {
      uri = URI.create(endpoint);
    } catch (IllegalArgumentException e) {
      throw new IllegalArgumentException("Invalid endpoint format: " + endpoint, e);
    }

    if (uri.getHost() == null) {
      throw new IllegalArgumentException("Endpoint must contain a valid host: " + endpoint);
    }
    if (uri.getPort() == -1) {
      throw new IllegalArgumentException("Endpoint must contain a valid port: " + endpoint);
    }

    return new GrpcChannelBuilder(uri);
  }

  /**
   * Enables insecure plaintext connections (no SSL/TLS).
   * This must be explicitly called for insecure connections.
   *
   * <p><strong>Security Warning:</strong> This disables all encryption and should only be used
   * for development or internal networks.</p>
   *
   * <p><strong>Validation:</strong> Cannot be combined with any certificate configuration
   * ({@link #withCaPem(String)}, {@link #withClientCertificate(GrpcClientCertificate)}).
   * Throws {@link IllegalStateException} if certificates are also provided.</p>
   *
   * @return this builder instance
   * @throws IllegalStateException during {@link #build()} if certificates are also provided
   */
  public GrpcChannelBuilder withUnsecureConnection() {
    this.useUnsecureConnection = true;
    return this;
  }

  /**
   * Sets the CA certificate file path for server verification.
   *
   * @param caPemPath Path to the CA certificate PEM file, or null to use system trust store
   * @return this builder instance
   * @throws IllegalStateException during {@link #build()} if {@link #withUnsecureConnection()} was also called
   */
  public GrpcChannelBuilder withCaPem(String caPemPath) {
    this.caPem = caPemPath;
    return this;
  }

  /**
   * Sets the client certificate for mutual TLS authentication.
   *
   * @param clientCertificate Client certificate implementation (PEM, PKCS#12, etc.)
   * @return this builder instance
   */
  public GrpcChannelBuilder withClientCertificate(GrpcClientCertificate clientCertificate) {
    this.clientCertificate = clientCertificate;
    return this;
  }

  /**
   * Configures automatic retry behavior for transient failures.
   *
   * <p>When configured, the channel will automatically retry failed requests that match
   * retryable status codes (UNAVAILABLE, DEADLINE_EXCEEDED, RESOURCE_EXHAUSTED, ABORTED)
   * using exponential backoff.
   *
   * <p>By default, retry is disabled. You must explicitly call this method to enable retry.
   *
   * @param retryPolicy Retry configuration (use {@link RetryPolicy#DEFAULT} for recommended settings)
   * @return this builder instance
   * @see RetryPolicy
   */
  public GrpcChannelBuilder withRetry(RetryPolicy retryPolicy) {
    this.retryPolicy = retryPolicy;
    return this;
  }

  /**
   * Configures HTTP/2 keep-alive with default timeout (20 seconds).
   *
   * <p>Keep-alive sends periodic PING frames to detect broken connections early
   * and prevent idle connection closure by intermediate proxies or firewalls.
   *
   * <p>The default timeout of 20 seconds matches gRPC's internal default.
   *
   * <p>Recommended for long-lived connections or environments with aggressive connection timeouts.
   *
   * @param keepAliveTime Time between keep-alive PING frames when connection is idle
   * @return this builder instance
   * @throws IllegalArgumentException if keepAliveTime is null, zero, or negative
   * @see #withKeepAlive(Duration, Duration) for custom timeout configuration
   */
  public GrpcChannelBuilder withKeepAlive(Duration keepAliveTime) {
    return withKeepAlive(keepAliveTime, Duration.ofSeconds(20));
  }

  /**
   * Configures HTTP/2 keep-alive with custom timeout.
   *
   * <p>Keep-alive sends periodic PING frames to detect broken connections early
   * and prevent idle connection closure by intermediate proxies or firewalls.
   *
   * <p>Use this overload when you need fine-grained control over the timeout value.
   * For most cases, {@link #withKeepAlive(Duration)} with the default 20-second timeout is sufficient.
   *
   * @param keepAliveTime    Time between keep-alive PING frames when connection is idle
   * @param keepAliveTimeout Maximum time to wait for PING acknowledgement before considering connection dead
   * @return this builder instance
   * @throws IllegalArgumentException if either parameter is null, zero, or negative
   */
  public GrpcChannelBuilder withKeepAlive(Duration keepAliveTime, Duration keepAliveTimeout) {
    if (keepAliveTime == null || keepAliveTime.isZero() || keepAliveTime.isNegative()) {
      throw new IllegalArgumentException("keepAliveTime must be positive, got: " + keepAliveTime);
    }
    if (keepAliveTimeout == null || keepAliveTimeout.isZero() || keepAliveTimeout.isNegative()) {
      throw new IllegalArgumentException("keepAliveTimeout must be positive, got: " + keepAliveTimeout);
    }
    this.keepAliveTime = keepAliveTime;
    this.keepAliveTimeout = keepAliveTimeout;
    return this;
  }

  /**
   * Configures maximum idle time before closing inactive connections.
   *
   * <p>When a connection has no active streams for this duration, it will be automatically closed
   * to free resources. This is useful for applications with bursty traffic patterns.
   *
   * @param idleTimeout Maximum idle time before connection closure
   * @return this builder instance
   * @throws IllegalArgumentException if idleTimeout is null, zero, or negative
   */
  public GrpcChannelBuilder withIdleTimeout(Duration idleTimeout) {
    if (idleTimeout == null || idleTimeout.isZero() || idleTimeout.isNegative()) {
      throw new IllegalArgumentException("idleTimeout must be positive, got: " + idleTimeout);
    }
    this.idleTimeout = idleTimeout;
    return this;
  }

  /**
   * Builds the ManagedChannel with the configured settings.
   *
   * @return A new {@link ManagedChannel} instance
   * @throws RuntimeException      if SSL context setup fails
   * @throws IllegalStateException if the configuration is invalid (conflicting insecure/certificate settings)
   */
  public ManagedChannel build() {
    var builder = NettyChannelBuilder.forAddress(endpointUri.getHost(), endpointUri.getPort())
                                     .maxInboundMetadataSize(MAX_INBOUND_METADATA_SIZE)
                                     .maxInboundMessageSize(MAX_INBOUND_MESSAGE_SIZE);

    if (useUnsecureConnection) {
      validateUnsecureConnectionConfiguration();
      logger.warn( "SECURITY WARNING: Insecure plaintext connection enabled for endpoint {}. " +
          "This disables all encryption and should only be used for development or internal networks.", endpointUri);

      builder.usePlaintext();
    } else {
      configureSslContext(builder);
    }

    configureRetryPolicy(builder);
    configureKeepAlive(builder);
    configureIdleTimeout(builder);

    return builder.build();
  }

  /**
   * Configures SSL/TLS context for secure connections.
   */
  private void configureSslContext(NettyChannelBuilder builder) {
    try {
      var sslContextBuilder = GrpcSslContexts.forClient();
      if (caPem != null) {
        sslContextBuilder.trustManager(new File(caPem));
      }
      if (clientCertificate != null) {
        clientCertificate.configureKeyManager(sslContextBuilder);
        logger.info("Creating secure gRPC channel with mutual TLS to {} using {}", endpointUri, clientCertificate.getDescription());
      }
      builder.sslContext(sslContextBuilder.build());
    } catch (Exception e) {
      throw new RuntimeException("Failed to set up SSL context", e);
    }
  }

  /**
   * Configures retry policy using gRPC service config.
   */
  private void configureRetryPolicy(NettyChannelBuilder builder) {
    if (retryPolicy != null) {
      var serviceConfig = Map.of(
        "methodConfig", List.of(Map.of(
          "name", List.of(Map.of()),
          "retryPolicy", Map.of(
            "maxAttempts", (double) retryPolicy.maxAttempts(),
            "initialBackoff", formatDuration(retryPolicy.initialBackoff()),
            "maxBackoff", formatDuration(retryPolicy.maxBackoff()),
            "backoffMultiplier", retryPolicy.backoffMultiplier(),
            "retryableStatusCodes", RETRYABLE_STATUS_CODES
          )
        ))
      );

      builder.defaultServiceConfig(serviceConfig);
      builder.enableRetry();

      logger.debug("Retry policy configured: maxAttempts={}, initialBackoff={}, maxBackoff={}, multiplier={}",
        retryPolicy.maxAttempts(),
        retryPolicy.initialBackoff(),
        retryPolicy.maxBackoff(),
        retryPolicy.backoffMultiplier()
      );
    }
  }

  /**
   * Configures HTTP/2 keep-alive settings.
   */
  private void configureKeepAlive(NettyChannelBuilder builder) {
    if (keepAliveTime != null && keepAliveTimeout != null) {
      builder.keepAliveTime(keepAliveTime.toMillis(), TimeUnit.MILLISECONDS);
      builder.keepAliveTimeout(keepAliveTimeout.toMillis(), TimeUnit.MILLISECONDS);
      builder.keepAliveWithoutCalls(true);

      logger.debug("Keep-alive configured: time={}, timeout={}", keepAliveTime, keepAliveTimeout);
    }
  }

  /**
   * Configures idle connection timeout.
   */
  private void configureIdleTimeout(NettyChannelBuilder builder) {
    if (idleTimeout != null) {
      builder.idleTimeout(idleTimeout.toMillis(), TimeUnit.MILLISECONDS);
      logger.debug("Idle timeout configured: {}", idleTimeout);
    }
  }

  /**
   * Formats a Duration into gRPC service config format (e.g., "1.5s", "100ms").
   */
  private String formatDuration(Duration duration) {
    long nanos = duration.toNanos();
    if (nanos == 0) {
      return "0s";
    }
    double seconds = nanos / 1_000_000_000.0;
    if (seconds >= 1.0) {
      return String.format(Locale.ROOT, "%.9f", seconds)
                   .replaceAll("0+$", "")
                   .replaceAll("\\.$", "") + "s";
    } else {
      long millis = duration.toMillis();
      if (millis > 0) {
        return millis + "ms";
      }
      return nanos + "ns";
    }
  }

  /**
   * Validates that insecure connection is not combined with certificates.
   */
  private void validateUnsecureConnectionConfiguration() {
    if (useUnsecureConnection && (clientCertificate != null || caPem != null)) {
      throw new IllegalStateException(
        "Cannot use certificates with unsecure connection. " +
          "Either remove withUnsecureConnection() to use SSL/TLS with certificates, or remove certificate methods for insecure connection."
      );
    }
  }
}
