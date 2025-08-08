package fr.aneo.armonik.client;

import io.grpc.netty.shaded.io.netty.handler.ssl.SslContextBuilder;

import javax.net.ssl.KeyManagerFactory;
import java.io.FileInputStream;
import java.security.KeyStore;

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
 * gRPC client certificate implementation for PKCS#12 format certificates.
 */
public class Pkcs12ClientCertificate implements GrpcClientCertificate {

  private final String pkcs12Path;
  private final char[] password;

  /**
   * Creates a PKCS#12 gRPC client certificate with password.
   *
   * @param pkcs12Path Path to the PKCS#12 certificate file
   * @param password Password for the certificate (can be null for passwordless)
   */
  public Pkcs12ClientCertificate(String pkcs12Path, String password) {
    if (pkcs12Path == null || pkcs12Path.isBlank()) {
      throw new IllegalArgumentException("Certificate path cannot be null or blank");
    }

    this.pkcs12Path = pkcs12Path;
    this.password = password != null ? password.toCharArray() : new char[0];
  }

  /**
   * Creates a passwordless PKCS#12 gRPC client certificate.
   *
   * @param pkcs12Path Path to the PKCS#12 certificate file
   */
  public Pkcs12ClientCertificate(String pkcs12Path) {
    this(pkcs12Path, null);
  }

  @Override
  public void configureKeyManager(SslContextBuilder sslContextBuilder) throws Exception {
    KeyManagerFactory keyManagerFactory = createKeyManagerFactory();
    sslContextBuilder.keyManager(keyManagerFactory);
  }

  private KeyManagerFactory createKeyManagerFactory() throws Exception {
    KeyStore keyStore = KeyStore.getInstance("PKCS12");
    try (FileInputStream fis = new FileInputStream(pkcs12Path)) {
      keyStore.load(fis, password);
    }

    KeyManagerFactory keyManagerFactory = KeyManagerFactory.getInstance(KeyManagerFactory.getDefaultAlgorithm());
    keyManagerFactory.init(keyStore, password);
    return keyManagerFactory;
  }

  @Override
  public String getDescription() {
    boolean hasPassword = password != null && password.length > 0;
    return String.format("PKCS#12 certificate (path: %s, password: %s)",
      pkcs12Path, hasPassword ? "******" : "none");
  }

  /**
   * Static factory method for creating passwordless PKCS#12 gRPC client certificates.
   *
   * @param pkcs12Path Path to the PKCS#12 certificate file
   * @return New Pkcs12ClientCertificate instance
   */
  public static Pkcs12ClientCertificate of(String pkcs12Path) {
    return new Pkcs12ClientCertificate(pkcs12Path);
  }

  /**
   * Static factory method for creating password-protected PKCS#12 gRPC client certificates.
   *
   * @param pkcs12Path Path to the PKCS#12 certificate file
   * @param password Password for the certificate
   * @return New Pkcs12ClientCertificate instance
   */
  public static Pkcs12ClientCertificate of(String pkcs12Path, String password) {
    return new Pkcs12ClientCertificate(pkcs12Path, password);
  }
}
