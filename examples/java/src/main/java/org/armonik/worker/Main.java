package org.armonik.worker;

import armonik.api.grpc.v1.worker.WorkerCommon;
import armonik.worker.FutureWorker;
import armonik.worker.Worker;
import io.grpc.ManagedChannel;
import io.grpc.ManagedChannelBuilder;
import io.grpc.Server;
import io.grpc.ServerBuilder;
import io.grpc.stub.StreamObserver;

import java.io.IOException;
import java.util.List;

public class Main {

    public static void main(String[] args) throws IOException, InterruptedException {
        // Creating a managed channel to connect to the server
        ManagedChannel managedChannel = ManagedChannelBuilder
                .forAddress("::ffff:172.18.64.170", 5001)
                .usePlaintext()
                .build();

        // Creating a FutureWorker instance to handle tasks asynchronously
        var worker = new FutureWorker(managedChannel, taskHandler -> {
            System.out.println(">> TASK HANDLER IS BEING EXECUTED...");
        });

        // Processing a request and defining response handling logic
        worker.process(WorkerCommon.ProcessRequest.newBuilder().build(), new StreamObserver<>() {
            @Override
            public void onNext(WorkerCommon.ProcessReply processReply) {
                System.out.println(">> REPLY:");
                System.out.println(processReply);
            }

            @Override
            public void onError(Throwable throwable) {
                System.out.println(">> ERROR/");
                System.out.println(throwable.getMessage());
            }

            @Override
            public void onCompleted() {
                System.out.println("COMPLETED");
            }
        });
    }

}
