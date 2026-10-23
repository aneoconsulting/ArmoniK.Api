# V1
<a id="top"></a>

## agent_common.proto



<a id="armonik-api-grpc-v1-agent-CreateResultsMetaDataRequest"></a>

### CreateResultsMetaDataRequest
Request for creating results without data


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| results | [CreateResultsMetaDataRequest.ResultCreate](#armonik-api-grpc-v1-agent-CreateResultsMetaDataRequest-ResultCreate) | repeated | The list of results to create. |
| session_id | [string](#string) |  | The session in which create results. |
| communication_token | [string](#string) |  | Communication token received by the worker during task processing |






<a id="armonik-api-grpc-v1-agent-CreateResultsMetaDataRequest-ResultCreate"></a>

### CreateResultsMetaDataRequest.ResultCreate
A result to create.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| name | [string](#string) |  | The result name. Given by the client. |






<a id="armonik-api-grpc-v1-agent-CreateResultsMetaDataResponse"></a>

### CreateResultsMetaDataResponse
Response for creating results without data


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| results | [ResultMetaData](#armonik-api-grpc-v1-agent-ResultMetaData) | repeated | The list of metadata results that were created. |
| communication_token | [string](#string) |  | Communication token received by the worker during task processing |






<a id="armonik-api-grpc-v1-agent-CreateResultsRequest"></a>

### CreateResultsRequest
Request for creating results with data


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| results | [CreateResultsRequest.ResultCreate](#armonik-api-grpc-v1-agent-CreateResultsRequest-ResultCreate) | repeated | The results to create. |
| session_id | [string](#string) |  | The session in which create results. |
| communication_token | [string](#string) |  | Communication token received by the worker during task processing |






<a id="armonik-api-grpc-v1-agent-CreateResultsRequest-ResultCreate"></a>

### CreateResultsRequest.ResultCreate
A result to create.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| name | [string](#string) |  | The result name. Given by the client. |
| data | [bytes](#bytes) |  | The actual data of the result. |






<a id="armonik-api-grpc-v1-agent-CreateResultsResponse"></a>

### CreateResultsResponse
Response for creating results without data


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| results | [ResultMetaData](#armonik-api-grpc-v1-agent-ResultMetaData) | repeated | The raw results that were created. |
| communication_token | [string](#string) |  | Communication token received by the worker during task processing |






<a id="armonik-api-grpc-v1-agent-CreateTaskReply"></a>

### CreateTaskReply



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| creation_status_list | [CreateTaskReply.CreationStatusList](#armonik-api-grpc-v1-agent-CreateTaskReply-CreationStatusList) |  |  |
| error | [string](#string) |  |  |
| communication_token | [string](#string) |  | Communication token received by the worker during task processing |






<a id="armonik-api-grpc-v1-agent-CreateTaskReply-CreationStatus"></a>

### CreateTaskReply.CreationStatus



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task_info | [CreateTaskReply.TaskInfo](#armonik-api-grpc-v1-agent-CreateTaskReply-TaskInfo) |  |  |
| error | [string](#string) |  |  |






<a id="armonik-api-grpc-v1-agent-CreateTaskReply-CreationStatusList"></a>

### CreateTaskReply.CreationStatusList



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| creation_statuses | [CreateTaskReply.CreationStatus](#armonik-api-grpc-v1-agent-CreateTaskReply-CreationStatus) | repeated |  |






<a id="armonik-api-grpc-v1-agent-CreateTaskReply-TaskInfo"></a>

### CreateTaskReply.TaskInfo



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task_id | [string](#string) |  | The task ID. |
| expected_output_keys | [string](#string) | repeated | The expected output IDs. A task have expected output IDs. |
| data_dependencies | [string](#string) | repeated | The data dependencies IDs (inputs). A task have data dependencies. |
| payload_id | [string](#string) |  | Unique ID of the result that will be used as payload. Results are created implicitly. |






<a id="armonik-api-grpc-v1-agent-CreateTaskRequest"></a>

### CreateTaskRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| init_request | [CreateTaskRequest.InitRequest](#armonik-api-grpc-v1-agent-CreateTaskRequest-InitRequest) |  |  |
| init_task | [armonik.api.grpc.v1.InitTaskRequest](#armonik-api-grpc-v1-InitTaskRequest) |  |  |
| task_payload | [armonik.api.grpc.v1.DataChunk](#armonik-api-grpc-v1-DataChunk) |  |  |
| communication_token | [string](#string) |  | Communication token received by the worker during task processing |






<a id="armonik-api-grpc-v1-agent-CreateTaskRequest-InitRequest"></a>

### CreateTaskRequest.InitRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task_options | [armonik.api.grpc.v1.TaskOptions](#armonik-api-grpc-v1-TaskOptions) |  |  |






<a id="armonik-api-grpc-v1-agent-DataRequest"></a>

### DataRequest
Request to retrieve data


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| communication_token | [string](#string) |  | Communication token received by the worker during task processing |
| result_id | [string](#string) |  | Id of the result that will be retrieved |






<a id="armonik-api-grpc-v1-agent-DataResponse"></a>

### DataResponse
Response when data is available in the shared folder


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| result_id | [string](#string) |  | Id of the result that will be retrieved |






<a id="armonik-api-grpc-v1-agent-NotifyResultDataRequest"></a>

### NotifyResultDataRequest
Request for notifying results data are available in files.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| ids | [NotifyResultDataRequest.ResultIdentifier](#armonik-api-grpc-v1-agent-NotifyResultDataRequest-ResultIdentifier) | repeated | The possible messages that constitute a UploadResultDataRequest

* The identifier of the result to which add data. |
| communication_token | [string](#string) |  | Communication token received by the worker during task processing |






<a id="armonik-api-grpc-v1-agent-NotifyResultDataRequest-ResultIdentifier"></a>

### NotifyResultDataRequest.ResultIdentifier
The metadata to identify the result to update.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session_id | [string](#string) |  | The session of the result. |
| result_id | [string](#string) |  | The ID of the result. |






<a id="armonik-api-grpc-v1-agent-NotifyResultDataResponse"></a>

### NotifyResultDataResponse
Response for notifying data file availability for result
Received when data are successfully copied to the ObjectStorage


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| result_ids | [string](#string) | repeated | The Id of the result to which data were added |






<a id="armonik-api-grpc-v1-agent-ResultMetaData"></a>

### ResultMetaData
Result metadata


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session_id | [string](#string) |  | The session ID. |
| result_id | [string](#string) |  | The result ID. |
| name | [string](#string) |  | The result name. |
| status | [armonik.api.grpc.v1.result_status.ResultStatus](#armonik-api-grpc-v1-result_status-ResultStatus) |  | The result status. |
| created_at | [google.protobuf.Timestamp](#google-protobuf-Timestamp) |  | The result creation date. |






<a id="armonik-api-grpc-v1-agent-SubmitTasksRequest"></a>

### SubmitTasksRequest
Request to create tasks.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session_id | [string](#string) |  | The session ID. |
| task_options | [armonik.api.grpc.v1.TaskOptions](#armonik-api-grpc-v1-TaskOptions) |  | The options for the tasks. Each task will have the same. Options are merged with the one from the session. |
| task_creations | [SubmitTasksRequest.TaskCreation](#armonik-api-grpc-v1-agent-SubmitTasksRequest-TaskCreation) | repeated | Task creation requests. |
| communication_token | [string](#string) |  | Communication token received by the worker during task processing |






<a id="armonik-api-grpc-v1-agent-SubmitTasksRequest-TaskCreation"></a>

### SubmitTasksRequest.TaskCreation



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| expected_output_keys | [string](#string) | repeated | Unique ID of the results that will be produced by the task. Results must be created using ResultsService. |
| data_dependencies | [string](#string) | repeated | Unique ID of the results that will be used as datadependencies. Results must be created using ResultsService. |
| payload_id | [string](#string) |  | Unique ID of the result that will be used as payload. Result must be created using ResultsService. |
| task_options | [armonik.api.grpc.v1.TaskOptions](#armonik-api-grpc-v1-TaskOptions) |  | Optionnal task options. |






<a id="armonik-api-grpc-v1-agent-SubmitTasksResponse"></a>

### SubmitTasksResponse
Response to create tasks.

expected_output_ids and data_dependencies must be created through ResultsService.

Remark : this may have to be enriched to a better management of errors but
will the client application be able to manage a missing data dependency or expected output ?


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task_infos | [SubmitTasksResponse.TaskInfo](#armonik-api-grpc-v1-agent-SubmitTasksResponse-TaskInfo) | repeated | List of task infos if submission successful, else throw gRPC exception. |
| communication_token | [string](#string) |  | Communication token received by the worker during task processing |






<a id="armonik-api-grpc-v1-agent-SubmitTasksResponse-TaskInfo"></a>

### SubmitTasksResponse.TaskInfo



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task_id | [string](#string) |  | The task ID. |
| expected_output_ids | [string](#string) | repeated | The expected output IDs. A task has expected output IDs. |
| data_dependencies | [string](#string) | repeated | The data dependencies IDs (inputs). A task has data dependencies. |
| payload_id | [string](#string) |  | Unique ID of the result that will be used as payload. Results are created implicitly. |





 

 

 

 



<a id="agent_service-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## agent_service.proto


 

 

 


<a id="armonik-api-grpc-v1-agent-Agent"></a>

### Agent


| Method Name | Request Type | Response Type | Description |
| ----------- | ------------ | ------------- | ------------|
| CreateTask | [CreateTaskRequest](#armonik-api-grpc-v1-agent-CreateTaskRequest) stream | [CreateTaskReply](#armonik-api-grpc-v1-agent-CreateTaskReply) |  |
| CreateResultsMetaData | [CreateResultsMetaDataRequest](#armonik-api-grpc-v1-agent-CreateResultsMetaDataRequest) | [CreateResultsMetaDataResponse](#armonik-api-grpc-v1-agent-CreateResultsMetaDataResponse) | Create the metadata of multiple results at once Data have to be uploaded separately |
| CreateResults | [CreateResultsRequest](#armonik-api-grpc-v1-agent-CreateResultsRequest) | [CreateResultsResponse](#armonik-api-grpc-v1-agent-CreateResultsResponse) | Create one result with data included in the request |
| NotifyResultData | [NotifyResultDataRequest](#armonik-api-grpc-v1-agent-NotifyResultDataRequest) | [NotifyResultDataResponse](#armonik-api-grpc-v1-agent-NotifyResultDataResponse) | Notify Agent that a data file representing the Result to upload is available in the shared folder The name of the file should be the result id Blocks until data are stored in Object Storage |
| SubmitTasks | [SubmitTasksRequest](#armonik-api-grpc-v1-agent-SubmitTasksRequest) | [SubmitTasksResponse](#armonik-api-grpc-v1-agent-SubmitTasksResponse) | Create tasks metadata and submit task for processing. |
| GetResourceData | [DataRequest](#armonik-api-grpc-v1-agent-DataRequest) | [DataResponse](#armonik-api-grpc-v1-agent-DataResponse) | Retrieve Resource Data from the Agent Data is stored in the shared folder between Agent and Worker as a file with the result id as name Blocks until data are available in the shared folder |
| GetCommonData | [DataRequest](#armonik-api-grpc-v1-agent-DataRequest) | [DataResponse](#armonik-api-grpc-v1-agent-DataResponse) | Retrieve Resource Data from the Agent Data is stored in the shared folder between Agent and Worker as a file with the result id as name Blocks until data are available in the shared folder |
| GetDirectData | [DataRequest](#armonik-api-grpc-v1-agent-DataRequest) | [DataResponse](#armonik-api-grpc-v1-agent-DataResponse) | Retrieve Resource Data from the Agent Data is stored in the shared folder between Agent and Worker as a file with the result id as name Blocks until data are available in the shared folder |

 



<a id="applications_common-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## applications_common.proto
Messages describing applications and associated requests and responses.


<a id="armonik-api-grpc-v1-applications-ApplicationRaw"></a>

### ApplicationRaw
A raw application object.

Used when a list of applications is requested.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| name | [string](#string) |  | Application name. |
| version | [string](#string) |  | Application version. |
| namespace | [string](#string) |  | Application namespace used in the excecuted class. |
| service | [string](#string) |  | Application service used in the excecuted class. |






<a id="armonik-api-grpc-v1-applications-ListApplicationsRequest"></a>

### ListApplicationsRequest
Request to list applications.

Use pagination, filtering and sorting.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| page | [int32](#int32) |  | The page number. Start at 0. |
| page_size | [int32](#int32) |  | Number of items per page. |
| filters | [Filters](#armonik-api-grpc-v1-applications-Filters) |  | The filters. |
| sort | [ListApplicationsRequest.Sort](#armonik-api-grpc-v1-applications-ListApplicationsRequest-Sort) |  | The sort.

Must be set for every request. |






<a id="armonik-api-grpc-v1-applications-ListApplicationsRequest-Sort"></a>

### ListApplicationsRequest.Sort
Represents the sort object.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| fields | [ApplicationField](#armonik-api-grpc-v1-applications-ApplicationField) | repeated | Fields to order by. |
| direction | [armonik.api.grpc.v1.sort_direction.SortDirection](#armonik-api-grpc-v1-sort_direction-SortDirection) |  | The order direction. |






<a id="armonik-api-grpc-v1-applications-ListApplicationsResponse"></a>

### ListApplicationsResponse
Response to list applications.

Use pagination, filtering and sorting from the request.
Return a list of raw applications.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| applications | [ApplicationRaw](#armonik-api-grpc-v1-applications-ApplicationRaw) | repeated |  |
| page | [int32](#int32) |  | The current page. Start at 0. |
| page_size | [int32](#int32) |  | Number of items per page. |
| total | [int32](#int32) |  | Total number of items. |





 

 

 

 



<a id="applications_fields-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## applications_fields.proto



<a id="armonik-api-grpc-v1-applications-ApplicationField"></a>

### ApplicationField



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| application_field | [ApplicationRawField](#armonik-api-grpc-v1-applications-ApplicationRawField) |  |  |






<a id="armonik-api-grpc-v1-applications-ApplicationRawField"></a>

### ApplicationRawField
This message is used to wrap the enum in order to facilitate the &#39;oneOf&#39; generation.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| field | [ApplicationRawEnumField](#armonik-api-grpc-v1-applications-ApplicationRawEnumField) |  |  |





 


<a id="armonik-api-grpc-v1-applications-ApplicationRawEnumField"></a>

### ApplicationRawEnumField
Represents every available field in an application.

| Name | Number | Description |
| ---- | ------ | ----------- |
| APPLICATION_RAW_ENUM_FIELD_UNSPECIFIED | 0 | Unspecified |
| APPLICATION_RAW_ENUM_FIELD_NAME | 1 | Application name. |
| APPLICATION_RAW_ENUM_FIELD_VERSION | 2 | Application version. |
| APPLICATION_RAW_ENUM_FIELD_NAMESPACE | 3 | Application namespace. |
| APPLICATION_RAW_ENUM_FIELD_SERVICE | 4 | Application service. |


 

 

 



<a id="applications_filters-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## applications_filters.proto



<a id="armonik-api-grpc-v1-applications-FilterField"></a>

### FilterField



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| field | [ApplicationField](#armonik-api-grpc-v1-applications-ApplicationField) |  |  |
| filter_string | [armonik.api.grpc.v1.FilterString](#armonik-api-grpc-v1-FilterString) |  |  |






<a id="armonik-api-grpc-v1-applications-Filters"></a>

### Filters



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| or | [FiltersAnd](#armonik-api-grpc-v1-applications-FiltersAnd) | repeated |  |






<a id="armonik-api-grpc-v1-applications-FiltersAnd"></a>

### FiltersAnd



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| and | [FilterField](#armonik-api-grpc-v1-applications-FilterField) | repeated |  |





 

 

 

 



<a id="applications_service-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## applications_service.proto
Applications related methods within a service.

 

 

 


<a id="armonik-api-grpc-v1-applications-Applications"></a>

### Applications
Service for handling applications.

| Method Name | Request Type | Response Type | Description |
| ----------- | ------------ | ------------- | ------------|
| ListApplications | [ListApplicationsRequest](#armonik-api-grpc-v1-applications-ListApplicationsRequest) | [ListApplicationsResponse](#armonik-api-grpc-v1-applications-ListApplicationsResponse) | Get a applications list using pagination, filters and sorting; |

 



<a id="auth_common-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## auth_common.proto
Messages describing authentication and associated requests and responses.


<a id="armonik-api-grpc-v1-auth-GetCurrentUserRequest"></a>

### GetCurrentUserRequest
Request to get current user informations.






<a id="armonik-api-grpc-v1-auth-GetCurrentUserResponse"></a>

### GetCurrentUserResponse
Response to get current user informations.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| user | [User](#armonik-api-grpc-v1-auth-User) |  | Return current user. If auth failed, must throw a gRPC error. |






<a id="armonik-api-grpc-v1-auth-User"></a>

### User
A user.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| username | [string](#string) |  | Username. If authentication is disabled, must be set to &#39;Anonymous&#39; |
| roles | [string](#string) | repeated | Roles. If authentication is disabled, must return []. |
| permissions | [string](#string) | repeated | Permissions. If authentication is disabled, must return every permissions. |





 

 

 

 



<a id="auth_service-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## auth_service.proto
Authentication related methods within a service.

 

 

 


<a id="armonik-api-grpc-v1-auth-Authentication"></a>

### Authentication
Service for authentication management.

| Method Name | Request Type | Response Type | Description |
| ----------- | ------------ | ------------- | ------------|
| GetCurrentUser | [GetCurrentUserRequest](#armonik-api-grpc-v1-auth-GetCurrentUserRequest) | [GetCurrentUserResponse](#armonik-api-grpc-v1-auth-GetCurrentUserResponse) | Get current user |

 



<a id="events_common-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## events_common.proto



<a id="armonik-api-grpc-v1-events-EventSubscriptionRequest"></a>

### EventSubscriptionRequest
Request to subscribe to the event stream.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session_id | [string](#string) |  | Id of the session that will be used to subscribe events for. * |
| tasks_filters | [armonik.api.grpc.v1.tasks.Filters](#armonik-api-grpc-v1-tasks-Filters) |  | Filter for task related events. |
| results_filters | [armonik.api.grpc.v1.results.Filters](#armonik-api-grpc-v1-results-Filters) |  | Filter for result related events. |
| returned_events | [EventsEnum](#armonik-api-grpc-v1-events-EventsEnum) | repeated | Filter the type of events to return. Empty means all. |






<a id="armonik-api-grpc-v1-events-EventSubscriptionResponse"></a>

### EventSubscriptionResponse
Response containing the update event.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session_id | [string](#string) |  | Id of the session that will be used to subscribe events for. * |
| task_status_update | [EventSubscriptionResponse.TaskStatusUpdate](#armonik-api-grpc-v1-events-EventSubscriptionResponse-TaskStatusUpdate) |  | An update to the status of a task. * |
| result_status_update | [EventSubscriptionResponse.ResultStatusUpdate](#armonik-api-grpc-v1-events-EventSubscriptionResponse-ResultStatusUpdate) |  | An update to the status of a result. * |
| result_owner_update | [EventSubscriptionResponse.ResultOwnerUpdate](#armonik-api-grpc-v1-events-EventSubscriptionResponse-ResultOwnerUpdate) |  | An update to the owner of a result. * |
| new_task | [EventSubscriptionResponse.NewTask](#armonik-api-grpc-v1-events-EventSubscriptionResponse-NewTask) |  | A new task in ArmoniK. * |
| new_result | [EventSubscriptionResponse.NewResult](#armonik-api-grpc-v1-events-EventSubscriptionResponse-NewResult) |  | A new result in ArmoniK. * |






<a id="armonik-api-grpc-v1-events-EventSubscriptionResponse-NewResult"></a>

### EventSubscriptionResponse.NewResult
Represents the submission of a new result in ArmoniK.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| result_id | [string](#string) |  | The result id. * |
| owner_id | [string](#string) |  | The owner task id. * |
| status | [armonik.api.grpc.v1.result_status.ResultStatus](#armonik-api-grpc-v1-result_status-ResultStatus) |  | The result status. * |






<a id="armonik-api-grpc-v1-events-EventSubscriptionResponse-NewTask"></a>

### EventSubscriptionResponse.NewTask
Represents the submission of a new task in ArmoniK.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task_id | [string](#string) |  | The task id. * |
| payload_id | [string](#string) |  | The payload id. * |
| origin_task_id | [string](#string) |  | The task id before retry. * |
| status | [armonik.api.grpc.v1.task_status.TaskStatus](#armonik-api-grpc-v1-task_status-TaskStatus) |  | The task status. * |
| expected_output_keys | [string](#string) | repeated | The keys of the expected outputs * |
| data_dependencies | [string](#string) | repeated | The keys of the data dependencies. * |
| retry_of_ids | [string](#string) | repeated | The list of retried tasks from the first retry to the current. * |
| parent_task_ids | [string](#string) | repeated | The parent task IDs. A tasks can be a child of another task. * |






<a id="armonik-api-grpc-v1-events-EventSubscriptionResponse-ResultOwnerUpdate"></a>

### EventSubscriptionResponse.ResultOwnerUpdate
Represents an update to the owner task id of a result.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| result_id | [string](#string) |  | The result id. * |
| previous_owner_id | [string](#string) |  | The previous owner id. * |
| current_owner_id | [string](#string) |  | The current owner id. * |






<a id="armonik-api-grpc-v1-events-EventSubscriptionResponse-ResultStatusUpdate"></a>

### EventSubscriptionResponse.ResultStatusUpdate
Represents an update to the status of a result.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| result_id | [string](#string) |  | The result id. * |
| status | [armonik.api.grpc.v1.result_status.ResultStatus](#armonik-api-grpc-v1-result_status-ResultStatus) |  | The result status. * |






<a id="armonik-api-grpc-v1-events-EventSubscriptionResponse-TaskStatusUpdate"></a>

### EventSubscriptionResponse.TaskStatusUpdate
Represents an update to the status of a task.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task_id | [string](#string) |  | The task id. * |
| status | [armonik.api.grpc.v1.task_status.TaskStatus](#armonik-api-grpc-v1-task_status-TaskStatus) |  | The task status. * |





 


<a id="armonik-api-grpc-v1-events-EventsEnum"></a>

### EventsEnum
Represents the events that can be returned in the EventSubscriptionResponse

| Name | Number | Description |
| ---- | ------ | ----------- |
| EVENTS_ENUM_UNSPECIFIED | 0 | Unspecified |
| EVENTS_ENUM_NEW_TASK | 1 | New task |
| EVENTS_ENUM_TASK_STATUS_UPDATE | 2 | Task status update |
| EVENTS_ENUM_NEW_RESULT | 3 | New restult |
| EVENTS_ENUM_RESULT_STATUS_UPDATE | 4 | Result status update |
| EVENTS_ENUM_RESULT_OWNER_UPDATE | 5 | Result owner update |


 

 

 



<a id="events_service-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## events_service.proto
Events subscription related methods within a service.

This service can be used to receive events related to the update of tasks
and results whithin a session.
The endpoint can be called to listen to the modifications of multiple sessions
if needed.

Note: As for now, all the events of a session will be sent whithout filtering.
It is possible that the API will evolve to a more refined way to filter the events
to be received.

 

 

 


<a id="armonik-api-grpc-v1-events-Events"></a>

### Events
Service for subscribing to events representing modifications to ArmoniK result and task data

| Method Name | Request Type | Response Type | Description |
| ----------- | ------------ | ------------- | ------------|
| GetEvents | [EventSubscriptionRequest](#armonik-api-grpc-v1-events-EventSubscriptionRequest) | [EventSubscriptionResponse](#armonik-api-grpc-v1-events-EventSubscriptionResponse) stream | Get events that represents updates of result and tasks data. |

 



<a id="filters_common-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## filters_common.proto



<a id="armonik-api-grpc-v1-FilterArray"></a>

### FilterArray



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| value | [string](#string) |  |  |
| operator | [FilterArrayOperator](#armonik-api-grpc-v1-FilterArrayOperator) |  |  |






<a id="armonik-api-grpc-v1-FilterBoolean"></a>

### FilterBoolean



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| value | [bool](#bool) |  |  |
| operator | [FilterBooleanOperator](#armonik-api-grpc-v1-FilterBooleanOperator) |  |  |






<a id="armonik-api-grpc-v1-FilterDate"></a>

### FilterDate



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| value | [google.protobuf.Timestamp](#google-protobuf-Timestamp) |  |  |
| operator | [FilterDateOperator](#armonik-api-grpc-v1-FilterDateOperator) |  |  |






<a id="armonik-api-grpc-v1-FilterDuration"></a>

### FilterDuration



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| value | [google.protobuf.Duration](#google-protobuf-Duration) |  |  |
| operator | [FilterDurationOperator](#armonik-api-grpc-v1-FilterDurationOperator) |  |  |






<a id="armonik-api-grpc-v1-FilterNumber"></a>

### FilterNumber



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| value | [int64](#int64) |  |  |
| operator | [FilterNumberOperator](#armonik-api-grpc-v1-FilterNumberOperator) |  |  |






<a id="armonik-api-grpc-v1-FilterString"></a>

### FilterString



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| value | [string](#string) |  |  |
| operator | [FilterStringOperator](#armonik-api-grpc-v1-FilterStringOperator) |  |  |





 


<a id="armonik-api-grpc-v1-FilterArrayOperator"></a>

### FilterArrayOperator


| Name | Number | Description |
| ---- | ------ | ----------- |
| FILTER_ARRAY_OPERATOR_CONTAINS | 0 | Contains |
| FILTER_ARRAY_OPERATOR_NOT_CONTAINS | 1 | Not contains |



<a id="armonik-api-grpc-v1-FilterBooleanOperator"></a>

### FilterBooleanOperator


| Name | Number | Description |
| ---- | ------ | ----------- |
| FILTER_BOOLEAN_OPERATOR_IS | 0 | Is |



<a id="armonik-api-grpc-v1-FilterDateOperator"></a>

### FilterDateOperator


| Name | Number | Description |
| ---- | ------ | ----------- |
| FILTER_DATE_OPERATOR_EQUAL | 0 | Equal |
| FILTER_DATE_OPERATOR_NOT_EQUAL | 1 | Not equal |
| FILTER_DATE_OPERATOR_BEFORE | 2 | Before |
| FILTER_DATE_OPERATOR_BEFORE_OR_EQUAL | 3 | Before or equal |
| FILTER_DATE_OPERATOR_AFTER_OR_EQUAL | 4 | After or equal |
| FILTER_DATE_OPERATOR_AFTER | 5 | After |



<a id="armonik-api-grpc-v1-FilterDurationOperator"></a>

### FilterDurationOperator


| Name | Number | Description |
| ---- | ------ | ----------- |
| FILTER_DURATION_OPERATOR_EQUAL | 0 | Equal |
| FILTER_DURATION_OPERATOR_NOT_EQUAL | 1 | Not equal |
| FILTER_DURATION_OPERATOR_SHORTER_THAN | 2 | Shorter than |
| FILTER_DURATION_OPERATOR_SHORTER_THAN_OR_EQUAL | 3 | Shorter than or equal |
| FILTER_DURATION_OPERATOR_LONGER_THAN_OR_EQUAL | 4 | Longer than or equal |
| FILTER_DURATION_OPERATOR_LONGER_THAN | 5 | Longer than |



<a id="armonik-api-grpc-v1-FilterNumberOperator"></a>

### FilterNumberOperator


| Name | Number | Description |
| ---- | ------ | ----------- |
| FILTER_NUMBER_OPERATOR_EQUAL | 0 | Equal |
| FILTER_NUMBER_OPERATOR_NOT_EQUAL | 1 | Not equal |
| FILTER_NUMBER_OPERATOR_LESS_THAN | 2 | Less than |
| FILTER_NUMBER_OPERATOR_LESS_THAN_OR_EQUAL | 3 | Less than or equal |
| FILTER_NUMBER_OPERATOR_GREATER_THAN_OR_EQUAL | 4 | Greater than or equal |
| FILTER_NUMBER_OPERATOR_GREATER_THAN | 5 | Greater than |



<a id="armonik-api-grpc-v1-FilterStatusOperator"></a>

### FilterStatusOperator


| Name | Number | Description |
| ---- | ------ | ----------- |
| FILTER_STATUS_OPERATOR_EQUAL | 0 | Equal |
| FILTER_STATUS_OPERATOR_NOT_EQUAL | 1 | Not equal |



<a id="armonik-api-grpc-v1-FilterStringOperator"></a>

### FilterStringOperator


| Name | Number | Description |
| ---- | ------ | ----------- |
| FILTER_STRING_OPERATOR_EQUAL | 0 | Equal |
| FILTER_STRING_OPERATOR_NOT_EQUAL | 1 | Not equal |
| FILTER_STRING_OPERATOR_CONTAINS | 2 | Contains |
| FILTER_STRING_OPERATOR_NOT_CONTAINS | 3 | Not contains |
| FILTER_STRING_OPERATOR_STARTS_WITH | 4 | Starts with |
| FILTER_STRING_OPERATOR_ENDS_WITH | 5 | Ends with |


 

 

 



<a id="health_checks_common-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## health_checks_common.proto



<a id="armonik-api-grpc-v1-health_checks-CheckHealthRequest"></a>

### CheckHealthRequest
Request to check if all services are healthy






<a id="armonik-api-grpc-v1-health_checks-CheckHealthResponse"></a>

### CheckHealthResponse
Response to check if all services are healthy


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| services | [CheckHealthResponse.ServiceHealth](#armonik-api-grpc-v1-health_checks-CheckHealthResponse-ServiceHealth) | repeated |  |






<a id="armonik-api-grpc-v1-health_checks-CheckHealthResponse-ServiceHealth"></a>

### CheckHealthResponse.ServiceHealth



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| name | [string](#string) |  | Name of the service (e.g. &#34;control_plane&#34;, &#34;database&#34;, &#34;redis&#34;) |
| message | [string](#string) |  |  |
| healthy | [HealthStatusEnum](#armonik-api-grpc-v1-health_checks-HealthStatusEnum) |  |  |





 


<a id="armonik-api-grpc-v1-health_checks-HealthStatusEnum"></a>

### HealthStatusEnum
Represents the available health status

| Name | Number | Description |
| ---- | ------ | ----------- |
| HEALTH_STATUS_ENUM_UNSPECIFIED | 0 | Unspecified |
| HEALTH_STATUS_ENUM_HEALTHY | 1 | Service is working without issues |
| HEALTH_STATUS_ENUM_DEGRADED | 2 | Service has issues but still works |
| HEALTH_STATUS_ENUM_UNHEALTHY | 3 | Service does not work |


 

 

 



<a id="health_checks_service-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## health_checks_service.proto


 

 

 


<a id="armonik-api-grpc-v1-health_checks-HealthChecksService"></a>

### HealthChecksService
The HealthChecksService provides methods to verify the health of the cluster.

| Method Name | Request Type | Response Type | Description |
| ----------- | ------------ | ------------- | ------------|
| CheckHealth | [CheckHealthRequest](#armonik-api-grpc-v1-health_checks-CheckHealthRequest) | [CheckHealthResponse](#armonik-api-grpc-v1-health_checks-CheckHealthResponse) | Checks the health of the cluster. This can be used to verify that the cluster is up and running. |

 



<a id="objects-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## objects.proto



<a id="armonik-api-grpc-v1-Configuration"></a>

### Configuration



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| data_chunk_max_size | [int32](#int32) |  |  |






<a id="armonik-api-grpc-v1-Count"></a>

### Count



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| values | [StatusCount](#armonik-api-grpc-v1-StatusCount) | repeated |  |






<a id="armonik-api-grpc-v1-DataChunk"></a>

### DataChunk



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| data | [bytes](#bytes) |  |  |
| data_complete | [bool](#bool) |  |  |






<a id="armonik-api-grpc-v1-Empty"></a>

### Empty







<a id="armonik-api-grpc-v1-Error"></a>

### Error



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task_status | [task_status.TaskStatus](#armonik-api-grpc-v1-task_status-TaskStatus) |  |  |
| detail | [string](#string) |  |  |






<a id="armonik-api-grpc-v1-InitKeyedDataStream"></a>

### InitKeyedDataStream



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| key | [string](#string) |  |  |
| last_result | [bool](#bool) |  |  |






<a id="armonik-api-grpc-v1-InitTaskRequest"></a>

### InitTaskRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| header | [TaskRequestHeader](#armonik-api-grpc-v1-TaskRequestHeader) |  |  |
| last_task | [bool](#bool) |  |  |






<a id="armonik-api-grpc-v1-Output"></a>

### Output



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| ok | [Empty](#armonik-api-grpc-v1-Empty) |  |  |
| error | [Output.Error](#armonik-api-grpc-v1-Output-Error) |  |  |






<a id="armonik-api-grpc-v1-Output-Error"></a>

### Output.Error



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| details | [string](#string) |  |  |






<a id="armonik-api-grpc-v1-ResultRequest"></a>

### ResultRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session | [string](#string) |  |  |
| result_id | [string](#string) |  |  |






<a id="armonik-api-grpc-v1-Session"></a>

### Session



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| id | [string](#string) |  |  |






<a id="armonik-api-grpc-v1-StatusCount"></a>

### StatusCount



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| status | [task_status.TaskStatus](#armonik-api-grpc-v1-task_status-TaskStatus) |  |  |
| count | [int32](#int32) |  |  |






<a id="armonik-api-grpc-v1-TaskError"></a>

### TaskError



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task_id | [string](#string) |  |  |
| errors | [Error](#armonik-api-grpc-v1-Error) | repeated |  |






<a id="armonik-api-grpc-v1-TaskId"></a>

### TaskId



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session | [string](#string) |  |  |
| task | [string](#string) |  |  |






<a id="armonik-api-grpc-v1-TaskIdList"></a>

### TaskIdList



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task_ids | [string](#string) | repeated |  |






<a id="armonik-api-grpc-v1-TaskIdWithStatus"></a>

### TaskIdWithStatus



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task_id | [TaskId](#armonik-api-grpc-v1-TaskId) |  |  |
| status | [task_status.TaskStatus](#armonik-api-grpc-v1-task_status-TaskStatus) |  |  |






<a id="armonik-api-grpc-v1-TaskList"></a>

### TaskList



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task_ids | [TaskId](#armonik-api-grpc-v1-TaskId) | repeated |  |






<a id="armonik-api-grpc-v1-TaskOptions"></a>

### TaskOptions



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| options | [TaskOptions.OptionsEntry](#armonik-api-grpc-v1-TaskOptions-OptionsEntry) | repeated |  |
| max_duration | [google.protobuf.Duration](#google-protobuf-Duration) |  |  |
| max_retries | [int32](#int32) |  |  |
| priority | [int32](#int32) |  |  |
| partition_id | [string](#string) |  |  |
| application_name | [string](#string) |  |  |
| application_version | [string](#string) |  |  |
| application_namespace | [string](#string) |  |  |
| application_service | [string](#string) |  |  |
| engine_type | [string](#string) |  |  |






<a id="armonik-api-grpc-v1-TaskOptions-OptionsEntry"></a>

### TaskOptions.OptionsEntry



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| key | [string](#string) |  |  |
| value | [string](#string) |  |  |






<a id="armonik-api-grpc-v1-TaskOutputRequest"></a>

### TaskOutputRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session | [string](#string) |  |  |
| task_id | [string](#string) |  |  |






<a id="armonik-api-grpc-v1-TaskRequest"></a>

### TaskRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| expected_output_keys | [string](#string) | repeated | Given names to the expected outputs that will be created implicitly. IDs are returned after task creation |
| data_dependencies | [string](#string) | repeated | IDs of the results that will be used as data dependency. |
| payload | [bytes](#bytes) |  | Content of the payload for the task. |
| payload_name | [string](#string) |  | Name that will be associated to the result created for the payload. Optionnal |






<a id="armonik-api-grpc-v1-TaskRequestHeader"></a>

### TaskRequestHeader



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| expected_output_keys | [string](#string) | repeated | Given names to the expected outputs that will be created implicitly. IDs are returned after task creation |
| data_dependencies | [string](#string) | repeated | IDs of the results that will be used as data dependency. |





 

 

 

 



<a id="partitions_common-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## partitions_common.proto



<a id="armonik-api-grpc-v1-partitions-GetPartitionRequest"></a>

### GetPartitionRequest
Request to get a partition.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| id | [string](#string) |  | The partition ID. |






<a id="armonik-api-grpc-v1-partitions-GetPartitionResponse"></a>

### GetPartitionResponse
Response to get a partition.

Return a raw partition.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| partition | [PartitionRaw](#armonik-api-grpc-v1-partitions-PartitionRaw) |  | The raw partition. |






<a id="armonik-api-grpc-v1-partitions-ListPartitionsRequest"></a>

### ListPartitionsRequest
Request to list partitions.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| page | [int32](#int32) |  | The page number. Start at 0. |
| page_size | [int32](#int32) |  | The number of items per page. |
| filters | [Filters](#armonik-api-grpc-v1-partitions-Filters) |  | The filter. |
| sort | [ListPartitionsRequest.Sort](#armonik-api-grpc-v1-partitions-ListPartitionsRequest-Sort) |  | The sort.

Must be set for every request. |






<a id="armonik-api-grpc-v1-partitions-ListPartitionsRequest-Sort"></a>

### ListPartitionsRequest.Sort
Represents the sort object.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| field | [PartitionField](#armonik-api-grpc-v1-partitions-PartitionField) |  | The field to sort on. |
| direction | [armonik.api.grpc.v1.sort_direction.SortDirection](#armonik-api-grpc-v1-sort_direction-SortDirection) |  | The sort direction. |






<a id="armonik-api-grpc-v1-partitions-ListPartitionsResponse"></a>

### ListPartitionsResponse
Response to list partitions.

Use pagination, filtering and sorting from the request.
Retunr a list of raw partitions.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| partitions | [PartitionRaw](#armonik-api-grpc-v1-partitions-PartitionRaw) | repeated | The list of raw partitions. |
| page | [int32](#int32) |  | The page number. Start at 0. |
| page_size | [int32](#int32) |  | The page size. |
| total | [int32](#int32) |  | The total number of partitions. |






<a id="armonik-api-grpc-v1-partitions-PartitionRaw"></a>

### PartitionRaw
A raw partition object.

Used when a list or a single partition is returned.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| id | [string](#string) |  | The partition ID. |
| parent_partition_ids | [string](#string) | repeated | The parent partition IDs. |
| pod_reserved | [int64](#int64) |  | Whether the partition is reserved for pods. |
| pod_max | [int64](#int64) |  | The maximum number of pods that can be used by sessions using the partition. |
| pod_configuration | [PartitionRaw.PodConfigurationEntry](#armonik-api-grpc-v1-partitions-PartitionRaw-PodConfigurationEntry) | repeated | The pod configuration. |
| preemption_percentage | [int64](#int64) |  | The percentage of the partition that can be preempted. |
| priority | [int64](#int64) |  | The priority of the partition. |






<a id="armonik-api-grpc-v1-partitions-PartitionRaw-PodConfigurationEntry"></a>

### PartitionRaw.PodConfigurationEntry



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| key | [string](#string) |  |  |
| value | [string](#string) |  |  |





 

 

 

 



<a id="partitions_fields-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## partitions_fields.proto



<a id="armonik-api-grpc-v1-partitions-PartitionField"></a>

### PartitionField



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| partition_raw_field | [PartitionRawField](#armonik-api-grpc-v1-partitions-PartitionRawField) |  | The partition raw field. |






<a id="armonik-api-grpc-v1-partitions-PartitionRawField"></a>

### PartitionRawField
This message is used to wrap the enum in order to facilitate the &#39;oneOf&#39; generation.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| field | [PartitionRawEnumField](#armonik-api-grpc-v1-partitions-PartitionRawEnumField) |  |  |





 


<a id="armonik-api-grpc-v1-partitions-PartitionRawEnumField"></a>

### PartitionRawEnumField
Represents every available field in a partition.

| Name | Number | Description |
| ---- | ------ | ----------- |
| PARTITION_RAW_ENUM_FIELD_UNSPECIFIED | 0 | Unspecified. |
| PARTITION_RAW_ENUM_FIELD_ID | 1 | The partition ID. |
| PARTITION_RAW_ENUM_FIELD_PARENT_PARTITION_IDS | 2 | The parent partition IDs. |
| PARTITION_RAW_ENUM_FIELD_POD_RESERVED | 3 | Whether the partition is reserved for pods. |
| PARTITION_RAW_ENUM_FIELD_POD_MAX | 4 | The maximum number of pods that can be used by sessions using the partition. |
| PARTITION_RAW_ENUM_FIELD_PREEMPTION_PERCENTAGE | 5 | The percentage of the partition that can be preempted. |
| PARTITION_RAW_ENUM_FIELD_PRIORITY | 6 | The priority of the partition. |


 

 

 



<a id="partitions_filters-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## partitions_filters.proto



<a id="armonik-api-grpc-v1-partitions-FilterField"></a>

### FilterField



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| field | [PartitionField](#armonik-api-grpc-v1-partitions-PartitionField) |  |  |
| filter_string | [armonik.api.grpc.v1.FilterString](#armonik-api-grpc-v1-FilterString) |  |  |
| filter_number | [armonik.api.grpc.v1.FilterNumber](#armonik-api-grpc-v1-FilterNumber) |  |  |
| filter_boolean | [armonik.api.grpc.v1.FilterBoolean](#armonik-api-grpc-v1-FilterBoolean) |  |  |
| filter_array | [armonik.api.grpc.v1.FilterArray](#armonik-api-grpc-v1-FilterArray) |  |  |






<a id="armonik-api-grpc-v1-partitions-Filters"></a>

### Filters



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| or | [FiltersAnd](#armonik-api-grpc-v1-partitions-FiltersAnd) | repeated |  |






<a id="armonik-api-grpc-v1-partitions-FiltersAnd"></a>

### FiltersAnd



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| and | [FilterField](#armonik-api-grpc-v1-partitions-FilterField) | repeated |  |





 

 

 

 



<a id="partitions_service-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## partitions_service.proto


 

 

 


<a id="armonik-api-grpc-v1-partitions-Partitions"></a>

### Partitions
The PartitionsService provides methods to manage partitions.

| Method Name | Request Type | Response Type | Description |
| ----------- | ------------ | ------------- | ------------|
| ListPartitions | [ListPartitionsRequest](#armonik-api-grpc-v1-partitions-ListPartitionsRequest) | [ListPartitionsResponse](#armonik-api-grpc-v1-partitions-ListPartitionsResponse) | Get a partitions list using pagination, filters and sorting. |
| GetPartition | [GetPartitionRequest](#armonik-api-grpc-v1-partitions-GetPartitionRequest) | [GetPartitionResponse](#armonik-api-grpc-v1-partitions-GetPartitionResponse) | Get a partition by its ID. |

 



<a id="results_common-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## results_common.proto



<a id="armonik-api-grpc-v1-results-CreateResultsMetaDataRequest"></a>

### CreateResultsMetaDataRequest
Request for creating results without data


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| results | [CreateResultsMetaDataRequest.ResultCreate](#armonik-api-grpc-v1-results-CreateResultsMetaDataRequest-ResultCreate) | repeated | The list of results to create. |
| session_id | [string](#string) |  | The session in which create results. |






<a id="armonik-api-grpc-v1-results-CreateResultsMetaDataRequest-ResultCreate"></a>

### CreateResultsMetaDataRequest.ResultCreate
A result to create.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| name | [string](#string) |  | The result name. Given by the client. |
| manual_deletion | [bool](#bool) |  | If the user is responsible for the deletion of the data in the underlying object storage. |






<a id="armonik-api-grpc-v1-results-CreateResultsMetaDataResponse"></a>

### CreateResultsMetaDataResponse
Response for creating results without data


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| results | [ResultRaw](#armonik-api-grpc-v1-results-ResultRaw) | repeated | The list of raw results that were created. |






<a id="armonik-api-grpc-v1-results-CreateResultsRequest"></a>

### CreateResultsRequest
Request for creating results with data


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| results | [CreateResultsRequest.ResultCreate](#armonik-api-grpc-v1-results-CreateResultsRequest-ResultCreate) | repeated | Results to create. |
| session_id | [string](#string) |  | The session in which create results. |






<a id="armonik-api-grpc-v1-results-CreateResultsRequest-ResultCreate"></a>

### CreateResultsRequest.ResultCreate
A result to create.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| name | [string](#string) |  | The result name. Given by the client. |
| data | [bytes](#bytes) |  | The actual data of the result. |
| manual_deletion | [bool](#bool) |  | If the user is responsible for the deletion of the data in the underlying object storage. |






<a id="armonik-api-grpc-v1-results-CreateResultsResponse"></a>

### CreateResultsResponse
Response for creating results without data


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| results | [ResultRaw](#armonik-api-grpc-v1-results-ResultRaw) | repeated | The raw results that were created. |






<a id="armonik-api-grpc-v1-results-DeleteResultsDataRequest"></a>

### DeleteResultsDataRequest
Request deleting data from results results but keeping metadata


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session_id | [string](#string) |  | The session of the results. |
| result_id | [string](#string) | repeated | The ID of the results to delete. |






<a id="armonik-api-grpc-v1-results-DeleteResultsDataResponse"></a>

### DeleteResultsDataResponse
Response deleting data from results results but keeping metadata


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session_id | [string](#string) |  | The session of the results. |
| result_id | [string](#string) | repeated | The ID of the deleted results. |






<a id="armonik-api-grpc-v1-results-DownloadResultDataRequest"></a>

### DownloadResultDataRequest
Request for getting a result


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session_id | [string](#string) |  | The session of the result. |
| result_id | [string](#string) |  | The ID of the result. |






<a id="armonik-api-grpc-v1-results-DownloadResultDataResponse"></a>

### DownloadResultDataResponse
Response for creating results without data


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| data_chunk | [bytes](#bytes) |  | The possible messages that constitute a UploadResultDataRequest Get the data chunks of the result

* A chunk of data. |






<a id="armonik-api-grpc-v1-results-GetOwnerTaskIdRequest"></a>

### GetOwnerTaskIdRequest
Request for getting the id of the task that should create this result


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session_id | [string](#string) |  | The session ID. |
| result_id | [string](#string) | repeated | The list of result ID/name. |






<a id="armonik-api-grpc-v1-results-GetOwnerTaskIdResponse"></a>

### GetOwnerTaskIdResponse
Response for getting the id of the task that should create this result


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| result_task | [GetOwnerTaskIdResponse.MapResultTask](#armonik-api-grpc-v1-results-GetOwnerTaskIdResponse-MapResultTask) | repeated |  |
| session_id | [string](#string) |  | The session ID. |






<a id="armonik-api-grpc-v1-results-GetOwnerTaskIdResponse-MapResultTask"></a>

### GetOwnerTaskIdResponse.MapResultTask



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| result_id | [string](#string) |  | The result ID/name. |
| task_id | [string](#string) |  | The owner task ID associated to the result. |






<a id="armonik-api-grpc-v1-results-GetResultRequest"></a>

### GetResultRequest
Request to get an result.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| result_id | [string](#string) |  | Result id. Must fail when name is empty. * |






<a id="armonik-api-grpc-v1-results-GetResultResponse"></a>

### GetResultResponse
Response to get an result.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| result | [ResultRaw](#armonik-api-grpc-v1-results-ResultRaw) |  | The result. |






<a id="armonik-api-grpc-v1-results-ImportResultsDataRequest"></a>

### ImportResultsDataRequest
Request importing existing data from the object storage


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session_id | [string](#string) |  | The session of the results. |
| results | [ImportResultsDataRequest.ResultOpaqueId](#armonik-api-grpc-v1-results-ImportResultsDataRequest-ResultOpaqueId) | repeated | The opaque ids associated to the results to import. |






<a id="armonik-api-grpc-v1-results-ImportResultsDataRequest-ResultOpaqueId"></a>

### ImportResultsDataRequest.ResultOpaqueId



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| result_id | [string](#string) |  | The ID of the result. |
| opaque_id | [bytes](#bytes) |  | ID of the data in the underlying object storage. |






<a id="armonik-api-grpc-v1-results-ImportResultsDataResponse"></a>

### ImportResultsDataResponse
Response importing existing data from the object storage


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| results | [ResultRaw](#armonik-api-grpc-v1-results-ResultRaw) | repeated | The updated results. |






<a id="armonik-api-grpc-v1-results-ListResultsRequest"></a>

### ListResultsRequest
Request to list results.

Use pagination, filtering and sorting.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| page | [int32](#int32) |  | The page number. Start at 0. |
| page_size | [int32](#int32) |  | The page size. |
| filters | [Filters](#armonik-api-grpc-v1-results-Filters) |  | The filters. |
| sort | [ListResultsRequest.Sort](#armonik-api-grpc-v1-results-ListResultsRequest-Sort) |  | The sort.

Must be set for every request. |






<a id="armonik-api-grpc-v1-results-ListResultsRequest-Sort"></a>

### ListResultsRequest.Sort
Represents the sort object.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| field | [ResultField](#armonik-api-grpc-v1-results-ResultField) |  | The field to use to sort results. |
| direction | [armonik.api.grpc.v1.sort_direction.SortDirection](#armonik-api-grpc-v1-sort_direction-SortDirection) |  | The direction to use to sort results. |






<a id="armonik-api-grpc-v1-results-ListResultsResponse"></a>

### ListResultsResponse
Response to list results.

Use pagination, filtering and sorting from the request.
Retunr a list of raw results.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| results | [ResultRaw](#armonik-api-grpc-v1-results-ResultRaw) | repeated | The list of raw results. |
| page | [int32](#int32) |  | The page number. Start at 0. |
| page_size | [int32](#int32) |  | The page size. |
| total | [int32](#int32) |  | The total number of results. |






<a id="armonik-api-grpc-v1-results-ResultRaw"></a>

### ResultRaw
A raw result object.

Used when a list or a single result is returned.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session_id | [string](#string) |  | The session ID. |
| name | [string](#string) |  | The result name. Given by the client. |
| owner_task_id | [string](#string) |  | The owner task ID. |
| status | [armonik.api.grpc.v1.result_status.ResultStatus](#armonik-api-grpc-v1-result_status-ResultStatus) |  | The result status. |
| created_at | [google.protobuf.Timestamp](#google-protobuf-Timestamp) |  | The result creation date. |
| completed_at | [google.protobuf.Timestamp](#google-protobuf-Timestamp) |  | The result completion date. |
| result_id | [string](#string) |  | The result ID. Uniquely generated by the server. |
| size | [int64](#int64) |  | The size of the Result Data. |
| created_by | [string](#string) |  | The ID of the Task that as submitted this result. |
| opaque_id | [bytes](#bytes) |  | ID of the data in the underlying object storage. |
| manual_deletion | [bool](#bool) |  | If the user is responsible for the deletion of the data in the underlying object storage. |






<a id="armonik-api-grpc-v1-results-ResultsServiceConfigurationResponse"></a>

### ResultsServiceConfigurationResponse
Response for obtaining results service configuration


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| data_chunk_max_size | [int32](#int32) |  | Maximum size supported by a data chunk for the result service |






<a id="armonik-api-grpc-v1-results-UploadResultDataRequest"></a>

### UploadResultDataRequest
Request for uploading results data through stream.
Data must be sent in multiple chunks.
Only one result can be uploaded.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| id | [UploadResultDataRequest.ResultIdentifier](#armonik-api-grpc-v1-results-UploadResultDataRequest-ResultIdentifier) |  | The identifier of the result to which add data. |
| data_chunk | [bytes](#bytes) |  | A chunk of data. |






<a id="armonik-api-grpc-v1-results-UploadResultDataRequest-ResultIdentifier"></a>

### UploadResultDataRequest.ResultIdentifier
The metadata to identify the result to update.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session_id | [string](#string) |  | The session of the result. |
| result_id | [string](#string) |  | The ID of the result. |






<a id="armonik-api-grpc-v1-results-UploadResultDataResponse"></a>

### UploadResultDataResponse
Response for creating results without data


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| result | [ResultRaw](#armonik-api-grpc-v1-results-ResultRaw) |  | The metadata of the updated result that was updated. |






<a id="armonik-api-grpc-v1-results-WatchResultRequest"></a>

### WatchResultRequest
Request to watch result states
It contains the list of result ids you want to watch
  and some options to filter out some events.
Chunking is achieved by sending multiple messages with different result ids.
It is the responsability of the client to chunk the messages properly and avoid messages too large.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| fetch_statuses | [armonik.api.grpc.v1.result_status.ResultStatus](#armonik-api-grpc-v1-result_status-ResultStatus) | repeated | list of statuses to check results against for the initial fetch |
| watch_statuses | [armonik.api.grpc.v1.result_status.ResultStatus](#armonik-api-grpc-v1-result_status-ResultStatus) | repeated | list of statuses to check results against for the watch |
| result_ids | [string](#string) | repeated | result ids to fetch/watch |






<a id="armonik-api-grpc-v1-results-WatchResultResponse"></a>

### WatchResultResponse
List of Result statuses
Result Ids are grouped by status. One message contains result Ids that have the same status.
Chunking is achieved by receiving several messages with the same status and the list of ids in multiple parts.
As chunking is implicit, there is no way to distinguish between chunked messages and actually separate messages.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| status | [armonik.api.grpc.v1.result_status.ResultStatus](#armonik-api-grpc-v1-result_status-ResultStatus) |  | Status of the results |
| result_ids | [string](#string) | repeated | List of result ids that triggered the event |





 

 

 

 



<a id="results_fields-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## results_fields.proto



<a id="armonik-api-grpc-v1-results-ResultField"></a>

### ResultField



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| result_raw_field | [ResultRawField](#armonik-api-grpc-v1-results-ResultRawField) |  | The field to use to sort results. |






<a id="armonik-api-grpc-v1-results-ResultRawField"></a>

### ResultRawField
This message is used to wrap the enum in order to facilitate the &#39;oneOf&#39; generation.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| field | [ResultRawEnumField](#armonik-api-grpc-v1-results-ResultRawEnumField) |  |  |





 


<a id="armonik-api-grpc-v1-results-ResultRawEnumField"></a>

### ResultRawEnumField
Represents every available field in a result.

| Name | Number | Description |
| ---- | ------ | ----------- |
| RESULT_RAW_ENUM_FIELD_UNSPECIFIED | 0 | The default value. |
| RESULT_RAW_ENUM_FIELD_SESSION_ID | 1 | The session ID. |
| RESULT_RAW_ENUM_FIELD_NAME | 2 | The result name. |
| RESULT_RAW_ENUM_FIELD_OWNER_TASK_ID | 3 | The owner task ID. |
| RESULT_RAW_ENUM_FIELD_STATUS | 4 | The result status. |
| RESULT_RAW_ENUM_FIELD_CREATED_AT | 5 | The result creation date. |
| RESULT_RAW_ENUM_FIELD_COMPLETED_AT | 6 | The result completion date. |
| RESULT_RAW_ENUM_FIELD_RESULT_ID | 7 | The result ID. |
| RESULT_RAW_ENUM_FIELD_SIZE | 8 | The size of the result. |
| RESULT_RAW_ENUM_FIELD_CREATED_BY | 9 | The size of the result. |
| RESULT_RAW_ENUM_FIELD_OPAQUE_ID | 10 | The ID of the data in the underlying object storage. |
| RESULT_RAW_ENUM_FIELD_MANUAL_DELETION | 11 | If the user is responsible for the deletion of the data in the underlying object storage. |


 

 

 



<a id="results_filters-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## results_filters.proto



<a id="armonik-api-grpc-v1-results-FilterField"></a>

### FilterField



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| field | [ResultField](#armonik-api-grpc-v1-results-ResultField) |  |  |
| filter_string | [armonik.api.grpc.v1.FilterString](#armonik-api-grpc-v1-FilterString) |  |  |
| filter_date | [armonik.api.grpc.v1.FilterDate](#armonik-api-grpc-v1-FilterDate) |  |  |
| filter_array | [armonik.api.grpc.v1.FilterArray](#armonik-api-grpc-v1-FilterArray) |  |  |
| filter_status | [FilterStatus](#armonik-api-grpc-v1-results-FilterStatus) |  |  |
| filter_number | [armonik.api.grpc.v1.FilterNumber](#armonik-api-grpc-v1-FilterNumber) |  |  |






<a id="armonik-api-grpc-v1-results-FilterStatus"></a>

### FilterStatus



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| value | [armonik.api.grpc.v1.result_status.ResultStatus](#armonik-api-grpc-v1-result_status-ResultStatus) |  |  |
| operator | [armonik.api.grpc.v1.FilterStatusOperator](#armonik-api-grpc-v1-FilterStatusOperator) |  |  |






<a id="armonik-api-grpc-v1-results-Filters"></a>

### Filters



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| or | [FiltersAnd](#armonik-api-grpc-v1-results-FiltersAnd) | repeated |  |






<a id="armonik-api-grpc-v1-results-FiltersAnd"></a>

### FiltersAnd



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| and | [FilterField](#armonik-api-grpc-v1-results-FilterField) | repeated |  |





 

 

 

 



<a id="results_service-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## results_service.proto


 

 

 


<a id="armonik-api-grpc-v1-results-Results"></a>

### Results
The ResultsService provides methods for interacting with results

| Method Name | Request Type | Response Type | Description |
| ----------- | ------------ | ------------- | ------------|
| ListResults | [ListResultsRequest](#armonik-api-grpc-v1-results-ListResultsRequest) | [ListResultsResponse](#armonik-api-grpc-v1-results-ListResultsResponse) | Get a results list using pagination, filters and sorting |
| GetResult | [GetResultRequest](#armonik-api-grpc-v1-results-GetResultRequest) | [GetResultResponse](#armonik-api-grpc-v1-results-GetResultResponse) | Get a result by id. |
| GetOwnerTaskId | [GetOwnerTaskIdRequest](#armonik-api-grpc-v1-results-GetOwnerTaskIdRequest) | [GetOwnerTaskIdResponse](#armonik-api-grpc-v1-results-GetOwnerTaskIdResponse) | Get the id of the task that should produce the result |
| CreateResultsMetaData | [CreateResultsMetaDataRequest](#armonik-api-grpc-v1-results-CreateResultsMetaDataRequest) | [CreateResultsMetaDataResponse](#armonik-api-grpc-v1-results-CreateResultsMetaDataResponse) | Create the metadata of multiple results at once Data have to be uploaded separately |
| CreateResults | [CreateResultsRequest](#armonik-api-grpc-v1-results-CreateResultsRequest) | [CreateResultsResponse](#armonik-api-grpc-v1-results-CreateResultsResponse) | Create one result with data included in the request |
| UploadResultData | [UploadResultDataRequest](#armonik-api-grpc-v1-results-UploadResultDataRequest) stream | [UploadResultDataResponse](#armonik-api-grpc-v1-results-UploadResultDataResponse) | Upload data for result with stream |
| DownloadResultData | [DownloadResultDataRequest](#armonik-api-grpc-v1-results-DownloadResultDataRequest) | [DownloadResultDataResponse](#armonik-api-grpc-v1-results-DownloadResultDataResponse) stream | Retrieve data |
| DeleteResultsData | [DeleteResultsDataRequest](#armonik-api-grpc-v1-results-DeleteResultsDataRequest) | [DeleteResultsDataResponse](#armonik-api-grpc-v1-results-DeleteResultsDataResponse) | Delete data from multiple results |
| ImportResultsData | [ImportResultsDataRequest](#armonik-api-grpc-v1-results-ImportResultsDataRequest) | [ImportResultsDataResponse](#armonik-api-grpc-v1-results-ImportResultsDataResponse) | Import existing data from the object storage into existing results |
| GetServiceConfiguration | [.armonik.api.grpc.v1.Empty](#armonik-api-grpc-v1-Empty) | [ResultsServiceConfigurationResponse](#armonik-api-grpc-v1-results-ResultsServiceConfigurationResponse) | Get the configuration of the service |
| WatchResults | [WatchResultRequest](#armonik-api-grpc-v1-results-WatchResultRequest) stream | [WatchResultResponse](#armonik-api-grpc-v1-results-WatchResultResponse) stream | This endpoint allows a user to watch a list of results and be notified when there is any change. The user sends the list of ids they want to watch. The submitter will then send the statuses for all requested ids immediately and keep the stream open. Ids not present in DB will be returned at that time with the special state NOTFOUND. The submitter will send updates to the client via the opened stream. Any reply can be implicitely chunked if there are too many event to report at the same time (or for the first reply). It is possible to filter out specific statuses from events. |

 



<a id="result_status-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## result_status.proto


 


<a id="armonik-api-grpc-v1-result_status-ResultStatus"></a>

### ResultStatus


| Name | Number | Description |
| ---- | ------ | ----------- |
| RESULT_STATUS_UNSPECIFIED | 0 | Result is in an unspecified state. |
| RESULT_STATUS_CREATED | 1 | Result is created and task is created, submitted or dispatched. |
| RESULT_STATUS_COMPLETED | 2 | Result is completed with a completed task. |
| RESULT_STATUS_ABORTED | 3 | Result is aborted. |
| RESULT_STATUS_DELETED | 4 | Result is completed, but data has been deleted from object storage. |
| RESULT_STATUS_NOTFOUND | 127 | NOTFOUND is encoded as 127 to make it small while still leaving enough room for future status extensions

see https://developers.google.com/protocol-buffers/docs/proto3#enum |


 

 

 



<a id="sessions_common-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## sessions_common.proto
Messages describing sessions and associated requests and responses.


<a id="armonik-api-grpc-v1-sessions-CancelSessionRequest"></a>

### CancelSessionRequest
Request for cancelling a single session.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session_id | [string](#string) |  | The session ID. |






<a id="armonik-api-grpc-v1-sessions-CancelSessionResponse"></a>

### CancelSessionResponse
Response for cancelling a single session.

Return a raw session.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session | [SessionRaw](#armonik-api-grpc-v1-sessions-SessionRaw) |  | The session. |






<a id="armonik-api-grpc-v1-sessions-CloseSessionRequest"></a>

### CloseSessionRequest
Request for closing a single session.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session_id | [string](#string) |  | The session ID. |






<a id="armonik-api-grpc-v1-sessions-CloseSessionResponse"></a>

### CloseSessionResponse
Response for closing a single session.

Return a raw session.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session | [SessionRaw](#armonik-api-grpc-v1-sessions-SessionRaw) |  | The session. |






<a id="armonik-api-grpc-v1-sessions-CreateSessionReply"></a>

### CreateSessionReply
Reply after session creation.
We have this reply in case of success.
When the session creation is not successful, there is an rpc exception.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session_id | [string](#string) |  | Session id of the created session if successful |






<a id="armonik-api-grpc-v1-sessions-CreateSessionRequest"></a>

### CreateSessionRequest
Request for creating session.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| default_task_option | [armonik.api.grpc.v1.TaskOptions](#armonik-api-grpc-v1-TaskOptions) |  | Default tasks options for tasks in the session |
| partition_ids | [string](#string) | repeated | List of partitions allowed during the session |






<a id="armonik-api-grpc-v1-sessions-DeleteSessionRequest"></a>

### DeleteSessionRequest
Request for deleting a single session.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session_id | [string](#string) |  | The session ID. |






<a id="armonik-api-grpc-v1-sessions-DeleteSessionResponse"></a>

### DeleteSessionResponse
Response for deleting a single session.

Return a raw session.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session | [SessionRaw](#armonik-api-grpc-v1-sessions-SessionRaw) |  | The session. |






<a id="armonik-api-grpc-v1-sessions-GetSessionRequest"></a>

### GetSessionRequest
Request for getting a single session.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session_id | [string](#string) |  | The session ID. |






<a id="armonik-api-grpc-v1-sessions-GetSessionResponse"></a>

### GetSessionResponse
Response for getting a single session.

Return a raw session.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session | [SessionRaw](#armonik-api-grpc-v1-sessions-SessionRaw) |  | The session. |






<a id="armonik-api-grpc-v1-sessions-ListSessionsRequest"></a>

### ListSessionsRequest
Request to list sessions.

Use pagination, filtering and sorting.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| page | [int32](#int32) |  | The page number. Start at 0. |
| page_size | [int32](#int32) |  | The page size. |
| filters | [Filters](#armonik-api-grpc-v1-sessions-Filters) |  | The filters. |
| sort | [ListSessionsRequest.Sort](#armonik-api-grpc-v1-sessions-ListSessionsRequest-Sort) |  | The sort.

Must be set for every request. |
| with_task_options | [bool](#bool) |  | Flag to tell if server must return task options in summary sessions |






<a id="armonik-api-grpc-v1-sessions-ListSessionsRequest-Sort"></a>

### ListSessionsRequest.Sort
Represents the sort object.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| field | [SessionField](#armonik-api-grpc-v1-sessions-SessionField) |  | The field to sort on. |
| direction | [armonik.api.grpc.v1.sort_direction.SortDirection](#armonik-api-grpc-v1-sort_direction-SortDirection) |  | The sort direction. |






<a id="armonik-api-grpc-v1-sessions-ListSessionsResponse"></a>

### ListSessionsResponse
Response to list sessions.

Use pagination, filtering and sorting from the request.
Return a list of summary sessions.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| sessions | [SessionRaw](#armonik-api-grpc-v1-sessions-SessionRaw) | repeated | The list of sessions. |
| page | [int32](#int32) |  | The current page. Start at 0. |
| page_size | [int32](#int32) |  | The page size. |
| total | [int32](#int32) |  | The total number of sessions. |






<a id="armonik-api-grpc-v1-sessions-PauseSessionRequest"></a>

### PauseSessionRequest
Request for pausing a single session.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session_id | [string](#string) |  | The session ID. |






<a id="armonik-api-grpc-v1-sessions-PauseSessionResponse"></a>

### PauseSessionResponse
Response for pausing a single session.

Return a raw session.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session | [SessionRaw](#armonik-api-grpc-v1-sessions-SessionRaw) |  | The session. |






<a id="armonik-api-grpc-v1-sessions-PurgeSessionRequest"></a>

### PurgeSessionRequest
Request for purging a single session.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session_id | [string](#string) |  | The session ID. |






<a id="armonik-api-grpc-v1-sessions-PurgeSessionResponse"></a>

### PurgeSessionResponse
Response for purging a single session.

Return a raw session.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session | [SessionRaw](#armonik-api-grpc-v1-sessions-SessionRaw) |  | The session. |






<a id="armonik-api-grpc-v1-sessions-ResumeSessionRequest"></a>

### ResumeSessionRequest
Request for resuming a single session.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session_id | [string](#string) |  | The session ID. |






<a id="armonik-api-grpc-v1-sessions-ResumeSessionResponse"></a>

### ResumeSessionResponse
Response for resuming a single session.

Return a raw session.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session | [SessionRaw](#armonik-api-grpc-v1-sessions-SessionRaw) |  | The session. |






<a id="armonik-api-grpc-v1-sessions-SessionRaw"></a>

### SessionRaw
A raw session object.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session_id | [string](#string) |  | The session ID. |
| status | [armonik.api.grpc.v1.session_status.SessionStatus](#armonik-api-grpc-v1-session_status-SessionStatus) |  | The session status. |
| client_submission | [bool](#bool) |  | Whether clients can submit tasks in the session. |
| worker_submission | [bool](#bool) |  | Whether workers can submit tasks in the session. |
| partition_ids | [string](#string) | repeated | The partition IDs. |
| options | [armonik.api.grpc.v1.TaskOptions](#armonik-api-grpc-v1-TaskOptions) |  | The task options. In fact, these are used as default value in child tasks. |
| created_at | [google.protobuf.Timestamp](#google-protobuf-Timestamp) |  | The creation date. |
| cancelled_at | [google.protobuf.Timestamp](#google-protobuf-Timestamp) |  | The cancellation date. Only set when status is &#39;cancelled&#39;. |
| closed_at | [google.protobuf.Timestamp](#google-protobuf-Timestamp) |  | The closure date. Only set when status is &#39;closed&#39;. |
| purged_at | [google.protobuf.Timestamp](#google-protobuf-Timestamp) |  | The purge date. Only set when status is &#39;purged&#39;. |
| deleted_at | [google.protobuf.Timestamp](#google-protobuf-Timestamp) |  | The deletion date. Only set when status is &#39;deleted&#39;. |
| duration | [google.protobuf.Duration](#google-protobuf-Duration) |  | The duration. Only set when status is &#39;cancelled&#39; and &#39;closed&#39;. |






<a id="armonik-api-grpc-v1-sessions-StopSubmissionRequest"></a>

### StopSubmissionRequest
Request for stopping new tasks submissions from clients or workers in the given session.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session_id | [string](#string) |  | The session ID. |
| client | [bool](#bool) |  | Whether to stop client submission. |
| worker | [bool](#bool) |  | Whether to stop worker submission. |






<a id="armonik-api-grpc-v1-sessions-StopSubmissionResponse"></a>

### StopSubmissionResponse
Response for stopping new tasks submissions from clients or workers in the given session.

Return a raw session.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session | [SessionRaw](#armonik-api-grpc-v1-sessions-SessionRaw) |  | The session. |





 

 

 

 



<a id="sessions_fields-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## sessions_fields.proto



<a id="armonik-api-grpc-v1-sessions-SessionField"></a>

### SessionField



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session_raw_field | [SessionRawField](#armonik-api-grpc-v1-sessions-SessionRawField) |  |  |
| task_option_field | [TaskOptionField](#armonik-api-grpc-v1-sessions-TaskOptionField) |  | The task option field. |
| task_option_generic_field | [TaskOptionGenericField](#armonik-api-grpc-v1-sessions-TaskOptionGenericField) |  | The task option generic field. |






<a id="armonik-api-grpc-v1-sessions-SessionRawField"></a>

### SessionRawField



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| field | [SessionRawEnumField](#armonik-api-grpc-v1-sessions-SessionRawEnumField) |  |  |






<a id="armonik-api-grpc-v1-sessions-TaskOptionField"></a>

### TaskOptionField
This message is used to wrap the enum in order to facilitate the &#39;oneOf&#39; generation.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| field | [TaskOptionEnumField](#armonik-api-grpc-v1-sessions-TaskOptionEnumField) |  |  |






<a id="armonik-api-grpc-v1-sessions-TaskOptionGenericField"></a>

### TaskOptionGenericField
Represents a generic field in a task option.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| field | [string](#string) |  | The generic field. |





 


<a id="armonik-api-grpc-v1-sessions-SessionRawEnumField"></a>

### SessionRawEnumField
Represents every available field in a session raw.

| Name | Number | Description |
| ---- | ------ | ----------- |
| SESSION_RAW_ENUM_FIELD_UNSPECIFIED | 0 |  |
| SESSION_RAW_ENUM_FIELD_SESSION_ID | 1 |  |
| SESSION_RAW_ENUM_FIELD_STATUS | 2 |  |
| SESSION_RAW_ENUM_FIELD_PARTITION_IDS | 3 |  |
| SESSION_RAW_ENUM_FIELD_OPTIONS | 4 |  |
| SESSION_RAW_ENUM_FIELD_CREATED_AT | 5 |  |
| SESSION_RAW_ENUM_FIELD_CANCELLED_AT | 6 |  |
| SESSION_RAW_ENUM_FIELD_CLOSED_AT | 8 |  |
| SESSION_RAW_ENUM_FIELD_PURGED_AT | 9 |  |
| SESSION_RAW_ENUM_FIELD_DELETED_AT | 10 |  |
| SESSION_RAW_ENUM_FIELD_DURATION | 7 |  |
| SESSION_RAW_ENUM_FIELD_WORKER_SUBMISSION | 11 |  |
| SESSION_RAW_ENUM_FIELD_CLIENT_SUBMISSION | 12 |  |



<a id="armonik-api-grpc-v1-sessions-TaskOptionEnumField"></a>

### TaskOptionEnumField
Represents a field in a task option.

| Name | Number | Description |
| ---- | ------ | ----------- |
| TASK_OPTION_ENUM_FIELD_UNSPECIFIED | 0 |  |
| TASK_OPTION_ENUM_FIELD_MAX_DURATION | 1 |  |
| TASK_OPTION_ENUM_FIELD_MAX_RETRIES | 2 |  |
| TASK_OPTION_ENUM_FIELD_PRIORITY | 3 |  |
| TASK_OPTION_ENUM_FIELD_PARTITION_ID | 4 |  |
| TASK_OPTION_ENUM_FIELD_APPLICATION_NAME | 5 |  |
| TASK_OPTION_ENUM_FIELD_APPLICATION_VERSION | 6 |  |
| TASK_OPTION_ENUM_FIELD_APPLICATION_NAMESPACE | 7 |  |
| TASK_OPTION_ENUM_FIELD_APPLICATION_SERVICE | 8 |  |
| TASK_OPTION_ENUM_FIELD_ENGINE_TYPE | 9 |  |


 

 

 



<a id="sessions_filters-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## sessions_filters.proto



<a id="armonik-api-grpc-v1-sessions-FilterField"></a>

### FilterField



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| field | [SessionField](#armonik-api-grpc-v1-sessions-SessionField) |  |  |
| filter_string | [armonik.api.grpc.v1.FilterString](#armonik-api-grpc-v1-FilterString) |  |  |
| filter_number | [armonik.api.grpc.v1.FilterNumber](#armonik-api-grpc-v1-FilterNumber) |  |  |
| filter_boolean | [armonik.api.grpc.v1.FilterBoolean](#armonik-api-grpc-v1-FilterBoolean) |  |  |
| filter_status | [FilterStatus](#armonik-api-grpc-v1-sessions-FilterStatus) |  |  |
| filter_date | [armonik.api.grpc.v1.FilterDate](#armonik-api-grpc-v1-FilterDate) |  |  |
| filter_array | [armonik.api.grpc.v1.FilterArray](#armonik-api-grpc-v1-FilterArray) |  |  |
| filter_duration | [armonik.api.grpc.v1.FilterDuration](#armonik-api-grpc-v1-FilterDuration) |  |  |






<a id="armonik-api-grpc-v1-sessions-FilterStatus"></a>

### FilterStatus



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| value | [armonik.api.grpc.v1.session_status.SessionStatus](#armonik-api-grpc-v1-session_status-SessionStatus) |  |  |
| operator | [armonik.api.grpc.v1.FilterStatusOperator](#armonik-api-grpc-v1-FilterStatusOperator) |  |  |






<a id="armonik-api-grpc-v1-sessions-Filters"></a>

### Filters



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| or | [FiltersAnd](#armonik-api-grpc-v1-sessions-FiltersAnd) | repeated |  |






<a id="armonik-api-grpc-v1-sessions-FiltersAnd"></a>

### FiltersAnd



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| and | [FilterField](#armonik-api-grpc-v1-sessions-FilterField) | repeated |  |





 

 

 

 



<a id="sessions_service-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## sessions_service.proto
Tasks related methods within a service.

 

 

 


<a id="armonik-api-grpc-v1-sessions-Sessions"></a>

### Sessions
Service for handling sessions.

| Method Name | Request Type | Response Type | Description |
| ----------- | ------------ | ------------- | ------------|
| ListSessions | [ListSessionsRequest](#armonik-api-grpc-v1-sessions-ListSessionsRequest) | [ListSessionsResponse](#armonik-api-grpc-v1-sessions-ListSessionsResponse) | Get a sessions list using pagination, filters and sorting. |
| GetSession | [GetSessionRequest](#armonik-api-grpc-v1-sessions-GetSessionRequest) | [GetSessionResponse](#armonik-api-grpc-v1-sessions-GetSessionResponse) | Get a session by its id. |
| CancelSession | [CancelSessionRequest](#armonik-api-grpc-v1-sessions-CancelSessionRequest) | [CancelSessionResponse](#armonik-api-grpc-v1-sessions-CancelSessionResponse) | Cancel a session by its id. |
| CreateSession | [CreateSessionRequest](#armonik-api-grpc-v1-sessions-CreateSessionRequest) | [CreateSessionReply](#armonik-api-grpc-v1-sessions-CreateSessionReply) | Create a session |
| PauseSession | [PauseSessionRequest](#armonik-api-grpc-v1-sessions-PauseSessionRequest) | [PauseSessionResponse](#armonik-api-grpc-v1-sessions-PauseSessionResponse) | Pause a session by its id. |
| ResumeSession | [ResumeSessionRequest](#armonik-api-grpc-v1-sessions-ResumeSessionRequest) | [ResumeSessionResponse](#armonik-api-grpc-v1-sessions-ResumeSessionResponse) | Resume a paused session by its id. |
| CloseSession | [CloseSessionRequest](#armonik-api-grpc-v1-sessions-CloseSessionRequest) | [CloseSessionResponse](#armonik-api-grpc-v1-sessions-CloseSessionResponse) | Close a session by its id.. |
| PurgeSession | [PurgeSessionRequest](#armonik-api-grpc-v1-sessions-PurgeSessionRequest) | [PurgeSessionResponse](#armonik-api-grpc-v1-sessions-PurgeSessionResponse) | Purge a session by its id. Removes Results data. |
| DeleteSession | [DeleteSessionRequest](#armonik-api-grpc-v1-sessions-DeleteSessionRequest) | [DeleteSessionResponse](#armonik-api-grpc-v1-sessions-DeleteSessionResponse) | Delete a session by its id. Removes metadata from Results, Sessions and Tasks associated to the session. |
| StopSubmission | [StopSubmissionRequest](#armonik-api-grpc-v1-sessions-StopSubmissionRequest) | [StopSubmissionResponse](#armonik-api-grpc-v1-sessions-StopSubmissionResponse) | Stops clients and/or workers from submitting new tasks in the given session. |

 



<a id="session_status-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## session_status.proto
Status of a session.

 


<a id="armonik-api-grpc-v1-session_status-SessionStatus"></a>

### SessionStatus
Session status.

| Name | Number | Description |
| ---- | ------ | ----------- |
| SESSION_STATUS_UNSPECIFIED | 0 | Session is in an unknown state. |
| SESSION_STATUS_RUNNING | 1 | Session is open and accepting tasks for execution. |
| SESSION_STATUS_CANCELLED | 2 | Session is cancelled. No more tasks can be submitted and no more tasks will be executed. |
| SESSION_STATUS_PAUSED | 3 | Session is paused. Tasks can be submitted but no more new tasks will be executed. Already running tasks will continue until they finish. |
| SESSION_STATUS_CLOSED | 4 | Session is closed. No more tasks can be submitted and executed. |
| SESSION_STATUS_PURGED | 5 | Session is purged. No more tasks can be submitted and executed. Results data will be deleted. |
| SESSION_STATUS_DELETED | 6 | Session is deleted. No more tasks can be submitted and executed. Sessions, tasks and results metadata associated to the session will be deleted. |


 

 

 



<a id="sort_direction-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## sort_direction.proto


 


<a id="armonik-api-grpc-v1-sort_direction-SortDirection"></a>

### SortDirection
Represents every available sort directions.

| Name | Number | Description |
| ---- | ------ | ----------- |
| SORT_DIRECTION_UNSPECIFIED | 0 | Unspecified. Do not use. |
| SORT_DIRECTION_ASC | 1 | Ascending. |
| SORT_DIRECTION_DESC | 2 | Descending. |


 

 

 



<a id="submitter_common-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## submitter_common.proto



<a id="armonik-api-grpc-v1-submitter-AvailabilityReply"></a>

### AvailabilityReply



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| ok | [armonik.api.grpc.v1.Empty](#armonik-api-grpc-v1-Empty) |  |  |
| error | [armonik.api.grpc.v1.TaskError](#armonik-api-grpc-v1-TaskError) |  |  |
| not_completed_task | [string](#string) |  |  |






<a id="armonik-api-grpc-v1-submitter-CreateLargeTaskRequest"></a>

### CreateLargeTaskRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| init_request | [CreateLargeTaskRequest.InitRequest](#armonik-api-grpc-v1-submitter-CreateLargeTaskRequest-InitRequest) |  |  |
| init_task | [armonik.api.grpc.v1.InitTaskRequest](#armonik-api-grpc-v1-InitTaskRequest) |  |  |
| task_payload | [armonik.api.grpc.v1.DataChunk](#armonik-api-grpc-v1-DataChunk) |  |  |






<a id="armonik-api-grpc-v1-submitter-CreateLargeTaskRequest-InitRequest"></a>

### CreateLargeTaskRequest.InitRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session_id | [string](#string) |  |  |
| task_options | [armonik.api.grpc.v1.TaskOptions](#armonik-api-grpc-v1-TaskOptions) |  |  |






<a id="armonik-api-grpc-v1-submitter-CreateSessionReply"></a>

### CreateSessionReply
Reply after session creation.
We have this reply in case of success.
When the session creation is not successful, there is an rpc exception.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session_id | [string](#string) |  | Session id of the created session if successful |






<a id="armonik-api-grpc-v1-submitter-CreateSessionRequest"></a>

### CreateSessionRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| default_task_option | [armonik.api.grpc.v1.TaskOptions](#armonik-api-grpc-v1-TaskOptions) |  |  |
| partition_ids | [string](#string) | repeated | List of partitions allowed during the session |






<a id="armonik-api-grpc-v1-submitter-CreateSmallTaskRequest"></a>

### CreateSmallTaskRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session_id | [string](#string) |  |  |
| task_options | [armonik.api.grpc.v1.TaskOptions](#armonik-api-grpc-v1-TaskOptions) |  |  |
| task_requests | [armonik.api.grpc.v1.TaskRequest](#armonik-api-grpc-v1-TaskRequest) | repeated |  |






<a id="armonik-api-grpc-v1-submitter-CreateTaskReply"></a>

### CreateTaskReply



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| creation_status_list | [CreateTaskReply.CreationStatusList](#armonik-api-grpc-v1-submitter-CreateTaskReply-CreationStatusList) |  |  |
| error | [string](#string) |  |  |






<a id="armonik-api-grpc-v1-submitter-CreateTaskReply-CreationStatus"></a>

### CreateTaskReply.CreationStatus



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task_info | [CreateTaskReply.TaskInfo](#armonik-api-grpc-v1-submitter-CreateTaskReply-TaskInfo) |  |  |
| error | [string](#string) |  |  |






<a id="armonik-api-grpc-v1-submitter-CreateTaskReply-CreationStatusList"></a>

### CreateTaskReply.CreationStatusList



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| creation_statuses | [CreateTaskReply.CreationStatus](#armonik-api-grpc-v1-submitter-CreateTaskReply-CreationStatus) | repeated |  |






<a id="armonik-api-grpc-v1-submitter-CreateTaskReply-TaskInfo"></a>

### CreateTaskReply.TaskInfo



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task_id | [string](#string) |  | Unique ID of the created task. |
| expected_output_keys | [string](#string) | repeated | Unique ID of the result that will be used as expected output. Results should already exist. |
| data_dependencies | [string](#string) | repeated | Unique ID of the result that will be used as data dependency. Results should already exist. |
| payload_id | [string](#string) |  | Unique ID of the result that will be used as payload. Result associated to the payload is created implicitly. |






<a id="armonik-api-grpc-v1-submitter-GetResultStatusReply"></a>

### GetResultStatusReply



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| id_statuses | [GetResultStatusReply.IdStatus](#armonik-api-grpc-v1-submitter-GetResultStatusReply-IdStatus) | repeated |  |






<a id="armonik-api-grpc-v1-submitter-GetResultStatusReply-IdStatus"></a>

### GetResultStatusReply.IdStatus



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| result_id | [string](#string) |  |  |
| status | [armonik.api.grpc.v1.result_status.ResultStatus](#armonik-api-grpc-v1-result_status-ResultStatus) |  |  |






<a id="armonik-api-grpc-v1-submitter-GetResultStatusRequest"></a>

### GetResultStatusRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| result_ids | [string](#string) | repeated |  |
| session_id | [string](#string) |  |  |






<a id="armonik-api-grpc-v1-submitter-GetTaskStatusReply"></a>

### GetTaskStatusReply



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| id_statuses | [GetTaskStatusReply.IdStatus](#armonik-api-grpc-v1-submitter-GetTaskStatusReply-IdStatus) | repeated |  |






<a id="armonik-api-grpc-v1-submitter-GetTaskStatusReply-IdStatus"></a>

### GetTaskStatusReply.IdStatus



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task_id | [string](#string) |  |  |
| status | [armonik.api.grpc.v1.task_status.TaskStatus](#armonik-api-grpc-v1-task_status-TaskStatus) |  |  |






<a id="armonik-api-grpc-v1-submitter-GetTaskStatusRequest"></a>

### GetTaskStatusRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task_ids | [string](#string) | repeated |  |






<a id="armonik-api-grpc-v1-submitter-ResultReply"></a>

### ResultReply



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| result | [armonik.api.grpc.v1.DataChunk](#armonik-api-grpc-v1-DataChunk) |  |  |
| error | [armonik.api.grpc.v1.TaskError](#armonik-api-grpc-v1-TaskError) |  |  |
| not_completed_task | [string](#string) |  |  |






<a id="armonik-api-grpc-v1-submitter-SessionFilter"></a>

### SessionFilter



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| sessions | [string](#string) | repeated |  |
| included | [SessionFilter.StatusesRequest](#armonik-api-grpc-v1-submitter-SessionFilter-StatusesRequest) |  |  |
| excluded | [SessionFilter.StatusesRequest](#armonik-api-grpc-v1-submitter-SessionFilter-StatusesRequest) |  |  |






<a id="armonik-api-grpc-v1-submitter-SessionFilter-StatusesRequest"></a>

### SessionFilter.StatusesRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| statuses | [armonik.api.grpc.v1.session_status.SessionStatus](#armonik-api-grpc-v1-session_status-SessionStatus) | repeated |  |






<a id="armonik-api-grpc-v1-submitter-SessionIdList"></a>

### SessionIdList



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session_ids | [string](#string) | repeated |  |






<a id="armonik-api-grpc-v1-submitter-SessionList"></a>

### SessionList



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| sessions | [armonik.api.grpc.v1.Session](#armonik-api-grpc-v1-Session) | repeated |  |






<a id="armonik-api-grpc-v1-submitter-TaskFilter"></a>

### TaskFilter



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session | [TaskFilter.IdsRequest](#armonik-api-grpc-v1-submitter-TaskFilter-IdsRequest) |  |  |
| task | [TaskFilter.IdsRequest](#armonik-api-grpc-v1-submitter-TaskFilter-IdsRequest) |  |  |
| included | [TaskFilter.StatusesRequest](#armonik-api-grpc-v1-submitter-TaskFilter-StatusesRequest) |  |  |
| excluded | [TaskFilter.StatusesRequest](#armonik-api-grpc-v1-submitter-TaskFilter-StatusesRequest) |  |  |






<a id="armonik-api-grpc-v1-submitter-TaskFilter-IdsRequest"></a>

### TaskFilter.IdsRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| ids | [string](#string) | repeated |  |






<a id="armonik-api-grpc-v1-submitter-TaskFilter-StatusesRequest"></a>

### TaskFilter.StatusesRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| statuses | [armonik.api.grpc.v1.task_status.TaskStatus](#armonik-api-grpc-v1-task_status-TaskStatus) | repeated |  |






<a id="armonik-api-grpc-v1-submitter-WaitRequest"></a>

### WaitRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| filter | [TaskFilter](#armonik-api-grpc-v1-submitter-TaskFilter) |  |  |
| stop_on_first_task_error | [bool](#bool) |  |  |
| stop_on_first_task_cancellation | [bool](#bool) |  |  |






<a id="armonik-api-grpc-v1-submitter-WatchResultRequest"></a>

### WatchResultRequest
Request to watch result states
It contains the list of result ids you want to watch
  and some options to filter out some events.
Chunking is achieved by sending multiple messages with different result ids.
It is the responsability of the client to chunk the messages properly and avoid messages too large.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| fetch_statuses | [armonik.api.grpc.v1.result_status.ResultStatus](#armonik-api-grpc-v1-result_status-ResultStatus) | repeated | list of statuses to check results against for the initial fetch |
| watch_statuses | [armonik.api.grpc.v1.result_status.ResultStatus](#armonik-api-grpc-v1-result_status-ResultStatus) | repeated | list of statuses to check results against for the watch |
| result_ids | [string](#string) | repeated | result ids to fetch/watch |






<a id="armonik-api-grpc-v1-submitter-WatchResultStream"></a>

### WatchResultStream
List of Result statuses
Result Ids are grouped by status. One message contains result Ids that have the same status.
Chunking is achieved by receiving several messages with the same status and the list of ids in multiple parts.
As chunking is implicit, there is no way to distinguish between chunked messages and actually separate messages.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| status | [armonik.api.grpc.v1.result_status.ResultStatus](#armonik-api-grpc-v1-result_status-ResultStatus) |  | Status of the results |
| result_ids | [string](#string) | repeated | List of result ids that triggered the event |





 

 

 

 



<a id="submitter_service-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## submitter_service.proto


 

 

 


<a id="armonik-api-grpc-v1-submitter-Submitter"></a>

### Submitter


| Method Name | Request Type | Response Type | Description |
| ----------- | ------------ | ------------- | ------------|
| GetServiceConfiguration | [.armonik.api.grpc.v1.Empty](#armonik-api-grpc-v1-Empty) | [.armonik.api.grpc.v1.Configuration](#armonik-api-grpc-v1-Configuration) |  |
| CreateSession | [CreateSessionRequest](#armonik-api-grpc-v1-submitter-CreateSessionRequest) | [CreateSessionReply](#armonik-api-grpc-v1-submitter-CreateSessionReply) |  |
| CancelSession | [.armonik.api.grpc.v1.Session](#armonik-api-grpc-v1-Session) | [.armonik.api.grpc.v1.Empty](#armonik-api-grpc-v1-Empty) |  |
| CreateSmallTasks | [CreateSmallTaskRequest](#armonik-api-grpc-v1-submitter-CreateSmallTaskRequest) | [CreateTaskReply](#armonik-api-grpc-v1-submitter-CreateTaskReply) |  |
| CreateLargeTasks | [CreateLargeTaskRequest](#armonik-api-grpc-v1-submitter-CreateLargeTaskRequest) stream | [CreateTaskReply](#armonik-api-grpc-v1-submitter-CreateTaskReply) |  |
| ListTasks | [TaskFilter](#armonik-api-grpc-v1-submitter-TaskFilter) | [.armonik.api.grpc.v1.TaskIdList](#armonik-api-grpc-v1-TaskIdList) |  |
| ListSessions | [SessionFilter](#armonik-api-grpc-v1-submitter-SessionFilter) | [SessionIdList](#armonik-api-grpc-v1-submitter-SessionIdList) |  |
| CountTasks | [TaskFilter](#armonik-api-grpc-v1-submitter-TaskFilter) | [.armonik.api.grpc.v1.Count](#armonik-api-grpc-v1-Count) |  |
| TryGetResultStream | [.armonik.api.grpc.v1.ResultRequest](#armonik-api-grpc-v1-ResultRequest) | [ResultReply](#armonik-api-grpc-v1-submitter-ResultReply) stream |  |
| TryGetTaskOutput | [.armonik.api.grpc.v1.TaskOutputRequest](#armonik-api-grpc-v1-TaskOutputRequest) | [.armonik.api.grpc.v1.Output](#armonik-api-grpc-v1-Output) |  |
| WaitForAvailability | [.armonik.api.grpc.v1.ResultRequest](#armonik-api-grpc-v1-ResultRequest) | [AvailabilityReply](#armonik-api-grpc-v1-submitter-AvailabilityReply) |  |
| WaitForCompletion | [WaitRequest](#armonik-api-grpc-v1-submitter-WaitRequest) | [.armonik.api.grpc.v1.Count](#armonik-api-grpc-v1-Count) |  |
| CancelTasks | [TaskFilter](#armonik-api-grpc-v1-submitter-TaskFilter) | [.armonik.api.grpc.v1.Empty](#armonik-api-grpc-v1-Empty) |  |
| GetTaskStatus | [GetTaskStatusRequest](#armonik-api-grpc-v1-submitter-GetTaskStatusRequest) | [GetTaskStatusReply](#armonik-api-grpc-v1-submitter-GetTaskStatusReply) |  |
| GetResultStatus | [GetResultStatusRequest](#armonik-api-grpc-v1-submitter-GetResultStatusRequest) | [GetResultStatusReply](#armonik-api-grpc-v1-submitter-GetResultStatusReply) |  |
| WatchResults | [WatchResultRequest](#armonik-api-grpc-v1-submitter-WatchResultRequest) stream | [WatchResultStream](#armonik-api-grpc-v1-submitter-WatchResultStream) stream | This endpoint allows a user to watch a list of results and be notified when there is any change. The user sends the list of ids they want to watch. The submitter will then send the statuses for all requested ids immediately and keep the stream open. Ids not present in DB will be returned at that time with the special state NOTFOUND. The submitter will send updates to the client via the opened stream. Any reply can be implicitely chunked if there are too many event to report at the same time (or for the first reply). It is possible to filter out specific statuses from events. |

 



<a id="tasks_common-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## tasks_common.proto
Messages describing tasks and associated requests and responses.


<a id="armonik-api-grpc-v1-tasks-CancelTasksRequest"></a>

### CancelTasksRequest
Request to cancel one or many tasks


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task_ids | [string](#string) | repeated | Ids of the tasks to cancel |






<a id="armonik-api-grpc-v1-tasks-CancelTasksResponse"></a>

### CancelTasksResponse
Response from canceling one or many tasks


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| tasks | [TaskSummary](#armonik-api-grpc-v1-tasks-TaskSummary) | repeated | Tasks that have been asked to cancel |






<a id="armonik-api-grpc-v1-tasks-CountTasksByStatusRequest"></a>

### CountTasksByStatusRequest
Request to get count from tasks by status


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| filters | [Filters](#armonik-api-grpc-v1-tasks-Filters) |  | The filters. |






<a id="armonik-api-grpc-v1-tasks-CountTasksByStatusResponse"></a>

### CountTasksByStatusResponse
Response to get count from tasks by status


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| status | [armonik.api.grpc.v1.StatusCount](#armonik-api-grpc-v1-StatusCount) | repeated | Number of tasks by status. Expected to have only 1 object by tasks status. |






<a id="armonik-api-grpc-v1-tasks-GetResultIdsRequest"></a>

### GetResultIdsRequest
Request for getting result ids of tasks ids.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task_id | [string](#string) | repeated | The task IDs. |






<a id="armonik-api-grpc-v1-tasks-GetResultIdsResponse"></a>

### GetResultIdsResponse
Response for getting result ids of tasks ids.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task_results | [GetResultIdsResponse.MapTaskResult](#armonik-api-grpc-v1-tasks-GetResultIdsResponse-MapTaskResult) | repeated | The task results. |






<a id="armonik-api-grpc-v1-tasks-GetResultIdsResponse-MapTaskResult"></a>

### GetResultIdsResponse.MapTaskResult
Represents a task result.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task_id | [string](#string) |  | The task ID. |
| result_ids | [string](#string) | repeated | The result IDs. |






<a id="armonik-api-grpc-v1-tasks-GetTaskRequest"></a>

### GetTaskRequest
Request for getting a single task.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task_id | [string](#string) |  | The task ID. |






<a id="armonik-api-grpc-v1-tasks-GetTaskResponse"></a>

### GetTaskResponse
Response for getting a single task.

Return a raw task.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task | [TaskDetailed](#armonik-api-grpc-v1-tasks-TaskDetailed) |  | The task. |






<a id="armonik-api-grpc-v1-tasks-ListTasksDetailedResponse"></a>

### ListTasksDetailedResponse
Response to list tasks.

Use pagination, filtering and sorting from the request.
Return a list of formatted tasks.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| tasks | [TaskDetailed](#armonik-api-grpc-v1-tasks-TaskDetailed) | repeated | The list of tasks. |
| page | [int32](#int32) |  | The page number. Start at 0. |
| page_size | [int32](#int32) |  | The page size. |
| total | [int32](#int32) |  | The total number of tasks. |






<a id="armonik-api-grpc-v1-tasks-ListTasksRequest"></a>

### ListTasksRequest
Request to list tasks.

Use pagination, filtering and sorting.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| page | [int32](#int32) |  | The page number. Start at 0. |
| page_size | [int32](#int32) |  | The page size. |
| filters | [Filters](#armonik-api-grpc-v1-tasks-Filters) |  | The filters. |
| sort | [ListTasksRequest.Sort](#armonik-api-grpc-v1-tasks-ListTasksRequest-Sort) |  | The sort.

Must be set for every request. |
| with_errors | [bool](#bool) |  | Request error message in case of error in task |






<a id="armonik-api-grpc-v1-tasks-ListTasksRequest-Sort"></a>

### ListTasksRequest.Sort
Represents the sort object.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| field | [TaskField](#armonik-api-grpc-v1-tasks-TaskField) |  | The field to sort on. |
| direction | [armonik.api.grpc.v1.sort_direction.SortDirection](#armonik-api-grpc-v1-sort_direction-SortDirection) |  | The sort direction. |






<a id="armonik-api-grpc-v1-tasks-ListTasksResponse"></a>

### ListTasksResponse
Response to list tasks.

Use pagination, filtering and sorting from the request.
Return a list of formatted tasks.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| tasks | [TaskSummary](#armonik-api-grpc-v1-tasks-TaskSummary) | repeated | The list of tasks. |
| page | [int32](#int32) |  | The page number. Start at 0. |
| page_size | [int32](#int32) |  | The page size. |
| total | [int32](#int32) |  | The total number of tasks. |






<a id="armonik-api-grpc-v1-tasks-SubmitTasksRequest"></a>

### SubmitTasksRequest
Request to create tasks.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session_id | [string](#string) |  | The session ID. |
| task_options | [armonik.api.grpc.v1.TaskOptions](#armonik-api-grpc-v1-TaskOptions) |  | The options for the tasks. Each task will have the same. Options are merged with the one from the session. |
| task_creations | [SubmitTasksRequest.TaskCreation](#armonik-api-grpc-v1-tasks-SubmitTasksRequest-TaskCreation) | repeated | Task creation requests. |






<a id="armonik-api-grpc-v1-tasks-SubmitTasksRequest-TaskCreation"></a>

### SubmitTasksRequest.TaskCreation



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| expected_output_keys | [string](#string) | repeated | Unique ID of the results that will be produced by the task. Results should be created using ResultsService. |
| data_dependencies | [string](#string) | repeated | Unique ID of the results that will be used as datadependencies. Results should be created using ResultsService. |
| payload_id | [string](#string) |  | Unique ID of the result that will be used as payload. Result should created using ResultsService. |
| task_options | [armonik.api.grpc.v1.TaskOptions](#armonik-api-grpc-v1-TaskOptions) |  | Optional task options. |






<a id="armonik-api-grpc-v1-tasks-SubmitTasksResponse"></a>

### SubmitTasksResponse
Response to create tasks.

expected_output_ids and data_dependencies must be created through ResultsService.

Remark : this may have to be enriched to a better management of errors but
will the client application be able to manage a missing data dependency or expected output ?


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task_infos | [SubmitTasksResponse.TaskInfo](#armonik-api-grpc-v1-tasks-SubmitTasksResponse-TaskInfo) | repeated | List of task infos if submission successful, else throw gRPC exception. |






<a id="armonik-api-grpc-v1-tasks-SubmitTasksResponse-TaskInfo"></a>

### SubmitTasksResponse.TaskInfo



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task_id | [string](#string) |  | The task ID. |
| expected_output_ids | [string](#string) | repeated | The expected output IDs. A task have expected output IDs. |
| data_dependencies | [string](#string) | repeated | The data dependencies IDs (inputs). A task have data dependencies. |
| payload_id | [string](#string) |  | Unique ID of the result that will be used as payload. Result should created using ResultsService. |






<a id="armonik-api-grpc-v1-tasks-TaskDetailed"></a>

### TaskDetailed
A raw task object.

Used when a single task is returned.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| id | [string](#string) |  | The task ID. |
| session_id | [string](#string) |  | The session ID. A task have only one related session but a session have many tasks. |
| owner_pod_id | [string](#string) |  | The owner pod ID. |
| initial_task_id | [string](#string) |  | The initial task ID. Set when a task is submitted independently of retries. |
| parent_task_ids | [string](#string) | repeated | The parent task IDs. A tasks can be a child of another task. |
| data_dependencies | [string](#string) | repeated | The data dependencies. A task have data dependencies. |
| expected_output_ids | [string](#string) | repeated | The expected output IDs. A task have expected output IDs. |
| retry_of_ids | [string](#string) | repeated | The retry of IDs. When a task fail, retry will use these set of IDs. |
| status | [armonik.api.grpc.v1.task_status.TaskStatus](#armonik-api-grpc-v1-task_status-TaskStatus) |  | The task status. |
| status_message | [string](#string) |  | The status message. |
| options | [armonik.api.grpc.v1.TaskOptions](#armonik-api-grpc-v1-TaskOptions) |  | The task options. |
| created_at | [google.protobuf.Timestamp](#google-protobuf-Timestamp) |  | The task creation date. |
| submitted_at | [google.protobuf.Timestamp](#google-protobuf-Timestamp) |  | The task submission date. |
| received_at | [google.protobuf.Timestamp](#google-protobuf-Timestamp) |  | When the task is received by the agent. |
| acquired_at | [google.protobuf.Timestamp](#google-protobuf-Timestamp) |  | When the task is acquired by the agent. |
| fetched_at | [google.protobuf.Timestamp](#google-protobuf-Timestamp) |  | Task data retrieval end date. |
| started_at | [google.protobuf.Timestamp](#google-protobuf-Timestamp) |  | The task start date. |
| processed_at | [google.protobuf.Timestamp](#google-protobuf-Timestamp) |  | The end of task processing date. |
| ended_at | [google.protobuf.Timestamp](#google-protobuf-Timestamp) |  | The task end date. Also used when task failed. |
| pod_ttl | [google.protobuf.Timestamp](#google-protobuf-Timestamp) |  | The pod TTL (Time To Live). |
| creation_to_end_duration | [google.protobuf.Duration](#google-protobuf-Duration) |  | The task duration. Between the creation date and the end date. |
| processing_to_end_duration | [google.protobuf.Duration](#google-protobuf-Duration) |  | The task calculated duration. Between the start date and the end date. |
| received_to_end_duration | [google.protobuf.Duration](#google-protobuf-Duration) |  | The task calculated duration. Between the received date and the end date. |
| payload_id | [string](#string) |  | The ID of the Result that is used as a payload for this task. |
| created_by | [string](#string) |  | The ID of the Task that as submitted this task empty if none. |
| output | [TaskDetailed.Output](#armonik-api-grpc-v1-tasks-TaskDetailed-Output) |  | The task output. |
| pod_hostname | [string](#string) |  | The hostname of the container running the task. |






<a id="armonik-api-grpc-v1-tasks-TaskDetailed-Output"></a>

### TaskDetailed.Output
Represents the task output.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| success | [bool](#bool) |  | To know if a task have failed or succeed. |
| error | [string](#string) |  | The error message. Only set if task have failed. |






<a id="armonik-api-grpc-v1-tasks-TaskSummary"></a>

### TaskSummary
A summary task object.

It contains only a subset of the fields from the underlying task object.
Used when a list of tasks are returned.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| id | [string](#string) |  | The task ID. |
| session_id | [string](#string) |  | The session ID. A task have only one related session but a session have many tasks. |
| owner_pod_id | [string](#string) |  | The owner pod ID. |
| initial_task_id | [string](#string) |  | The initial task ID. Set when a task is submitted independently of retries. |
| count_parent_task_ids | [int64](#int64) |  | Count the parent task IDs. A tasks can be a child of another task. |
| count_data_dependencies | [int64](#int64) |  | Count the data dependencies. A task have data dependencies. |
| count_expected_output_ids | [int64](#int64) |  | Count the expected output IDs. A task have expected output IDs. |
| count_retry_of_ids | [int64](#int64) |  | Count the retry of IDs. When a task fail, retry will use these set of IDs. |
| status | [armonik.api.grpc.v1.task_status.TaskStatus](#armonik-api-grpc-v1-task_status-TaskStatus) |  | The task status. |
| status_message | [string](#string) |  | The status message. |
| options | [armonik.api.grpc.v1.TaskOptions](#armonik-api-grpc-v1-TaskOptions) |  | The task options. |
| created_at | [google.protobuf.Timestamp](#google-protobuf-Timestamp) |  | The task creation date. |
| submitted_at | [google.protobuf.Timestamp](#google-protobuf-Timestamp) |  | The task submission date. |
| received_at | [google.protobuf.Timestamp](#google-protobuf-Timestamp) |  | When the task is received by the agent. |
| acquired_at | [google.protobuf.Timestamp](#google-protobuf-Timestamp) |  | When the task is acquired by the agent. |
| fetched_at | [google.protobuf.Timestamp](#google-protobuf-Timestamp) |  | Task data retrieval end date. |
| started_at | [google.protobuf.Timestamp](#google-protobuf-Timestamp) |  | The task start date. |
| processed_at | [google.protobuf.Timestamp](#google-protobuf-Timestamp) |  | The end of task processing date. |
| ended_at | [google.protobuf.Timestamp](#google-protobuf-Timestamp) |  | The task end date. Also used when task failed. |
| pod_ttl | [google.protobuf.Timestamp](#google-protobuf-Timestamp) |  | The pod TTL (Time To Live). |
| creation_to_end_duration | [google.protobuf.Duration](#google-protobuf-Duration) |  | The task duration. Between the creation date and the end date. |
| processing_to_end_duration | [google.protobuf.Duration](#google-protobuf-Duration) |  | The task calculated duration. Between the start date and the end date. |
| received_to_end_duration | [google.protobuf.Duration](#google-protobuf-Duration) |  | The task calculated duration. Between the received date and the end date. |
| payload_id | [string](#string) |  | The ID of the Result that is used as a payload for this task. |
| created_by | [string](#string) |  | The ID of the Task that as submitted this task empty if none. |
| error | [string](#string) |  | The error message. Only set if task have failed. |
| pod_hostname | [string](#string) |  | The hostname of the container running the task. |





 

 

 

 



<a id="tasks_fields-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## tasks_fields.proto



<a id="armonik-api-grpc-v1-tasks-TaskField"></a>

### TaskField



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task_summary_field | [TaskSummaryField](#armonik-api-grpc-v1-tasks-TaskSummaryField) |  | The task summary field. |
| task_option_field | [TaskOptionField](#armonik-api-grpc-v1-tasks-TaskOptionField) |  | The task option field. |
| task_option_generic_field | [TaskOptionGenericField](#armonik-api-grpc-v1-tasks-TaskOptionGenericField) |  | The task option generic field. |






<a id="armonik-api-grpc-v1-tasks-TaskOptionField"></a>

### TaskOptionField
This message is used to wrap the enum in order to facilitate the &#39;oneOf&#39; generation.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| field | [TaskOptionEnumField](#armonik-api-grpc-v1-tasks-TaskOptionEnumField) |  |  |






<a id="armonik-api-grpc-v1-tasks-TaskOptionGenericField"></a>

### TaskOptionGenericField
Represents a generic field in a task option.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| field | [string](#string) |  | The generic field. |






<a id="armonik-api-grpc-v1-tasks-TaskSummaryField"></a>

### TaskSummaryField
This message is used to wrap the enum in order to facilitate the &#39;oneOf&#39; generation.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| field | [TaskSummaryEnumField](#armonik-api-grpc-v1-tasks-TaskSummaryEnumField) |  |  |





 


<a id="armonik-api-grpc-v1-tasks-TaskOptionEnumField"></a>

### TaskOptionEnumField
Represents a field in a task option.

| Name | Number | Description |
| ---- | ------ | ----------- |
| TASK_OPTION_ENUM_FIELD_UNSPECIFIED | 0 |  |
| TASK_OPTION_ENUM_FIELD_MAX_DURATION | 1 |  |
| TASK_OPTION_ENUM_FIELD_MAX_RETRIES | 2 |  |
| TASK_OPTION_ENUM_FIELD_PRIORITY | 3 |  |
| TASK_OPTION_ENUM_FIELD_PARTITION_ID | 4 |  |
| TASK_OPTION_ENUM_FIELD_APPLICATION_NAME | 5 |  |
| TASK_OPTION_ENUM_FIELD_APPLICATION_VERSION | 6 |  |
| TASK_OPTION_ENUM_FIELD_APPLICATION_NAMESPACE | 7 |  |
| TASK_OPTION_ENUM_FIELD_APPLICATION_SERVICE | 8 |  |
| TASK_OPTION_ENUM_FIELD_ENGINE_TYPE | 9 |  |



<a id="armonik-api-grpc-v1-tasks-TaskSummaryEnumField"></a>

### TaskSummaryEnumField
Represents every available field in a task summary.

| Name | Number | Description |
| ---- | ------ | ----------- |
| TASK_SUMMARY_ENUM_FIELD_UNSPECIFIED | 0 | Unspecified |
| TASK_SUMMARY_ENUM_FIELD_TASK_ID | 16 | The task ID. |
| TASK_SUMMARY_ENUM_FIELD_SESSION_ID | 1 | The session ID. |
| TASK_SUMMARY_ENUM_FIELD_OWNER_POD_ID | 9 | The owner pod ID. |
| TASK_SUMMARY_ENUM_FIELD_INITIAL_TASK_ID | 10 | The initial task ID. Set when a task is submitted independently of retries. |
| TASK_SUMMARY_ENUM_FIELD_STATUS | 2 | The task status. |
| TASK_SUMMARY_ENUM_FIELD_CREATED_AT | 3 | The task creation date. |
| TASK_SUMMARY_ENUM_FIELD_SUBMITTED_AT | 11 | The task submission date. |
| TASK_SUMMARY_ENUM_FIELD_STARTED_AT | 4 | The task start date. |
| TASK_SUMMARY_ENUM_FIELD_ENDED_AT | 5 | The task end date. |
| TASK_SUMMARY_ENUM_FIELD_CREATION_TO_END_DURATION | 6 | The task duration. Between the creation date and the end date. |
| TASK_SUMMARY_ENUM_FIELD_PROCESSING_TO_END_DURATION | 7 | The task calculated duration. Between the start date and the end date. |
| TASK_SUMMARY_ENUM_FIELD_RECEIVED_TO_END_DURATION | 18 | The task calculated duration. Between the received date and the end date. |
| TASK_SUMMARY_ENUM_FIELD_POD_TTL | 12 | The pod TTL (Time To Live). |
| TASK_SUMMARY_ENUM_FIELD_POD_HOSTNAME | 13 | The hostname of the container running the task. |
| TASK_SUMMARY_ENUM_FIELD_RECEIVED_AT | 14 | When the task is received by the agent. |
| TASK_SUMMARY_ENUM_FIELD_ACQUIRED_AT | 15 | When the task is acquired by the agent. |
| TASK_SUMMARY_ENUM_FIELD_PROCESSED_AT | 17 | When the task is processed by the agent. |
| TASK_SUMMARY_ENUM_FIELD_ERROR | 8 | The error message. Only set if task have failed. |
| TASK_SUMMARY_ENUM_FIELD_FETCHED_AT | 19 | When task data are fetched by the agent. |
| TASK_SUMMARY_ENUM_FIELD_PAYLOAD_ID | 20 | The ID of the Result that is used as a payload for this task. |
| TASK_SUMMARY_ENUM_FIELD_CREATED_BY | 21 | The ID of the Result that is used as a payload for this task. |


 

 

 



<a id="tasks_filters-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## tasks_filters.proto



<a id="armonik-api-grpc-v1-tasks-FilterField"></a>

### FilterField



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| field | [TaskField](#armonik-api-grpc-v1-tasks-TaskField) |  |  |
| filter_string | [armonik.api.grpc.v1.FilterString](#armonik-api-grpc-v1-FilterString) |  |  |
| filter_number | [armonik.api.grpc.v1.FilterNumber](#armonik-api-grpc-v1-FilterNumber) |  |  |
| filter_boolean | [armonik.api.grpc.v1.FilterBoolean](#armonik-api-grpc-v1-FilterBoolean) |  |  |
| filter_status | [FilterStatus](#armonik-api-grpc-v1-tasks-FilterStatus) |  |  |
| filter_date | [armonik.api.grpc.v1.FilterDate](#armonik-api-grpc-v1-FilterDate) |  |  |
| filter_array | [armonik.api.grpc.v1.FilterArray](#armonik-api-grpc-v1-FilterArray) |  |  |
| filter_duration | [armonik.api.grpc.v1.FilterDuration](#armonik-api-grpc-v1-FilterDuration) |  |  |






<a id="armonik-api-grpc-v1-tasks-FilterStatus"></a>

### FilterStatus



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| value | [armonik.api.grpc.v1.task_status.TaskStatus](#armonik-api-grpc-v1-task_status-TaskStatus) |  |  |
| operator | [armonik.api.grpc.v1.FilterStatusOperator](#armonik-api-grpc-v1-FilterStatusOperator) |  |  |






<a id="armonik-api-grpc-v1-tasks-Filters"></a>

### Filters



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| or | [FiltersAnd](#armonik-api-grpc-v1-tasks-FiltersAnd) | repeated |  |






<a id="armonik-api-grpc-v1-tasks-FiltersAnd"></a>

### FiltersAnd



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| and | [FilterField](#armonik-api-grpc-v1-tasks-FilterField) | repeated |  |





 

 

 

 



<a id="tasks_service-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## tasks_service.proto
Tasks related methods within a service.

 

 

 


<a id="armonik-api-grpc-v1-tasks-Tasks"></a>

### Tasks
Service for handling tasks.

| Method Name | Request Type | Response Type | Description |
| ----------- | ------------ | ------------- | ------------|
| ListTasks | [ListTasksRequest](#armonik-api-grpc-v1-tasks-ListTasksRequest) | [ListTasksResponse](#armonik-api-grpc-v1-tasks-ListTasksResponse) | Get a tasks list using pagination, filters and sorting. |
| ListTasksDetailed | [ListTasksRequest](#armonik-api-grpc-v1-tasks-ListTasksRequest) | [ListTasksDetailedResponse](#armonik-api-grpc-v1-tasks-ListTasksDetailedResponse) | Get a tasks list using pagination, filters and sorting with complete metada. |
| GetTask | [GetTaskRequest](#armonik-api-grpc-v1-tasks-GetTaskRequest) | [GetTaskResponse](#armonik-api-grpc-v1-tasks-GetTaskResponse) | Get a task by its id. |
| CancelTasks | [CancelTasksRequest](#armonik-api-grpc-v1-tasks-CancelTasksRequest) | [CancelTasksResponse](#armonik-api-grpc-v1-tasks-CancelTasksResponse) | Cancel tasks using ids. |
| GetResultIds | [GetResultIdsRequest](#armonik-api-grpc-v1-tasks-GetResultIdsRequest) | [GetResultIdsResponse](#armonik-api-grpc-v1-tasks-GetResultIdsResponse) | Get ids of the result that tasks should produce. |
| CountTasksByStatus | [CountTasksByStatusRequest](#armonik-api-grpc-v1-tasks-CountTasksByStatusRequest) | [CountTasksByStatusResponse](#armonik-api-grpc-v1-tasks-CountTasksByStatusResponse) | Get count from tasks status. |
| SubmitTasks | [SubmitTasksRequest](#armonik-api-grpc-v1-tasks-SubmitTasksRequest) | [SubmitTasksResponse](#armonik-api-grpc-v1-tasks-SubmitTasksResponse) | Create tasks metadata and submit task for processing. |

 



<a id="task_status-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## task_status.proto
Status of a task.

 


<a id="armonik-api-grpc-v1-task_status-TaskStatus"></a>

### TaskStatus
Task status.

| Name | Number | Description |
| ---- | ------ | ----------- |
| TASK_STATUS_UNSPECIFIED | 0 | Task is in an unknown state. |
| TASK_STATUS_CREATING | 1 | Task is being created in database. |
| TASK_STATUS_SUBMITTED | 2 | Task is submitted to the queue. |
| TASK_STATUS_DISPATCHED | 3 | Task is dispatched to a worker. |
| TASK_STATUS_COMPLETED | 4 | Task is completed. |
| TASK_STATUS_ERROR | 5 | Task is in error state. |
| TASK_STATUS_TIMEOUT | 6 | Task is in timeout state. |
| TASK_STATUS_CANCELLING | 7 | Task is being cancelled. |
| TASK_STATUS_CANCELLED | 8 | Task is cancelled. |
| TASK_STATUS_PROCESSING | 9 | Task is being processed. |
| TASK_STATUS_PROCESSED | 10 | Task is processed. |
| TASK_STATUS_RETRIED | 11 | Task is retried. |
| TASK_STATUS_PENDING | 12 | Task is waiting for its dependencies before becoming executable. |
| TASK_STATUS_PAUSED | 13 | Task is paused and will not be executed until session is resumed. |


 

 

 



<a id="versions_common-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## versions_common.proto
Message describing versions.


<a id="armonik-api-grpc-v1-versions-ListVersionsRequest"></a>

### ListVersionsRequest
Request to list versions.






<a id="armonik-api-grpc-v1-versions-ListVersionsResponse"></a>

### ListVersionsResponse
Response to list versions.


| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| core | [string](#string) |  |  |
| api | [string](#string) |  | We can add more versions here. |





 

 

 

 



<a id="versions_service-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## versions_service.proto
Versions related methods within a service.
This service will be used to get the version of infrastructure components from outside the cluster.

 

 

 


<a id="armonik-api-grpc-v1-versions-Versions"></a>

### Versions
Service for handling versions.

| Method Name | Request Type | Response Type | Description |
| ----------- | ------------ | ------------- | ------------|
| ListVersions | [ListVersionsRequest](#armonik-api-grpc-v1-versions-ListVersionsRequest) | [ListVersionsResponse](#armonik-api-grpc-v1-versions-ListVersionsResponse) | Get all versions. |

 



<a id="worker_common-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## worker_common.proto



<a id="armonik-api-grpc-v1-worker-HealthCheckReply"></a>

### HealthCheckReply



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| status | [HealthCheckReply.ServingStatus](#armonik-api-grpc-v1-worker-HealthCheckReply-ServingStatus) |  |  |






<a id="armonik-api-grpc-v1-worker-ProcessReply"></a>

### ProcessReply



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| output | [armonik.api.grpc.v1.Output](#armonik-api-grpc-v1-Output) |  |  |






<a id="armonik-api-grpc-v1-worker-ProcessRequest"></a>

### ProcessRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| communication_token | [string](#string) |  |  |
| session_id | [string](#string) |  |  |
| task_id | [string](#string) |  |  |
| task_options | [armonik.api.grpc.v1.TaskOptions](#armonik-api-grpc-v1-TaskOptions) |  |  |
| expected_output_keys | [string](#string) | repeated |  |
| payload_id | [string](#string) |  |  |
| data_dependencies | [string](#string) | repeated |  |
| data_folder | [string](#string) |  |  |
| configuration | [armonik.api.grpc.v1.Configuration](#armonik-api-grpc-v1-Configuration) |  |  |





 


<a id="armonik-api-grpc-v1-worker-HealthCheckReply-ServingStatus"></a>

### HealthCheckReply.ServingStatus


| Name | Number | Description |
| ---- | ------ | ----------- |
| UNKNOWN | 0 |  |
| SERVING | 1 |  |
| NOT_SERVING | 2 |  |


 

 

 



<a id="worker_service-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## worker_service.proto


 

 

 


<a id="armonik-api-grpc-v1-worker-Worker"></a>

### Worker


| Method Name | Request Type | Response Type | Description |
| ----------- | ------------ | ------------- | ------------|
| Process | [ProcessRequest](#armonik-api-grpc-v1-worker-ProcessRequest) | [ProcessReply](#armonik-api-grpc-v1-worker-ProcessReply) |  |
| HealthCheck | [.armonik.api.grpc.v1.Empty](#armonik-api-grpc-v1-Empty) | [HealthCheckReply](#armonik-api-grpc-v1-worker-HealthCheckReply) |  |

 



## Scalar Value Types

| .proto Type | Notes | C++ | Java | Python | Go | C# | PHP | Ruby |
| ----------- | ----- | --- | ---- | ------ | -- | -- | --- | ---- |
| <a id="double" /> double |  | double | double | float | float64 | double | float | Float |
| <a id="float" /> float |  | float | float | float | float32 | float | float | Float |
| <a id="int32" /> int32 | Uses variable-length encoding. Inefficient for encoding negative numbers – if your field is likely to have negative values, use sint32 instead. | int32 | int | int | int32 | int | integer | Bignum or Fixnum (as required) |
| <a id="int64" /> int64 | Uses variable-length encoding. Inefficient for encoding negative numbers – if your field is likely to have negative values, use sint64 instead. | int64 | long | int/long | int64 | long | integer/string | Bignum |
| <a id="uint32" /> uint32 | Uses variable-length encoding. | uint32 | int | int/long | uint32 | uint | integer | Bignum or Fixnum (as required) |
| <a id="uint64" /> uint64 | Uses variable-length encoding. | uint64 | long | int/long | uint64 | ulong | integer/string | Bignum or Fixnum (as required) |
| <a id="sint32" /> sint32 | Uses variable-length encoding. Signed int value. These more efficiently encode negative numbers than regular int32s. | int32 | int | int | int32 | int | integer | Bignum or Fixnum (as required) |
| <a id="sint64" /> sint64 | Uses variable-length encoding. Signed int value. These more efficiently encode negative numbers than regular int64s. | int64 | long | int/long | int64 | long | integer/string | Bignum |
| <a id="fixed32" /> fixed32 | Always four bytes. More efficient than uint32 if values are often greater than 2^28. | uint32 | int | int | uint32 | uint | integer | Bignum or Fixnum (as required) |
| <a id="fixed64" /> fixed64 | Always eight bytes. More efficient than uint64 if values are often greater than 2^56. | uint64 | long | int/long | uint64 | ulong | integer/string | Bignum |
| <a id="sfixed32" /> sfixed32 | Always four bytes. | int32 | int | int | int32 | int | integer | Bignum or Fixnum (as required) |
| <a id="sfixed64" /> sfixed64 | Always eight bytes. | int64 | long | int/long | int64 | long | integer/string | Bignum |
| <a id="bool" /> bool |  | bool | boolean | boolean | bool | bool | boolean | TrueClass/FalseClass |
| <a id="string" /> string | A string must always contain UTF-8 encoded or 7-bit ASCII text. | string | String | str/unicode | string | string | string | String (UTF-8) |
| <a id="bytes" /> bytes | May contain any arbitrary sequence of bytes. | string | ByteString | str | []byte | ByteString | string | String (ASCII-8BIT) |

