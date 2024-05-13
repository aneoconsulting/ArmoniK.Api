package armonik.client.task.util;

import armonik.api.grpc.v1.tasks.TasksCommon.ListTasksRequest.Sort;
import armonik.api.grpc.v1.tasks.TasksFilters;

public record ListTasksRequestParams(int page, int pageSize, TasksFilters.Filters filters, Sort sort) {
}
