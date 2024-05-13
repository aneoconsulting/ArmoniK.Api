package org.armonik.client;


import armonik.api.grpc.v1.Objects.TaskOptions;
import armonik.api.grpc.v1.results.ResultsCommon.CreateResultsRequest;
import armonik.api.grpc.v1.results.ResultsCommon.CreateResultsRequest.ResultCreate;
import armonik.api.grpc.v1.results.ResultsCommon.ResultRaw;
import armonik.api.grpc.v1.tasks.TasksCommon.SubmitTasksRequest.TaskCreation;
import armonik.client.event.EventClient;
import armonik.client.event.util.records.EventSubscriptionResponseRecord;
import armonik.client.result.ResultClient;
import armonik.client.session.SessionClient;
import armonik.client.task.TaskClient;
import armonik.client.task.util.SubmitTasksRequestParams;
import ch.qos.logback.core.testUtil.RandomUtil;
import com.google.protobuf.ByteString;
import com.google.protobuf.Duration;
import io.grpc.ManagedChannel;
import io.grpc.ManagedChannelBuilder;

import java.util.ArrayList;
import java.util.List;

public class Main {
    public static void main(String[] args) throws InterruptedException {
        // Creating a managed channel to connect to the server
        ManagedChannel managedChannel = ManagedChannelBuilder.forAddress("::ffff:172.18.64.170", 5001).usePlaintext().build();


        // Creating a synchronous session client to interact with sessions
        SessionClient sessionClient = new SessionClient(managedChannel);

        // Creating a synchronous task client for task submission
        TaskClient taskClient = new TaskClient(managedChannel);

        // Defining options for the task
        TaskOptions taskOptions = TaskOptions.newBuilder()
                .setMaxDuration(Duration.newBuilder().setSeconds(3600).build())
                .setMaxRetries(2)
                .setPriority(1)
                .setApplicationVersion("1.0.0-700")
                .setApplicationService("ServiceApps")
                .setApplicationNamespace("Armonik.Samples.StressTests.Worker")
                .setApplicationName("Armonik.Samples.StressTests.Worker")
                .setPartitionId("default")
                .build();


        // Creating a session and obtaining its ID
        String sessionId = sessionClient.createSession(taskOptions, List.of("default"));
        System.out.println(">> Session ID:" + sessionId);

        // Create client for result creation
        ResultClient resultClient = new ResultClient(managedChannel);

        List<String> names = List.of("Result Name 1", "Result Name 2", "Result Name 3");
        List<String> resultMetaDataIds = resultClient.createResultsMetaData(sessionId, names).stream().map(ResultRaw::getResultId).toList();


        // Create the payload metadata (a result) and upload data at the same time
        List<ResultRaw> results = resultClient.createResults(
                CreateResultsRequest.newBuilder()
                        .setSessionId(sessionId)
                        .addResults(ResultCreate.newBuilder()
                                .setName("Payload")
                                .setData(ByteString.copyFrom(Integer.toBinaryString(RandomUtil.getPositiveInt()).getBytes()))
                                .build())
                        .build()
        );

        System.out.println(">> RESULTS RAW: ");
        results.forEach(System.out::println);

        List<TaskCreation> taskCreations = new ArrayList<>();
        for (ResultRaw resultRaw : results) {
            TaskCreation taskCreation = TaskCreation
                    .newBuilder()
                    .setPayloadId(resultRaw.getResultId())
                    .build();
            resultMetaDataIds.forEach(s -> taskCreation.toBuilder().addExpectedOutputKeys(s).build());
            taskCreations.add(taskCreation);
        }


        //submit and get the taskId for the submitted task
        String taskId = taskClient.submitTasks(new SubmitTasksRequestParams(sessionId, taskCreations, taskOptions))
                .get(0)
                .getTaskId();

        System.out.println(">> Task ID: " + taskId);
        EventClient eventClient = new EventClient(managedChannel);


        //getting events
        List<EventSubscriptionResponseRecord> events = eventClient.getEvents(sessionId, results.stream().map(ResultRaw::getResultId).toList());
        System.out.println(">> Events found:");
        events.forEach(System.out::println);


        System.out.println("DOWNLOADING DATA FOR SESSION-ID: " + sessionId + " AND RESULT-ID:" + results.get(0).getResultId());
        List<byte[]> bytes = resultClient.downloadResultData(sessionId, results.get(0).getResultId());

        System.out.println("DATA DOWNLOADED: " + bytes);


    }
}