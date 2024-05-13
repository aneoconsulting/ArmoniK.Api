package armonik.worker;

import armonik.api.grpc.v1.Objects.Empty;
import armonik.api.grpc.v1.Objects.Output;
import armonik.api.grpc.v1.agent.AgentGrpc;
import armonik.api.grpc.v1.worker.WorkerCommon.HealthCheckReply;
import armonik.api.grpc.v1.worker.WorkerCommon.HealthCheckReply.ServingStatus;
import armonik.api.grpc.v1.worker.WorkerCommon.ProcessReply;
import armonik.api.grpc.v1.worker.WorkerCommon.ProcessRequest;
import armonik.api.grpc.v1.worker.WorkerGrpc;
import armonik.worker.taskhandlers.FutureTaskHandler;
import io.grpc.ManagedChannel;
import io.grpc.stub.StreamObserver;

import java.util.function.Consumer;

/**
 * FutureWorker is a gRPC worker implementation that processes requests asynchronously using FutureTaskHandler.
 * It communicates with an Agent gRPC server using a FutureStub for handling tasks and provides a health check
 * functionality to determine its serving status.
 */
public class FutureWorker extends WorkerGrpc.WorkerImplBase {
  /** The gRPC client used to communicate with the Agent server asynchronously. */
  private final AgentGrpc.AgentFutureStub client;

  /** The function to handle FutureTaskHandler for processing tasks asynchronously. */
  private final Consumer<FutureTaskHandler> taskHandlerFunction;

  /** The serving status of the worker. */
  private ServingStatus status = ServingStatus.SERVING;


  public ServingStatus getStatus() {
    return status;
  }

  /**
   * Constructs a new FutureWorker with the specified managed channel and task handler consumer.
   *
   * @param channel the managed channel for communication with the Agent server
   * @param taskHandlerConsumer the consumer function to handle FutureTaskHandler for task processing
   */
  public FutureWorker(ManagedChannel channel, Consumer<FutureTaskHandler> taskHandlerConsumer) {
    client = AgentGrpc.newFutureStub(channel);
    this.taskHandlerFunction = taskHandlerConsumer;
  }

  /**
   * Processes the incoming request asynchronously using FutureTaskHandler and sends the response back.
   *
   * @param request the incoming process request
   * @param responseObserver the response observer for sending back the process reply
   */
  @Override
  public void process(ProcessRequest request, StreamObserver<ProcessReply> responseObserver) {
    try {
      status = ServingStatus.SERVING;
      FutureTaskHandler taskHandler = new FutureTaskHandler(request,client);
      taskHandlerFunction.accept(taskHandler);
      responseObserver.onNext(
        ProcessReply.newBuilder()
          .setOutput(Output
            .newBuilder()
            .setOk(Empty.newBuilder().build())
            .build())
          .build());

      responseObserver.onCompleted();

    } catch (Exception e) {
      status = ServingStatus.NOT_SERVING;
      responseObserver.onError(e);
    }
  }

  /**
   * Performs a health check and sends back the serving status of the worker.
   *
   * @param request the health check request
   * @param responseObserver the response observer for sending back the health check reply
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


