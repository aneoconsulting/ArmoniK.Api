package armonik.worker;

import java.io.File;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.concurrent.TimeUnit;
import java.util.logging.Logger;

import io.grpc.BindableService;
import io.grpc.ForwardingServerCall;
import io.grpc.Metadata;
import io.grpc.Server;
import io.grpc.ServerCall;
import io.grpc.ServerCallHandler;
import io.grpc.ServerInterceptor;
import io.grpc.ServerStreamTracer;
import io.grpc.Status;
import io.grpc.netty.NettyServerBuilder;
import io.netty.channel.ChannelOption;
import io.netty.channel.epoll.EpollEventLoopGroup;
import io.netty.channel.epoll.EpollServerDomainSocketChannel;
import io.netty.channel.unix.DomainSocketAddress;
import io.netty.handler.codec.http2.Http2FrameLogger;
import io.netty.handler.logging.LogLevel;

public class UnixDomainSocketGrpcServer {
    private static final Logger logger = Logger.getLogger(UnixDomainSocketGrpcServer.class.getName());

    public UnixDomainSocketGrpcServer() {

    }

    private Server server;

    private static class LoggingInterceptor implements ServerInterceptor {
        private static final Logger logger = Logger.getLogger(LoggingInterceptor.class.getName());

        @Override
        public <ReqT, RespT> ServerCall.Listener<ReqT> interceptCall(
                ServerCall<ReqT, RespT> call,
                Metadata headers,
                ServerCallHandler<ReqT, RespT> next) {
            logger.info("header received from client:" + headers);

            final ServerCall<ReqT, RespT> wrappedCall = new ForwardingServerCall.SimpleForwardingServerCall<ReqT, RespT>(
                    call) {
                @Override
                public void close(Status status, Metadata trailers) {
                    System.out.println("interceptor close(): " + status);
                    System.out.println("interceptor close() header: " + trailers);
                    super.close(status, trailers);
                }
            };
            return next.startCall(wrappedCall, headers);
        }
    }

    public void start(String socketPath, BindableService service) throws IOException {
        // Ensure the directory exists
        Path path = Paths.get(socketPath);
        Files.createDirectories(path.getParent());

        // Delete existing socket file if it exists
        Files.deleteIfExists(path);

        Http2FrameLogger frameLogger = new Http2FrameLogger(LogLevel.DEBUG, UnixDomainSocketGrpcServer.class);

        // Create a new Unix domain socket server
        server = NettyServerBuilder.forAddress(new DomainSocketAddress(new File(socketPath)))
                .channelType(EpollServerDomainSocketChannel.class)
                .bossEventLoopGroup(new EpollEventLoopGroup(1))
                .workerEventLoopGroup(new EpollEventLoopGroup(1))
                // .permitKeepAliveWithoutCalls(true)
                .permitKeepAliveTime(5, TimeUnit.SECONDS)
                .maxInboundMetadataSize(1024 * 10240)
                .maxConnectionAge(3, TimeUnit.MINUTES)
                .maxConnectionAgeGrace(2, TimeUnit.MINUTES)
                .withOption(ChannelOption.SO_KEEPALIVE, false)
                .withOption(ChannelOption.AUTO_CLOSE, false)
                .addService(service)
                .addStreamTracerFactory(new ServerStreamTracer.Factory() {
                    @Override
                    public ServerStreamTracer newServerStreamTracer(String fullMethodName, Metadata headers) {
                        logger.info("header StreamTracer:" + headers);
                        return new ServerStreamTracer() {
                            @Override
                            public void streamClosed(Status status) {
                                // Add your stream monitoring logic here
                                System.out.println("Stream closed: " + status);
                                System.out.println("Server saw stream closed on method " + fullMethodName
                                        + " with status " + status.toString());

                            }

                        };
                    }
                })
                .intercept(new LoggingInterceptor())
                .build()
                .start();

        logger.info("Server started, listening on " + socketPath);

        // Add shutdown hook
        Runtime.getRuntime().addShutdownHook(new Thread(() -> {
            try {
                shutdown();
            } catch (InterruptedException e) {
                logger.warning("Server shutdown interrupted");
            }
        }));
    }

    public void shutdown() throws InterruptedException {
        if (server != null) {
            server.shutdown().awaitTermination(30, TimeUnit.SECONDS);

            // Optional: delete the socket file after shutdown
            // try {
            // Files.deleteIfExists(Paths.get(server.getPort()));
            // } catch (IOException e) {
            // logger.warning("Failed to delete socket file: " + e.getMessage());
            // }
        }
    }

    public void blockUntilShutdown() throws InterruptedException {
        if (server != null) {
            server.awaitTermination();
        }
    }
}