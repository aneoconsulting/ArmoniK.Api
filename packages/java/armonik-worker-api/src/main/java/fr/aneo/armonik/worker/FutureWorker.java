package fr.aneo.armonik.worker;

import fr.aneo.armonik.api.grpc.v1.Objects.Empty;
import fr.aneo.armonik.api.grpc.v1.Objects.Output;
import fr.aneo.armonik.api.grpc.v1.agent.AgentGrpc;
import fr.aneo.armonik.api.grpc.v1.worker.WorkerCommon.HealthCheckReply;
import fr.aneo.armonik.api.grpc.v1.worker.WorkerCommon.HealthCheckReply.ServingStatus;
import fr.aneo.armonik.api.grpc.v1.worker.WorkerCommon.ProcessReply;
import fr.aneo.armonik.api.grpc.v1.worker.WorkerCommon.ProcessRequest;
import fr.aneo.armonik.api.grpc.v1.worker.WorkerGrpc;
import fr.aneo.armonik.worker.taskhandlers.FutureTaskHandler;
import io.grpc.ManagedChannel;
import io.grpc.netty.shaded.io.grpc.netty.NettyChannelBuilder;
import io.grpc.stub.StreamObserver;

import java.net.MalformedURLException;
import java.net.URI;
import java.net.URISyntaxException;
import java.net.UnknownHostException;
import java.util.concurrent.TimeUnit;
import java.util.logging.Logger;

/**
 * FutureWorker is a gRPC worker implementation that processes requests
 * asynchronously using FutureTaskHandler.
 * It communicates with an Agent gRPC server using a FutureStub for handling
 * tasks and provides a health check
 * functionality to determine its serving status.
 */
public abstract class FutureWorker extends WorkerGrpc.WorkerImplBase {
    /** The gRPC client used to communicate with the Agent server asynchronously. */
    private final AgentGrpc.AgentFutureStub client;

    /**
     * The function to handle FutureTaskHandler for processing tasks asynchronously.
     */
    // private final Consumer<FutureTaskHandler> taskHandlerFunction;

    /** The serving status of the worker. */
    private ServingStatus status = ServingStatus.SERVING;

    private static final Logger logger = Logger.getLogger(FutureWorker.class.getName());

    public ServingStatus getStatus() {
        return status;
    }

    public void setStatus(ServingStatus value) {
        status = value;
    }

    public AgentGrpc.AgentFutureStub getClient() {
        return client;
    }

    /**
     * Constructs a new FutureWorker with the specified managed channel
     *
     * @param channel the managed channel for communication with the
     *                Agent server
     * @throws UnknownHostException
     * @throws URISyntaxException
     * @throws MalformedURLException
     */
    public FutureWorker() throws UnknownHostException, URISyntaxException, MalformedURLException {

        String agentAddress = System.getenv("ComputePlane__AgentChannel__Address");
        URI agent = new URI(agentAddress);

        logger.info("Creating the channel to communicate with the agent");
        ManagedChannel channel = NettyChannelBuilder.forAddress(agent.toURL().getHost(), agent.toURL().getPort())
                .keepAliveWithoutCalls(true)
                .keepAliveTime(30, TimeUnit.SECONDS)
                .keepAliveTimeout(10, TimeUnit.SECONDS)
                .maxInboundMessageSize(10 * 1024)
                .usePlaintext()
                .enableRetry()
                .build();
        client = AgentGrpc.newFutureStub(channel);
    }

    /**
     * Processes the incoming request asynchronously using FutureTaskHandler and
     * sends the response back.
     *
     * @param request          the incoming process request
     * @param responseObserver the response observer for sending back the process
     *                         reply
     */
    @Override
    public void process(ProcessRequest request, StreamObserver<ProcessReply> responseObserver) {
        try {
            setStatus(ServingStatus.NOT_SERVING);
            FutureTaskHandler futureTaskHandler = new FutureTaskHandler(request, client);
            Output output = processInternal(futureTaskHandler);

            responseObserver.onNext(
                    ProcessReply.newBuilder()
                            .setOutput(output)
                            .build());
            setStatus(ServingStatus.SERVING);
            responseObserver.onCompleted();

        } catch (Exception e) {
            status = ServingStatus.SERVING;
            responseObserver.onError(e);
        }
    }

    /**
     * Performs a health check and sends back the serving status of the worker.
     *
     * @param request          the health check request
     * @param responseObserver the response observer for sending back the health
     *                         check reply
     */
    @Override
    public void healthCheck(Empty request, StreamObserver<HealthCheckReply> responseObserver) {
        responseObserver.onNext(HealthCheckReply.newBuilder()
                .setStatus(status)
                .setStatusValue(status.getNumber())
                .build());
        responseObserver.onCompleted();
    }

    public abstract Output processInternal(FutureTaskHandler futureTaskHandler);
}
