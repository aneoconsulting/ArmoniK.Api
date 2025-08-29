package fr.aneo.armonik.client;

import io.grpc.netty.shaded.io.netty.handler.ssl.SslContextBuilder;

import java.io.File;

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
 * gRPC client certificate implementation for PEM format certificates.
 * Requires both certificate and private key files.
 */
public class PemClientCertificate implements GrpcClientCertificate {

  private final String certPath;
  private final String keyPath;

  /**
   * Creates a PEM gRPC client certificate.
   *
   * @param certPath Path to the client certificate PEM file
   * @param keyPath  Path to the client private key PEM file
   * @throws IllegalArgumentException if either path is null or blank
   */
  public PemClientCertificate(String certPath, String keyPath) {
    if (certPath == null || certPath.isBlank()) {
      throw new IllegalArgumentException("Certificate path cannot be null or blank");
    }
    if (keyPath == null || keyPath.isBlank()) {
      throw new IllegalArgumentException("Key path cannot be null or blank");
    }

    this.certPath = certPath;
    this.keyPath = keyPath;
  }

  @Override
  public void configureKeyManager(SslContextBuilder sslContextBuilder) {
    sslContextBuilder.keyManager(new File(certPath), new File(keyPath));
  }

  @Override
  public String getDescription() {
    return String.format("PEM certificate (cert: %s, key: %s)", certPath, keyPath);
  }

  /**
   * Static factory method for creating PEM gRPC client certificates.
   *
   * @param certPath Path to the client certificate PEM file
   * @param keyPath  Path to the client private key PEM file
   * @return New PemGrpcClientCertificate instance
   * @throws IllegalArgumentException if either path is null or blank
   */
  public static PemClientCertificate of(String certPath, String keyPath) {
    return new PemClientCertificate(certPath, keyPath);
  }
}
