package fr.aneo.armonik.client;

import io.grpc.ManagedChannel;
import io.grpc.netty.shaded.io.grpc.netty.GrpcSslContexts;
import io.grpc.netty.shaded.io.grpc.netty.NettyChannelBuilder;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.io.File;
import java.net.URI;

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
 * Fluent builder for creating gRPC ManagedChannel instances with customizable SSL/TLS configuration.
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
 *
 * <h3>Usage Examples</h3>
 *
 * <h4>Development/Testing (Insecure)</h4>
 * <pre>{@code
 * ManagedChannel channel = GrpcChannelBuilder
 *     .forEndpoint("http://localhost:4312")
 *     .withUnsecureConnection()
 *     .build();
 * }</pre>
 *
 * <h4> TLS with System Trust Store</h4>
 * <pre>{@code
 * ManagedChannel channel = GrpcChannelBuilder
 *     .forEndpoint("https://api.example.com:443")
 *     .build();
 * }</pre>
 *
 * <h4>TLS with Custom CA Certificate</h4>
 * <pre>{@code
 * ManagedChannel channel = GrpcChannelBuilder
 *     .forEndpoint("https://api.example.com:443")
 *     .withCaPem("/path/to/ca.pem")
 *     .build();
 * }</pre>
 *
 * <h4>Mutual TLS with PEM Certificate</h4>
 * <pre>{@code
 * ManagedChannel channel = GrpcChannelBuilder
 *     .forEndpoint("https://api.example.com:443")
 *     .withCaPem("/path/to/ca.pem") // Optional
 *     .withClientCertificate(PemGrpcClientCertificate.of(
 *         "/path/to/client.pem",
 *         "/path/to/client.key"
 *     ))
 *     .build();
 * }</pre>
 *
 * <h4>Mutual TLS with PKCS#12 Certificate</h4>
 * <pre>{@code
 * ManagedChannel channel = GrpcChannelBuilder
 *     .forEndpoint("https://api.example.com:443")
 *     .withCaPem("/path/to/ca.pem") // Optional
 *     .withClientCertificate(Pkcs12GrpcClientCertificate.of(
 *         "/path/to/client.p12",
 *         "password"
 *     ))
 *     .build();
 * }</pre>
 */
public class GrpcChannelBuilder {

  private static final Logger logger = LoggerFactory.getLogger(GrpcChannelBuilder.class);

  private final URI endpointUri;
  private String caPem = null;
  private GrpcClientCertificate clientCertificate = null;
  private boolean useUnsecureConnection = false;

  /**
   * @param endpointUri The parsed gRPC server endpoint URI
   */
  private GrpcChannelBuilder(URI endpointUri) {
    this.endpointUri = endpointUri;
  }

  /**
   * Creates a new builder for the specified endpoint.
   *
   * @param endpoint The gRPC server endpoint (e.g., "<a href="https://localhost:4312">https://localhost:4312</a>" or "<a href="http://localhost:4312">http://localhost:4312</a>")
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
   * Builds the ManagedChannel with the configured settings.
   *
   * @return A new {@link ManagedChannel} instance
   * @throws RuntimeException      if SSL context setup fails
   * @throws IllegalStateException if the configuration is invalid (conflicting insecure/certificate settings)
   */
  public ManagedChannel build() {
    NettyChannelBuilder builder = NettyChannelBuilder.forAddress(endpointUri.getHost(), endpointUri.getPort());

    if (useUnsecureConnection) {
      validateUnsecureConnectionConfiguration();
      logger.warn(
        "SECURITY WARNING: Insecure plaintext connection enabled for endpoint {}. " +
          "This disables all encryption and should only be used for development or internal networks.", endpointUri
      );

      builder.usePlaintext();
    } else {
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
    return builder.build();
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
