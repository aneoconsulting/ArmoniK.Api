package fr.aneo.armonik.client;

import fr.aneo.armonik.api.grpc.v1.auth.AuthCommon;
import fr.aneo.armonik.api.grpc.v1.auth.AuthenticationGrpc;
import io.grpc.ManagedChannel;
import org.junit.jupiter.api.Test;
import org.testcontainers.containers.GenericContainer;

import java.nio.file.Paths;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;
import static org.testcontainers.utility.MountableFile.forHostPath;

class GrpcChannelConnectionTest {

  @SuppressWarnings("resource")
  private static final GenericContainer<?> grpcServer = new GenericContainer<>("shubhendumadhukar/camouflage:latest")
    .withExposedPorts(4312)
    .withCopyFileToContainer(forHostPath(Paths.get("src/test/resources/camouflage/grpc").toAbsolutePath()), "/app/grpc");

  @Test
  void connect_to_grpc_server_with_tls_mode() {
    // given
    grpcServer
      .withCopyFileToContainer(forHostPath(Paths.get("src/test/resources/camouflage/tls_config.yml").toAbsolutePath()), "/app/config.yml")
      .withCopyFileToContainer(forHostPath(Paths.get("src/test/resources/camouflage/certs/server").toAbsolutePath()), "/app/certs")
      .start();
    int mappedPort = grpcServer.getMappedPort(4312);

    var channel = GrpcChannelBuilder.forEndpoint("https://localhost:" + mappedPort)
                                    .withCaPem("src/test/resources/camouflage/certs/server/server.cert")
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
    grpcServer
      .withCopyFileToContainer(forHostPath(Paths.get("src/test/resources/camouflage/mtls_config.yml").toAbsolutePath()), "/app/config.yml")
      .withCopyFileToContainer(forHostPath(Paths.get("src/test/resources/camouflage/certs/server").toAbsolutePath()), "/app/certs")
      .start();
    int mappedPort = grpcServer.getMappedPort(4312);

    var channel = GrpcChannelBuilder.forEndpoint("https://localhost:" + mappedPort)
                                    .withCaPem("src/test/resources/camouflage/certs/server/server.cert")
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
    grpcServer
      .withCopyFileToContainer(forHostPath(Paths.get("src/test/resources/camouflage/mtls_config.yml").toAbsolutePath()), "/app/config.yml")
      .withCopyFileToContainer(forHostPath(Paths.get("src/test/resources/camouflage/certs/server").toAbsolutePath()), "/app/certs")
      .start();
    int mappedPort = grpcServer.getMappedPort(4312);

    var channel = GrpcChannelBuilder.forEndpoint("https://localhost:" + mappedPort)
                                    .withCaPem("src/test/resources/camouflage/certs//server/server.cert")
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
    grpcServer
      .withCopyFileToContainer(forHostPath(Paths.get("src/test/resources/camouflage/unsecure_config.yml").toAbsolutePath()), "/app/config.yml")
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
    // given

    // when/then
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
}
