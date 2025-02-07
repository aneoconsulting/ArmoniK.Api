package armonik.worker.taskhandlers;

import java.io.ByteArrayOutputStream;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.nio.file.Paths;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

import com.google.common.util.concurrent.ListenableFuture;
import com.google.protobuf.ProtocolStringList;

import armonik.api.grpc.v1.Objects;
import armonik.api.grpc.v1.Objects.TaskOptions;
import armonik.api.grpc.v1.agent.AgentCommon;
import armonik.api.grpc.v1.agent.AgentCommon.CreateResultsMetaDataRequest;
import armonik.api.grpc.v1.agent.AgentCommon.CreateResultsMetaDataResponse;
import armonik.api.grpc.v1.agent.AgentCommon.CreateResultsRequest.ResultCreate;
import armonik.api.grpc.v1.agent.AgentCommon.CreateResultsResponse;
import armonik.api.grpc.v1.agent.AgentCommon.NotifyResultDataRequest;
import armonik.api.grpc.v1.agent.AgentCommon.NotifyResultDataResponse;
import armonik.api.grpc.v1.agent.AgentCommon.SubmitTasksRequest.TaskCreation;
import armonik.api.grpc.v1.agent.AgentCommon.SubmitTasksResponse;
import armonik.api.grpc.v1.agent.AgentGrpc;
import armonik.api.grpc.v1.worker.WorkerCommon.ProcessRequest;

/**
 * FutureTaskHandler is responsible for handling tasks asynchronously in a gRPC
 * worker.
 * It processes incoming requests and provides methods for interacting with
 * task-related functionalities,
 * such as submitting tasks, creating results, notifying result data, and
 * accessing task information.
 */
public class FutureTaskHandler {
    private final AgentGrpc.AgentFutureStub client;

    /** The session ID associated with the task. */
    private final String sessionId;

    /** The ID of the task being processed. */
    private final String taskId;

    /** The options for the task. */
    private final TaskOptions taskOptions;

    /** The communication token for the task. */
    private final String token;

    /** The list of expected result keys. */
    private final List<String> expectedResults;

    /** Configuration associated with the task. */
    private final Objects.Configuration configuration;

    /** The ID of the payload associated with the task. */
    private final String payloadId;

    /** The folder containing data for the task. */
    private final String dataFolder;

    /** The payload data associated with the task. */
    private final byte[] payload;

    /** The map containing data dependencies for the task. */
    private final Map<String, byte[]> dataDependency;

    /**
     * Constructs a new FutureTaskHandler with the provided process request and gRPC
     * client.
     *
     * @param request The process request containing task information.
     * @param client  The gRPC client for interacting with the Agent server.
     */
    public FutureTaskHandler(ProcessRequest request, AgentGrpc.AgentFutureStub client) {
        this.client = client;
        this.sessionId = request.getSessionId();
        this.taskId = request.getTaskId();
        this.taskOptions = request.getTaskOptions();
        this.token = request.getCommunicationToken();
        this.expectedResults = request.getExpectedOutputKeysList();
        this.configuration = request.getConfiguration();
        this.payloadId = request.getPayloadId();
        this.dataFolder = request.getDataFolder();
        this.payload = readAndGetContent(dataFolder, payloadId);
        this.dataDependency = initDataDependency(request.getDataDependenciesList());
    }

    private Map<String, byte[]> initDataDependency(ProtocolStringList dataDependenciesList) {
        Map<String, byte[]> dataDependency = new HashMap<>();
        dataDependenciesList.forEach(dd -> dataDependency.put(dd, readAndGetContent(dataFolder, dd)));
        return dataDependency;
    }

    private byte[] readAndGetContent(String dataFolder, String payloadId) {
        try (InputStream inputStream = new FileInputStream(Paths.get(dataFolder, payloadId).toFile())) {
            ByteArrayOutputStream outputStream = new ByteArrayOutputStream();

            byte[] buffer = new byte[1024];
            int bytesRead;
            while ((bytesRead = inputStream.read(buffer)) != -1) {
                outputStream.write(buffer, 0, bytesRead);
            }

            return outputStream.toByteArray();
        } catch (IOException e) {
            System.out.println("Payload not found: " + e.getMessage());
        }
        return null;
    }

    /**
     * Submits tasks asynchronously with the provided task creations and task
     * options.
     *
     * @param taskCreations The list of task creations to be submitted.
     * @param taskOptions   The options for the submitted tasks.
     * @return A ListenableFuture representing the submit tasks operation.
     */
    public ListenableFuture<SubmitTasksResponse> submitTasks(List<TaskCreation> taskCreations,
            TaskOptions taskOptions) {
        return client.submitTasks(
                AgentCommon.SubmitTasksRequest.newBuilder()
                        .setSessionId(sessionId)
                        .addAllTaskCreations(taskCreations)
                        .setTaskOptions(taskOptions == null ? this.getTaskOptions() : taskOptions) // getDefaultInstance()
                        .setCommunicationToken(token)
                        .build());
    }

    /**
     * Creates results asynchronously with the provided result creations.
     *
     * @param resultCreates The list of result creations to be created.
     * @return A ListenableFuture representing the create results operation.
     */
    public ListenableFuture<CreateResultsResponse> createResults(List<ResultCreate> resultCreates) {
        return client.createResults(AgentCommon.CreateResultsRequest.newBuilder()
                .setCommunicationToken(token)
                .setSessionId(sessionId)
                .addAllResults(resultCreates)
                .build());
    }

    /**
     * Creates results metadata asynchronously with the provided result creations.
     *
     * @param resultCreates The list of result creations for metadata creation.
     * @return A ListenableFuture representing the create results metadata
     *         operation.
     */
    public ListenableFuture<CreateResultsMetaDataResponse> createResultsMetaData(
            List<CreateResultsMetaDataRequest.ResultCreate> resultCreates) {
        return client.createResultsMetaData(CreateResultsMetaDataRequest.newBuilder()
                .setCommunicationToken(token)
                .setSessionId(sessionId)
                .addAllResults(resultCreates)
                .build());
    }

    /**
     * Notifies result data asynchronously with the provided key and data.
     *
     * @param key  The key associated with the result data.
     * @param data The data to be notified.
     * @return A ListenableFuture representing the notify result data operation.
     * @throws IOException if an I/O error occurs while notifying result data.
     */
    public ListenableFuture<NotifyResultDataResponse> notifyResultData(String key, byte[] data) throws IOException {
        try (FileOutputStream fs = new FileOutputStream(Paths.get(dataFolder, key).toString(), true)) {
            fs.write(data);
        }
        NotifyResultDataRequest request = NotifyResultDataRequest.newBuilder()
                .setCommunicationToken(token)
                .addIds(NotifyResultDataRequest.ResultIdentifier.newBuilder()
                        .setSessionId(sessionId)
                        .setResultId(key)
                        .buildPartial())
                .build();
        return client.notifyResultData(request);
    }

    public byte[] getPayload() {
        return payload;
    }

    public String getTaskId() {
        return taskId;
    }

    public TaskOptions getTaskOptions() {
        return taskOptions;
    }

    public List<String> getExpectedResults() {
        return expectedResults;
    }

    public Objects.Configuration getConfiguration() {
        return configuration;
    }

    public String getPayloadId() {
        return payloadId;
    }

    public Map<String, byte[]> getDataDependency() {
        return dataDependency;
    }
}