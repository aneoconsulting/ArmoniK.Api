package fr.aneo.armonik.client;

import fr.aneo.armonik.api.grpc.v1.auth.AuthCommon;
import fr.aneo.armonik.api.grpc.v1.auth.AuthenticationGrpc;
import io.grpc.ManagedChannel;
import org.bouncycastle.asn1.x500.X500Name;
import org.bouncycastle.asn1.x509.BasicConstraints;
import org.bouncycastle.asn1.x509.Extension;
import org.bouncycastle.asn1.x509.GeneralName;
import org.bouncycastle.asn1.x509.GeneralNames;
import org.bouncycastle.cert.X509CertificateHolder;
import org.bouncycastle.cert.jcajce.JcaX509CertificateConverter;
import org.bouncycastle.cert.jcajce.JcaX509v3CertificateBuilder;
import org.bouncycastle.jce.provider.BouncyCastleProvider;
import org.bouncycastle.openssl.jcajce.JcaPEMWriter;
import org.bouncycastle.operator.jcajce.JcaContentSignerBuilder;
import org.bouncycastle.util.io.pem.PemObject;
import org.bouncycastle.util.io.pem.PemWriter;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;
import org.testcontainers.containers.GenericContainer;

import java.io.FileOutputStream;
import java.io.FileWriter;
import java.math.BigInteger;
import java.nio.file.Paths;
import java.security.*;
import java.security.cert.Certificate;
import java.security.cert.X509Certificate;
import java.util.Date;

import static java.lang.System.currentTimeMillis;
import static java.nio.file.Files.createDirectories;
import static java.time.LocalDateTime.now;
import static java.time.ZoneOffset.UTC;
import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;
import static org.bouncycastle.asn1.x509.Extension.basicConstraints;
import static org.testcontainers.shaded.org.apache.commons.io.FileUtils.deleteDirectory;
import static org.testcontainers.utility.MountableFile.forHostPath;

class GrpcChannelConnectionTest {

  @SuppressWarnings("resource")
  private static final GenericContainer<?> grpcServer = new GenericContainer<>("shubhendumadhukar/camouflage:latest")
    .withExposedPorts(4312)
    .withCopyFileToContainer(forHostPath(Paths.get("src/test/resources/camouflage/grpc/mocks").toAbsolutePath()), "/app/grpc/mocks")
    .withCopyFileToContainer(forHostPath(Paths.get("../../../Protos").toAbsolutePath()), "/app/grpc/protos");

  @BeforeAll
  static void generateCertificates() throws Exception {
    Security.addProvider(new BouncyCastleProvider());
    createDirectories(Paths.get("src/test/resources/camouflage/certs/server"));
    createDirectories(Paths.get("src/test/resources/camouflage/certs/client"));
    var caKeyPair = generateKeyPair();
    var caCert = generateCaCert(caKeyPair);
    generateServerCert(caKeyPair, caCert);
    generateClientCert(caKeyPair, caCert);
  }

  @AfterAll
  static void deleteCertificates() throws Exception {
    deleteDirectory(Paths.get("src/test/resources/camouflage/certs/").toFile());
  }


  @Test
  void connect_to_grpc_server_with_tls_mode() {
    // given
    grpcServer.withCopyFileToContainer(
                forHostPath(Paths.get("src/test/resources/camouflage/tls_config.yml").toAbsolutePath()), "/app/config.yml")
              .withCopyFileToContainer(
                forHostPath(Paths.get("src/test/resources/camouflage/certs/server").toAbsolutePath()), "/app/certs")
              .start();
    int mappedPort = grpcServer.getMappedPort(4312);

    var channel = GrpcChannelBuilder.forEndpoint("https://localhost:" + mappedPort)
                                    .withCaPem("src/test/resources/camouflage/certs/server/ca.cert")
                                    .build();

    // when
    var user = getUser(channel);

    // then
    assertThat(user.getUsername()).isEqualTo("admin");
    channel.shutdown();
    grpcServer.close();
  }

  @Test
  void connect_to_grpc_server_with_mtls_mode() {
    // given
    grpcServer.withCopyFileToContainer(
                forHostPath(Paths.get("src/test/resources/camouflage/mtls_config.yml").toAbsolutePath()), "/app/config.yml")
              .withCopyFileToContainer(
                forHostPath(Paths.get("src/test/resources/camouflage/certs/server").toAbsolutePath()), "/app/certs")
              .start();
    int mappedPort = grpcServer.getMappedPort(4312);

    var channel = GrpcChannelBuilder.forEndpoint("https://localhost:" + mappedPort)
                                    .withCaPem("src/test/resources/camouflage/certs/server/ca.cert")
                                    .withClientCertificate(PemClientCertificate.of("src/test/resources/camouflage/certs/client/client.cert", "src/test/resources/camouflage/certs/client/client.key"))
                                    .build();
    // when
    var user = getUser(channel);

    // then
    assertThat(user.getUsername()).isEqualTo("admin");
    channel.shutdown();
    grpcServer.close();
  }

  @Test
  void connect_to_grpc_server_with_pkcs12_certificate() {
    // given
    grpcServer.withCopyFileToContainer(
                forHostPath(Paths.get("src/test/resources/camouflage/mtls_config.yml").toAbsolutePath()), "/app/config.yml")
              .withCopyFileToContainer(
                forHostPath(Paths.get("src/test/resources/camouflage/certs/server").toAbsolutePath()), "/app/certs")
              .start();
    int mappedPort = grpcServer.getMappedPort(4312);

    var channel = GrpcChannelBuilder.forEndpoint("https://localhost:" + mappedPort)
                                    .withCaPem("src/test/resources/camouflage/certs/server/ca.cert")
                                    .withClientCertificate(Pkcs12ClientCertificate.of("src/test/resources/camouflage/certs/client/client.p12"))
                                    .build();

    // when
    var user = getUser(channel);

    // then
    assertThat(user.getUsername()).isEqualTo("admin");
    channel.shutdown();
    grpcServer.close();
  }


  @Test
  void connect_to_grpc_server_with_unsecure_connection() {
    // given
    grpcServer.withCopyFileToContainer(
                forHostPath(Paths.get("src/test/resources/camouflage/unsecure_config.yml").toAbsolutePath()), "/app/config.yml")
              .start();
    int mappedPort = grpcServer.getMappedPort(4312);

    var grpcChannel = GrpcChannelBuilder.forEndpoint("http://localhost:" + mappedPort)
                                        .withUnsecureConnection()
                                        .build();
    // when
    var user = getUser(grpcChannel);

    // then
    assertThat(user.getUsername()).isEqualTo("admin");
    grpcChannel.shutdown();
    grpcServer.close();
  }

  @Test
  void should_throw_exception_when_insecure_connection_is_set_with_ca_certificate() {
    assertThatThrownBy(() ->
      GrpcChannelBuilder.forEndpoint("http://localhost:4312")
                        .withUnsecureConnection()
                        .withCaPem("src/test/resources/camouflage/certs/server/server.cert")
                        .build()
    ).isInstanceOf(IllegalStateException.class);
  }

  @Test
  void should_throw_exception_when_insecure_connection_is_set_with_client_certificate() {
    assertThatThrownBy(() ->
      GrpcChannelBuilder.forEndpoint("http://localhost:4312")
                        .withUnsecureConnection()
                        .withClientCertificate(PemClientCertificate.of(
                          "src/test/resources/camouflage/certs/client/client.cert",
                          "src/test/resources/camouflage/certs/client/client.key"))
                        .build()
    ).isInstanceOf(IllegalStateException.class);
  }

  private static AuthCommon.User getUser(ManagedChannel channel) {
    var authenticationBlockingStub = AuthenticationGrpc.newBlockingStub(channel);
    var getUserRequest = AuthCommon.GetCurrentUserRequest.newBuilder().build();
    var getUserResponse = authenticationBlockingStub.getCurrentUser(getUserRequest);
    return getUserResponse.getUser();
  }

  private static void generateClientCert(KeyPair caKeyPair, X509Certificate caCert) throws Exception {
    var clientKeyPair = generateKeyPair();
    var clientCert = generateSignedCert(clientKeyPair.getPublic(), caKeyPair.getPrivate(), caCert, "CN=client", false);
    saveCertAsPem(clientCert, "src/test/resources/camouflage/certs/client/client.cert");
    saveKeyAsPem(clientKeyPair.getPrivate(), "src/test/resources/camouflage/certs/client/client.key");
    savePkcs12(clientKeyPair.getPrivate(), clientCert);
  }

  private static void generateServerCert(KeyPair caKeyPair, X509Certificate caCert) throws Exception {
    var serverKeyPair = generateKeyPair();
    var serverCert = generateSignedCert(serverKeyPair.getPublic(), caKeyPair.getPrivate(), caCert, "CN=localhost", true);
    saveCertAsPem(serverCert, "src/test/resources/camouflage/certs/server/server.cert");
    saveKeyAsPem(serverKeyPair.getPrivate(), "src/test/resources/camouflage/certs/server/server.key");
  }

  private static KeyPair generateKeyPair() throws Exception {
    KeyPairGenerator keyGen = KeyPairGenerator.getInstance("RSA");
    keyGen.initialize(2048);
    return keyGen.generateKeyPair();
  }

  private static X509Certificate generateCaCert(KeyPair caKeyPair) throws Exception {
    var issuer = new X500Name("CN=ArmoniKTestCA");
    var serial = BigInteger.valueOf(currentTimeMillis());
    var notBefore = new Date();
    var notAfter = Date.from(now().plusYears(5).toInstant(UTC));
    var signer = new JcaContentSignerBuilder("SHA256withRSA").build(caKeyPair.getPrivate());
    var certBuilder = new JcaX509v3CertificateBuilder(issuer, serial, notBefore, notAfter, issuer, caKeyPair.getPublic());
    certBuilder.addExtension(basicConstraints, true, new BasicConstraints(true));
    var caCert = new JcaX509CertificateConverter().getCertificate(certBuilder.build(signer));

    saveCertAsPem(caCert, "src/test/resources/camouflage/certs/server/ca.cert");
    return caCert;
  }

  private static X509Certificate generateSignedCert(PublicKey subjectPublicKey,
                                                    PrivateKey caPrivateKey,
                                                    X509Certificate caCert,
                                                    String subject,
                                                    boolean isServer) throws Exception {
    var issuer = new X500Name(caCert.getSubjectX500Principal().getName());
    var subjectName = new X500Name(subject);
    var serial = BigInteger.valueOf(currentTimeMillis());
    var notBefore = new Date();
    var notAfter = Date.from(now().plusYears(1).toInstant(UTC));
    var signer = new JcaContentSignerBuilder("SHA256withRSA").build(caPrivateKey);
    var certBuilder = new JcaX509v3CertificateBuilder(issuer, serial, notBefore, notAfter, subjectName, subjectPublicKey);

    if (isServer) {
      GeneralNames subjectAltNames = new GeneralNames(new GeneralName[]{
        new GeneralName(GeneralName.dNSName, "localhost"),
        new GeneralName(GeneralName.iPAddress, "127.0.0.1")
      });
      certBuilder.addExtension(Extension.subjectAlternativeName, false, subjectAltNames);
    }
    X509CertificateHolder certHolder = certBuilder.build(signer);
    return new JcaX509CertificateConverter().getCertificate(certHolder);
  }

  private static void saveCertAsPem(X509Certificate cert, String filename) throws Exception {
    try (JcaPEMWriter pemWriter = new JcaPEMWriter(new FileWriter(filename))) {
      pemWriter.writeObject(cert);
    }
  }

  private static void saveKeyAsPem(PrivateKey key, String filename) throws Exception {
    try (PemWriter pemWriter = new PemWriter(new FileWriter(filename))) {
      pemWriter.writeObject(new PemObject("PRIVATE KEY", key.getEncoded()));
    }
  }

  private static void savePkcs12(PrivateKey privateKey, X509Certificate cert) throws Exception {
    KeyStore keyStore = KeyStore.getInstance("PKCS12");
    keyStore.load(null, null);
    keyStore.setKeyEntry("client", privateKey, "".toCharArray(), new Certificate[]{cert});

    try (FileOutputStream fos = new FileOutputStream("src/test/resources/camouflage/certs/client/client.p12")) {
      keyStore.store(fos, "".toCharArray());
    }
  }
}
