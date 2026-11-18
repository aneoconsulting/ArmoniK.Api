package fr.aneo.armonik.client;

import fr.aneo.armonik.api.grpc.v1.auth.AuthCommon;
import fr.aneo.armonik.api.grpc.v1.auth.AuthenticationGrpc;
import io.grpc.ManagedChannel;
import io.grpc.Server;
import io.grpc.Status;
import io.grpc.StatusRuntimeException;
import io.grpc.netty.shaded.io.grpc.netty.GrpcSslContexts;
import io.grpc.netty.shaded.io.grpc.netty.NettyServerBuilder;
import io.grpc.netty.shaded.io.netty.handler.ssl.ClientAuth;
import io.grpc.stub.StreamObserver;
import org.bouncycastle.asn1.x500.X500Name;
import org.bouncycastle.asn1.x509.Extension;
import org.bouncycastle.asn1.x509.GeneralName;
import org.bouncycastle.asn1.x509.GeneralNames;
import org.bouncycastle.cert.jcajce.JcaX509CertificateConverter;
import org.bouncycastle.cert.jcajce.JcaX509v3CertificateBuilder;
import org.bouncycastle.openssl.jcajce.JcaPEMWriter;
import org.bouncycastle.operator.jcajce.JcaContentSignerBuilder;
import org.bouncycastle.util.io.pem.PemObject;
import org.bouncycastle.util.io.pem.PemWriter;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.io.FileWriter;
import java.math.BigInteger;
import java.nio.file.Path;
import java.security.KeyPair;
import java.security.KeyPairGenerator;
import java.security.PrivateKey;
import java.security.cert.X509Certificate;
import java.time.Duration;
import java.time.Instant;
import java.util.Date;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

/**
 * Connects to TLS servers whose certificate is self-signed and issued for another host name, so that
 * it passes neither chain nor host name validation. No container is needed: the servers run in the test.
 */
class GrpcChannelServerCertificateValidationTest {

  @TempDir
  static Path certificates;
  private static Server tlsServer;
  private static Server mtlsServer;

  @BeforeAll
  static void startServers() throws Exception {
    var serverKeys = keyPair();
    var serverCert = selfSigned(serverKeys, "untrusted.example");
    var serverCertFile = write(certificates.resolve("server.cert"), serverCert);
    var serverKeyFile = writeKey(certificates.resolve("server.key"), serverKeys.getPrivate());
    var clientKeys = keyPair();
    var clientCertFile = write(certificates.resolve("client.cert"), selfSigned(clientKeys, "client.example"));
    writeKey(certificates.resolve("client.key"), clientKeys.getPrivate());

    tlsServer = NettyServerBuilder.forPort(0)
                                  .sslContext(GrpcSslContexts.forServer(serverCertFile.toFile(), serverKeyFile.toFile()).build())
                                  .addService(new Authentication())
                                  .build()
                                  .start();
    mtlsServer = NettyServerBuilder.forPort(0)
                                   .sslContext(GrpcSslContexts.forServer(serverCertFile.toFile(), serverKeyFile.toFile())
                                                              .trustManager(clientCertFile.toFile())
                                                              .clientAuth(ClientAuth.REQUIRE)
                                                              .build())
                                   .addService(new Authentication())
                                   .build()
                                   .start();
  }

  @AfterAll
  static void stopServers() {
    if (tlsServer != null) tlsServer.shutdownNow();
    if (mtlsServer != null) mtlsServer.shutdownNow();
  }

  @Test
  void should_connect_over_tls_without_validating_the_server_certificate() {
    var channel = GrpcChannelBuilder.forEndpoint("https://localhost:" + tlsServer.getPort())
                                    .withoutServerCertificateValidation()
                                    .build();
    try {
      assertThat(getUser(channel).getUsername()).isEqualTo("admin");
    } finally {
      channel.shutdownNow();
    }
  }

  @Test
  void should_refuse_the_same_server_certificate_when_validation_is_kept() {
    var channel = GrpcChannelBuilder.forEndpoint("https://localhost:" + tlsServer.getPort()).build();
    try {
      assertThatThrownBy(() -> getUser(channel))
        .isInstanceOf(StatusRuntimeException.class)
        .extracting(error -> ((StatusRuntimeException) error).getStatus().getCode())
        .isEqualTo(Status.Code.UNAVAILABLE);
    } finally {
      channel.shutdownNow();
    }
  }

  @Test
  void should_still_send_the_client_certificate_for_mutual_tls() {
    var channel = GrpcChannelBuilder.forEndpoint("https://localhost:" + mtlsServer.getPort())
                                    .withoutServerCertificateValidation()
                                    .withClientCertificate(PemClientCertificate.of(
                                      certificates.resolve("client.cert").toString(),
                                      certificates.resolve("client.key").toString()))
                                    .build();
    try {
      assertThat(getUser(channel).getUsername()).isEqualTo("admin");
    } finally {
      channel.shutdownNow();
    }
  }

  @Test
  void should_throw_exception_when_server_certificate_validation_is_disabled_on_an_unsecure_connection() {
    assertThatThrownBy(() ->
      GrpcChannelBuilder.forEndpoint("http://localhost:4312")
                        .withUnsecureConnection()
                        .withoutServerCertificateValidation()
                        .build()
    ).isInstanceOf(IllegalStateException.class);
  }

  @Test
  void should_throw_exception_when_server_certificate_validation_is_disabled_with_a_ca_certificate() {
    assertThatThrownBy(() ->
      GrpcChannelBuilder.forEndpoint("https://localhost:4312")
                        .withCaPem(certificates.resolve("server.cert").toString())
                        .withoutServerCertificateValidation()
                        .build()
    ).isInstanceOf(IllegalStateException.class)
     .hasMessageContaining("server.cert");
  }

  private static AuthCommon.User getUser(ManagedChannel channel) {
    return AuthenticationGrpc.newBlockingStub(channel)
                             .withDeadlineAfter(Duration.ofSeconds(10))
                             .getCurrentUser(AuthCommon.GetCurrentUserRequest.getDefaultInstance())
                             .getUser();
  }

  private static KeyPair keyPair() throws Exception {
    var generator = KeyPairGenerator.getInstance("RSA");
    generator.initialize(2048);
    return generator.generateKeyPair();
  }

  private static X509Certificate selfSigned(KeyPair keys, String hostName) throws Exception {
    var name = new X500Name("CN=" + hostName);
    var now = Instant.now();
    var builder = new JcaX509v3CertificateBuilder(name, BigInteger.valueOf(now.toEpochMilli()), Date.from(now),
      Date.from(now.plus(Duration.ofDays(1))), name, keys.getPublic());
    builder.addExtension(Extension.subjectAlternativeName, false,
      new GeneralNames(new GeneralName(GeneralName.dNSName, hostName)));
    var signer = new JcaContentSignerBuilder("SHA256withRSA").build(keys.getPrivate());
    return new JcaX509CertificateConverter().getCertificate(builder.build(signer));
  }

  private static Path write(Path file, X509Certificate certificate) throws Exception {
    try (var writer = new JcaPEMWriter(new FileWriter(file.toFile()))) {
      writer.writeObject(certificate);
    }
    return file;
  }

  /** Writes the key as PKCS#8, the PEM format Netty reads. */
  private static Path writeKey(Path file, PrivateKey key) throws Exception {
    try (var writer = new PemWriter(new FileWriter(file.toFile()))) {
      writer.writeObject(new PemObject("PRIVATE KEY", key.getEncoded()));
    }
    return file;
  }

  private static final class Authentication extends AuthenticationGrpc.AuthenticationImplBase {
    @Override
    public void getCurrentUser(AuthCommon.GetCurrentUserRequest request, StreamObserver<AuthCommon.GetCurrentUserResponse> responseObserver) {
      responseObserver.onNext(AuthCommon.GetCurrentUserResponse.newBuilder()
                                                               .setUser(AuthCommon.User.newBuilder().setUsername("admin"))
                                                               .build());
      responseObserver.onCompleted();
    }
  }
}
