package armonik.client.task.util;

import armonik.api.grpc.v1.Objects;
import armonik.api.grpc.v1.tasks.TasksCommon;

import java.util.List;

public record SubmitTasksRequestParams(String sessionId,
                                       List<TasksCommon.SubmitTasksRequest.TaskCreation> taskCreations,
                                       Objects.TaskOptions taskOptions) {
}
