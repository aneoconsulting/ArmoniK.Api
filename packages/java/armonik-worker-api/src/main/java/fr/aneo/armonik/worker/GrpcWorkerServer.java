package fr.aneo.armonik.worker;

import io.grpc.BindableService;
import io.grpc.Server;
import io.grpc.netty.shaded.io.grpc.netty.NettyServerBuilder;

import java.io.IOException;
import java.net.URI;
import java.net.URISyntaxException;
import java.util.concurrent.TimeUnit;
import java.util.logging.Logger;

public class GrpcWorkerServer {
    private static final Logger logger = Logger.getLogger(GrpcWorkerServer.class.getName());

    public GrpcWorkerServer() {

    }

    private Server server;

    public void start(String workerAddress, BindableService service) throws IOException, URISyntaxException {
        URI worker = new URI(workerAddress);
        server = NettyServerBuilder.forPort(worker.toURL().getPort())
                .permitKeepAliveWithoutCalls(true)
                .permitKeepAliveTime(10, TimeUnit.SECONDS)
                .keepAliveTime(10, TimeUnit.SECONDS)
                .maxInboundMetadataSize(1024 * 10240)
                .keepAliveTimeout(5, TimeUnit.SECONDS)
                .maxInboundMetadataSize(1024 * 1024)
                .maxInboundMessageSize(4 * 1024 * 1024)
                .flowControlWindow(65535)
                .initialFlowControlWindow(1024)
                .maxConcurrentCallsPerConnection(1000)
                .addService(service)
                .build()
                .start();

        logger.info("Server started, listening on " + workerAddress);

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
        }
    }

    public void blockUntilShutdown() throws InterruptedException {
        if (server != null) {
            server.awaitTermination();
        }
    }
}
