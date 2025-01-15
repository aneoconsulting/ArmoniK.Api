package armonik.worker;

import java.io.File;
import java.net.InetAddress;
import java.net.UnknownHostException;
import java.util.logging.Logger;

import armonik.api.grpc.v1.Objects.Empty;
import armonik.api.grpc.v1.agent.AgentGrpc;
import armonik.api.grpc.v1.worker.WorkerCommon.HealthCheckReply;
import armonik.api.grpc.v1.worker.WorkerCommon.HealthCheckReply.ServingStatus;
import armonik.api.grpc.v1.worker.WorkerCommon.ProcessReply;
import armonik.api.grpc.v1.worker.WorkerCommon.ProcessRequest;
import armonik.api.grpc.v1.worker.WorkerGrpc;
import io.grpc.ManagedChannel;
import io.grpc.Server;
import io.grpc.netty.NegotiationType;
import io.grpc.netty.NettyChannelBuilder;
import io.grpc.stub.StreamObserver;
import io.netty.channel.epoll.EpollDomainSocketChannel;
import io.netty.channel.epoll.EpollEventLoopGroup;
import io.netty.channel.unix.DomainSocketAddress;

/**
 * FutureWorker is a gRPC worker implementation that processes requests
 * asynchronously using FutureTaskHandler.
 * It communicates with an Agent gRPC server using a FutureStub for handling
 * tasks and provides a health check
 * functionality to determine its serving status.
 */
public class FutureWorker extends WorkerGrpc.WorkerImplBase {
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

    private Server server;

    private String workerAddress = System.getenv("ComputePlane__WorkerChannel__Address");
    //
    private String agentAddress = System.getenv("ComputePlane__AgentChannel__Address");

    /**
     * Constructs a new FutureWorker with the specified managed channel
     *
     * @param channel the managed channel for communication with the
     *                Agent server
     * @throws UnknownHostException
     */
    public FutureWorker() throws UnknownHostException {
        System.out.println(InetAddress.getLocalHost().getHostAddress());
        logger.info("Creating the channel to communicate with the agent");
        ManagedChannel channel = NettyChannelBuilder.forAddress(new DomainSocketAddress(new File(agentAddress)))
                .channelType(EpollDomainSocketChannel.class)
                .overrideAuthority(InetAddress.getLocalHost().getHostAddress())
                .eventLoopGroup(new EpollEventLoopGroup()).usePlaintext()
                .negotiationType(NegotiationType.PLAINTEXT)
                .keepAliveWithoutCalls(true)
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

            responseObserver.onCompleted();

        } catch (Exception e) {
            status = ServingStatus.NOT_SERVING;
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
}