package fr.aneo.armonik.client;

import io.grpc.netty.shaded.io.netty.handler.ssl.SslContextBuilder;

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
 * Represents a client certificate for gRPC mutual TLS authentication.
 * Provides an abstraction over different certificate formats (PEM, PKCS#12, etc.).
 */
public interface GrpcClientCertificate {

  /**
   * Configures the given SSL context builder with this certificate.
   *
   * @param sslContextBuilder The SSL context builder to configure
   * @throws Exception if certificate loading fails
   */
  void configureKeyManager(SslContextBuilder sslContextBuilder) throws Exception;

  /**
   * Returns a description of this certificate for logging purposes.
   *
   * @return Human-readable description (without sensitive information)
   */
  String getDescription();
}
