# Protocol Documentation
<a name="top"></a>

## Table of Contents

- [objects.proto](#objects-proto)
    - [Configuration](#ArmoniK-api-grpc-v1-Configuration)
    - [Count](#ArmoniK-api-grpc-v1-Count)
    - [CreateTaskReply](#ArmoniK-api-grpc-v1-CreateTaskReply)
    - [CreateTaskReply.TaskIds](#ArmoniK-api-grpc-v1-CreateTaskReply-TaskIds)
    - [DataChunk](#ArmoniK-api-grpc-v1-DataChunk)
    - [Empty](#ArmoniK-api-grpc-v1-Empty)
    - [Error](#ArmoniK-api-grpc-v1-Error)
    - [InitKeyedDataStream](#ArmoniK-api-grpc-v1-InitKeyedDataStream)
    - [InitTaskRequest](#ArmoniK-api-grpc-v1-InitTaskRequest)
    - [Output](#ArmoniK-api-grpc-v1-Output)
    - [Output.Error](#ArmoniK-api-grpc-v1-Output-Error)
    - [ResultRequest](#ArmoniK-api-grpc-v1-ResultRequest)
    - [StatusCount](#ArmoniK-api-grpc-v1-StatusCount)
    - [TaskError](#ArmoniK-api-grpc-v1-TaskError)
    - [TaskId](#ArmoniK-api-grpc-v1-TaskId)
    - [TaskIdList](#ArmoniK-api-grpc-v1-TaskIdList)
    - [TaskIdWithStatus](#ArmoniK-api-grpc-v1-TaskIdWithStatus)
    - [TaskList](#ArmoniK-api-grpc-v1-TaskList)
    - [TaskOptions](#ArmoniK-api-grpc-v1-TaskOptions)
    - [TaskOptions.OptionsEntry](#ArmoniK-api-grpc-v1-TaskOptions-OptionsEntry)
    - [TaskRequest](#ArmoniK-api-grpc-v1-TaskRequest)
    - [TaskRequestHeader](#ArmoniK-api-grpc-v1-TaskRequestHeader)
  
- [result_status.proto](#result_status-proto)
    - [ResultStatus](#ArmoniK-api-grpc-v1-ResultStatus-ResultStatus)
  
- [session_status.proto](#session_status-proto)
    - [SessionStatus](#ArmoniK-api-grpc-v1-SessionStatus-SessionStatus)
  
- [submitter_service.proto](#submitter_service-proto)
    - [AvailabilityReply](#ArmoniK-api-grpc-v1-AvailabilityReply)
    - [CreateLargeTaskRequest](#ArmoniK-api-grpc-v1-CreateLargeTaskRequest)
    - [CreateLargeTaskRequest.InitRequest](#ArmoniK-api-grpc-v1-CreateLargeTaskRequest-InitRequest)
    - [CreateSessionReply](#ArmoniK-api-grpc-v1-CreateSessionReply)
    - [CreateSessionRequest](#ArmoniK-api-grpc-v1-CreateSessionRequest)
    - [CreateSmallTaskRequest](#ArmoniK-api-grpc-v1-CreateSmallTaskRequest)
    - [GetResultStatusReply](#ArmoniK-api-grpc-v1-GetResultStatusReply)
    - [GetResultStatusReply.IdStatus](#ArmoniK-api-grpc-v1-GetResultStatusReply-IdStatus)
    - [GetResultStatusRequest](#ArmoniK-api-grpc-v1-GetResultStatusRequest)
    - [GetTaskStatusReply](#ArmoniK-api-grpc-v1-GetTaskStatusReply)
    - [GetTaskStatusReply.IdStatus](#ArmoniK-api-grpc-v1-GetTaskStatusReply-IdStatus)
    - [GetTaskStatusRequest](#ArmoniK-api-grpc-v1-GetTaskStatusRequest)
    - [ResultReply](#ArmoniK-api-grpc-v1-ResultReply)
    - [Session](#ArmoniK-api-grpc-v1-Session)
    - [SessionFilter](#ArmoniK-api-grpc-v1-SessionFilter)
    - [SessionFilter.StatusesRequest](#ArmoniK-api-grpc-v1-SessionFilter-StatusesRequest)
    - [SessionIdList](#ArmoniK-api-grpc-v1-SessionIdList)
    - [SessionList](#ArmoniK-api-grpc-v1-SessionList)
    - [TaskFilter](#ArmoniK-api-grpc-v1-TaskFilter)
    - [TaskFilter.IdsRequest](#ArmoniK-api-grpc-v1-TaskFilter-IdsRequest)
    - [TaskFilter.StatusesRequest](#ArmoniK-api-grpc-v1-TaskFilter-StatusesRequest)
    - [WaitRequest](#ArmoniK-api-grpc-v1-WaitRequest)
  
    - [Submitter](#ArmoniK-api-grpc-v1-Submitter)
  
- [task_status.proto](#task_status-proto)
    - [TaskStatus](#ArmoniK-api-grpc-v1-TaskStatus-TaskStatus)
  
- [worker_service.proto](#worker_service-proto)
    - [HealthCheckReply](#ArmoniK-api-grpc-v1-HealthCheckReply)
    - [ProcessReply](#ArmoniK-api-grpc-v1-ProcessReply)
    - [ProcessReply.CreateLargeTaskRequest](#ArmoniK-api-grpc-v1-ProcessReply-CreateLargeTaskRequest)
    - [ProcessReply.CreateLargeTaskRequest.InitRequest](#ArmoniK-api-grpc-v1-ProcessReply-CreateLargeTaskRequest-InitRequest)
    - [ProcessReply.CreateSmallTaskRequest](#ArmoniK-api-grpc-v1-ProcessReply-CreateSmallTaskRequest)
    - [ProcessReply.DataRequest](#ArmoniK-api-grpc-v1-ProcessReply-DataRequest)
    - [ProcessReply.Result](#ArmoniK-api-grpc-v1-ProcessReply-Result)
    - [ProcessRequest](#ArmoniK-api-grpc-v1-ProcessRequest)
    - [ProcessRequest.ComputeRequest](#ArmoniK-api-grpc-v1-ProcessRequest-ComputeRequest)
    - [ProcessRequest.ComputeRequest.InitData](#ArmoniK-api-grpc-v1-ProcessRequest-ComputeRequest-InitData)
    - [ProcessRequest.ComputeRequest.InitRequest](#ArmoniK-api-grpc-v1-ProcessRequest-ComputeRequest-InitRequest)
    - [ProcessRequest.ComputeRequest.InitRequest.TaskOptionsEntry](#ArmoniK-api-grpc-v1-ProcessRequest-ComputeRequest-InitRequest-TaskOptionsEntry)
    - [ProcessRequest.CreateTask](#ArmoniK-api-grpc-v1-ProcessRequest-CreateTask)
    - [ProcessRequest.DataReply](#ArmoniK-api-grpc-v1-ProcessRequest-DataReply)
    - [ProcessRequest.DataReply.Init](#ArmoniK-api-grpc-v1-ProcessRequest-DataReply-Init)
  
    - [HealthCheckReply.ServingStatus](#ArmoniK-api-grpc-v1-HealthCheckReply-ServingStatus)
  
    - [Worker](#ArmoniK-api-grpc-v1-Worker)
  
- [Scalar Value Types](#scalar-value-types)



<a name="objects-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## objects.proto



<a name="ArmoniK-api-grpc-v1-Configuration"></a>

### Configuration



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| data_chunk_max_size | [int32](#int32) |  |  |






<a name="ArmoniK-api-grpc-v1-Count"></a>

### Count



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| values | [StatusCount](#ArmoniK-api-grpc-v1-StatusCount) | repeated |  |






<a name="ArmoniK-api-grpc-v1-CreateTaskReply"></a>

### CreateTaskReply



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| successfull | [Empty](#ArmoniK-api-grpc-v1-Empty) |  |  |
| non_successfull_ids | [CreateTaskReply.TaskIds](#ArmoniK-api-grpc-v1-CreateTaskReply-TaskIds) |  |  |






<a name="ArmoniK-api-grpc-v1-CreateTaskReply-TaskIds"></a>

### CreateTaskReply.TaskIds



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| ids | [string](#string) | repeated |  |






<a name="ArmoniK-api-grpc-v1-DataChunk"></a>

### DataChunk



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| data | [bytes](#bytes) |  |  |
| data_complete | [bool](#bool) |  |  |






<a name="ArmoniK-api-grpc-v1-Empty"></a>

### Empty







<a name="ArmoniK-api-grpc-v1-Error"></a>

### Error



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task_status | [TaskStatus.TaskStatus](#ArmoniK-api-grpc-v1-TaskStatus-TaskStatus) |  |  |
| detail | [string](#string) |  |  |






<a name="ArmoniK-api-grpc-v1-InitKeyedDataStream"></a>

### InitKeyedDataStream



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| key | [string](#string) |  |  |
| last_result | [bool](#bool) |  |  |






<a name="ArmoniK-api-grpc-v1-InitTaskRequest"></a>

### InitTaskRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| header | [TaskRequestHeader](#ArmoniK-api-grpc-v1-TaskRequestHeader) |  |  |
| last_task | [bool](#bool) |  |  |






<a name="ArmoniK-api-grpc-v1-Output"></a>

### Output



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| status | [TaskStatus.TaskStatus](#ArmoniK-api-grpc-v1-TaskStatus-TaskStatus) |  | **Deprecated.**  |
| ok | [Empty](#ArmoniK-api-grpc-v1-Empty) |  |  |
| error | [Output.Error](#ArmoniK-api-grpc-v1-Output-Error) |  |  |






<a name="ArmoniK-api-grpc-v1-Output-Error"></a>

### Output.Error



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| details | [string](#string) |  |  |
| kill_sub_tasks | [bool](#bool) |  | **Deprecated.**  |






<a name="ArmoniK-api-grpc-v1-ResultRequest"></a>

### ResultRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session | [string](#string) |  |  |
| key | [string](#string) |  |  |






<a name="ArmoniK-api-grpc-v1-StatusCount"></a>

### StatusCount



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| status | [TaskStatus.TaskStatus](#ArmoniK-api-grpc-v1-TaskStatus-TaskStatus) |  |  |
| count | [int32](#int32) |  |  |






<a name="ArmoniK-api-grpc-v1-TaskError"></a>

### TaskError



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task_id | [string](#string) |  |  |
| errors | [Error](#ArmoniK-api-grpc-v1-Error) | repeated |  |






<a name="ArmoniK-api-grpc-v1-TaskId"></a>

### TaskId



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session | [string](#string) |  |  |
| task | [string](#string) |  |  |






<a name="ArmoniK-api-grpc-v1-TaskIdList"></a>

### TaskIdList



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task_ids | [string](#string) | repeated |  |






<a name="ArmoniK-api-grpc-v1-TaskIdWithStatus"></a>

### TaskIdWithStatus



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task_id | [TaskId](#ArmoniK-api-grpc-v1-TaskId) |  |  |
| status | [TaskStatus.TaskStatus](#ArmoniK-api-grpc-v1-TaskStatus-TaskStatus) |  |  |






<a name="ArmoniK-api-grpc-v1-TaskList"></a>

### TaskList



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task_ids | [TaskId](#ArmoniK-api-grpc-v1-TaskId) | repeated |  |






<a name="ArmoniK-api-grpc-v1-TaskOptions"></a>

### TaskOptions



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| options | [TaskOptions.OptionsEntry](#ArmoniK-api-grpc-v1-TaskOptions-OptionsEntry) | repeated |  |
| max_duration | [google.protobuf.Duration](#google-protobuf-Duration) |  |  |
| max_retries | [int32](#int32) |  |  |
| priority | [int32](#int32) |  |  |






<a name="ArmoniK-api-grpc-v1-TaskOptions-OptionsEntry"></a>

### TaskOptions.OptionsEntry



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| key | [string](#string) |  |  |
| value | [string](#string) |  |  |






<a name="ArmoniK-api-grpc-v1-TaskRequest"></a>

### TaskRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| id | [string](#string) |  |  |
| expected_output_keys | [string](#string) | repeated |  |
| data_dependencies | [string](#string) | repeated |  |
| payload | [bytes](#bytes) |  |  |






<a name="ArmoniK-api-grpc-v1-TaskRequestHeader"></a>

### TaskRequestHeader



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| id | [string](#string) |  |  |
| expected_output_keys | [string](#string) | repeated |  |
| data_dependencies | [string](#string) | repeated |  |





 

 

 

 



<a name="result_status-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## result_status.proto


 


<a name="ArmoniK-api-grpc-v1-ResultStatus-ResultStatus"></a>

### ResultStatus


| Name | Number | Description |
| ---- | ------ | ----------- |
| RESULT_STATUS_UNSPECIFIED | 0 |  |
| RESULT_STATUS_CREATED | 1 |  |
| RESULT_STATUS_COMPLETED | 2 |  |
| RESULT_STATUS_ABORTED | 3 |  |


 

 

 



<a name="session_status-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## session_status.proto


 


<a name="ArmoniK-api-grpc-v1-SessionStatus-SessionStatus"></a>

### SessionStatus


| Name | Number | Description |
| ---- | ------ | ----------- |
| SESSION_STATUS_UNSPECIFIED | 0 |  |
| SESSION_STATUS_RUNNING | 1 |  |
| SESSION_STATUS_CANCELED | 2 |  |


 

 

 



<a name="submitter_service-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## submitter_service.proto



<a name="ArmoniK-api-grpc-v1-AvailabilityReply"></a>

### AvailabilityReply



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| ok | [Empty](#ArmoniK-api-grpc-v1-Empty) |  |  |
| error | [TaskError](#ArmoniK-api-grpc-v1-TaskError) |  |  |
| not_completed_task | [string](#string) |  |  |






<a name="ArmoniK-api-grpc-v1-CreateLargeTaskRequest"></a>

### CreateLargeTaskRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| init_request | [CreateLargeTaskRequest.InitRequest](#ArmoniK-api-grpc-v1-CreateLargeTaskRequest-InitRequest) |  |  |
| init_task | [InitTaskRequest](#ArmoniK-api-grpc-v1-InitTaskRequest) |  |  |
| task_payload | [DataChunk](#ArmoniK-api-grpc-v1-DataChunk) |  |  |






<a name="ArmoniK-api-grpc-v1-CreateLargeTaskRequest-InitRequest"></a>

### CreateLargeTaskRequest.InitRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session_id | [string](#string) |  |  |
| task_options | [TaskOptions](#ArmoniK-api-grpc-v1-TaskOptions) |  |  |






<a name="ArmoniK-api-grpc-v1-CreateSessionReply"></a>

### CreateSessionReply



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| ok | [Empty](#ArmoniK-api-grpc-v1-Empty) |  |  |
| error | [string](#string) |  |  |






<a name="ArmoniK-api-grpc-v1-CreateSessionRequest"></a>

### CreateSessionRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| id | [string](#string) |  |  |
| default_task_option | [TaskOptions](#ArmoniK-api-grpc-v1-TaskOptions) |  |  |






<a name="ArmoniK-api-grpc-v1-CreateSmallTaskRequest"></a>

### CreateSmallTaskRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session_id | [string](#string) |  |  |
| task_options | [TaskOptions](#ArmoniK-api-grpc-v1-TaskOptions) |  |  |
| task_requests | [TaskRequest](#ArmoniK-api-grpc-v1-TaskRequest) | repeated |  |






<a name="ArmoniK-api-grpc-v1-GetResultStatusReply"></a>

### GetResultStatusReply



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| id_statuses | [GetResultStatusReply.IdStatus](#ArmoniK-api-grpc-v1-GetResultStatusReply-IdStatus) | repeated |  |






<a name="ArmoniK-api-grpc-v1-GetResultStatusReply-IdStatus"></a>

### GetResultStatusReply.IdStatus



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| result_id | [string](#string) |  |  |
| status | [ResultStatus.ResultStatus](#ArmoniK-api-grpc-v1-ResultStatus-ResultStatus) |  |  |






<a name="ArmoniK-api-grpc-v1-GetResultStatusRequest"></a>

### GetResultStatusRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| result_ids | [string](#string) | repeated |  |
| session_id | [string](#string) |  |  |






<a name="ArmoniK-api-grpc-v1-GetTaskStatusReply"></a>

### GetTaskStatusReply



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| id_statuses | [GetTaskStatusReply.IdStatus](#ArmoniK-api-grpc-v1-GetTaskStatusReply-IdStatus) | repeated |  |






<a name="ArmoniK-api-grpc-v1-GetTaskStatusReply-IdStatus"></a>

### GetTaskStatusReply.IdStatus



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task_id | [string](#string) |  |  |
| status | [TaskStatus.TaskStatus](#ArmoniK-api-grpc-v1-TaskStatus-TaskStatus) |  |  |






<a name="ArmoniK-api-grpc-v1-GetTaskStatusRequest"></a>

### GetTaskStatusRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task_ids | [string](#string) | repeated |  |






<a name="ArmoniK-api-grpc-v1-ResultReply"></a>

### ResultReply



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| result | [DataChunk](#ArmoniK-api-grpc-v1-DataChunk) |  |  |
| error | [TaskError](#ArmoniK-api-grpc-v1-TaskError) |  |  |
| not_completed_task | [string](#string) |  |  |






<a name="ArmoniK-api-grpc-v1-Session"></a>

### Session



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| id | [string](#string) |  |  |






<a name="ArmoniK-api-grpc-v1-SessionFilter"></a>

### SessionFilter



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| sessions | [string](#string) | repeated |  |
| included | [SessionFilter.StatusesRequest](#ArmoniK-api-grpc-v1-SessionFilter-StatusesRequest) |  |  |
| excluded | [SessionFilter.StatusesRequest](#ArmoniK-api-grpc-v1-SessionFilter-StatusesRequest) |  |  |






<a name="ArmoniK-api-grpc-v1-SessionFilter-StatusesRequest"></a>

### SessionFilter.StatusesRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| statuses | [SessionStatus.SessionStatus](#ArmoniK-api-grpc-v1-SessionStatus-SessionStatus) | repeated |  |






<a name="ArmoniK-api-grpc-v1-SessionIdList"></a>

### SessionIdList



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session_ids | [string](#string) | repeated |  |






<a name="ArmoniK-api-grpc-v1-SessionList"></a>

### SessionList



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| sessions | [Session](#ArmoniK-api-grpc-v1-Session) | repeated |  |






<a name="ArmoniK-api-grpc-v1-TaskFilter"></a>

### TaskFilter



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| session | [TaskFilter.IdsRequest](#ArmoniK-api-grpc-v1-TaskFilter-IdsRequest) |  |  |
| task | [TaskFilter.IdsRequest](#ArmoniK-api-grpc-v1-TaskFilter-IdsRequest) |  |  |
| included | [TaskFilter.StatusesRequest](#ArmoniK-api-grpc-v1-TaskFilter-StatusesRequest) |  |  |
| excluded | [TaskFilter.StatusesRequest](#ArmoniK-api-grpc-v1-TaskFilter-StatusesRequest) |  |  |






<a name="ArmoniK-api-grpc-v1-TaskFilter-IdsRequest"></a>

### TaskFilter.IdsRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| ids | [string](#string) | repeated |  |






<a name="ArmoniK-api-grpc-v1-TaskFilter-StatusesRequest"></a>

### TaskFilter.StatusesRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| statuses | [TaskStatus.TaskStatus](#ArmoniK-api-grpc-v1-TaskStatus-TaskStatus) | repeated |  |






<a name="ArmoniK-api-grpc-v1-WaitRequest"></a>

### WaitRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| filter | [TaskFilter](#ArmoniK-api-grpc-v1-TaskFilter) |  |  |
| stop_on_first_task_error | [bool](#bool) |  |  |
| stop_on_first_task_cancellation | [bool](#bool) |  |  |





 

 

 


<a name="ArmoniK-api-grpc-v1-Submitter"></a>

### Submitter


| Method Name | Request Type | Response Type | Description |
| ----------- | ------------ | ------------- | ------------|
| GetServiceConfiguration | [Empty](#ArmoniK-api-grpc-v1-Empty) | [Configuration](#ArmoniK-api-grpc-v1-Configuration) |  |
| CreateSession | [CreateSessionRequest](#ArmoniK-api-grpc-v1-CreateSessionRequest) | [CreateSessionReply](#ArmoniK-api-grpc-v1-CreateSessionReply) |  |
| CancelSession | [Session](#ArmoniK-api-grpc-v1-Session) | [Empty](#ArmoniK-api-grpc-v1-Empty) |  |
| CreateSmallTasks | [CreateSmallTaskRequest](#ArmoniK-api-grpc-v1-CreateSmallTaskRequest) | [CreateTaskReply](#ArmoniK-api-grpc-v1-CreateTaskReply) |  |
| CreateLargeTasks | [CreateLargeTaskRequest](#ArmoniK-api-grpc-v1-CreateLargeTaskRequest) stream | [CreateTaskReply](#ArmoniK-api-grpc-v1-CreateTaskReply) |  |
| ListTasks | [TaskFilter](#ArmoniK-api-grpc-v1-TaskFilter) | [TaskIdList](#ArmoniK-api-grpc-v1-TaskIdList) |  |
| ListSessions | [SessionFilter](#ArmoniK-api-grpc-v1-SessionFilter) | [SessionIdList](#ArmoniK-api-grpc-v1-SessionIdList) |  |
| CountTasks | [TaskFilter](#ArmoniK-api-grpc-v1-TaskFilter) | [Count](#ArmoniK-api-grpc-v1-Count) |  |
| TryGetResultStream | [ResultRequest](#ArmoniK-api-grpc-v1-ResultRequest) | [ResultReply](#ArmoniK-api-grpc-v1-ResultReply) stream |  |
| TryGetTaskOutput | [ResultRequest](#ArmoniK-api-grpc-v1-ResultRequest) | [Output](#ArmoniK-api-grpc-v1-Output) |  |
| WaitForAvailability | [ResultRequest](#ArmoniK-api-grpc-v1-ResultRequest) | [AvailabilityReply](#ArmoniK-api-grpc-v1-AvailabilityReply) |  |
| WaitForCompletion | [WaitRequest](#ArmoniK-api-grpc-v1-WaitRequest) | [Count](#ArmoniK-api-grpc-v1-Count) |  |
| CancelTasks | [TaskFilter](#ArmoniK-api-grpc-v1-TaskFilter) | [Empty](#ArmoniK-api-grpc-v1-Empty) |  |
| GetTaskStatus | [GetTaskStatusRequest](#ArmoniK-api-grpc-v1-GetTaskStatusRequest) | [GetTaskStatusReply](#ArmoniK-api-grpc-v1-GetTaskStatusReply) |  |
| GetResultStatus | [GetResultStatusRequest](#ArmoniK-api-grpc-v1-GetResultStatusRequest) | [GetResultStatusReply](#ArmoniK-api-grpc-v1-GetResultStatusReply) |  |

 



<a name="task_status-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## task_status.proto


 


<a name="ArmoniK-api-grpc-v1-TaskStatus-TaskStatus"></a>

### TaskStatus


| Name | Number | Description |
| ---- | ------ | ----------- |
| TASK_STATUS_UNSPECIFIED | 0 |  |
| TASK_STATUS_CREATING | 1 |  |
| TASK_STATUS_SUBMITTED | 2 |  |
| TASK_STATUS_DISPATCHED | 3 |  |
| TASK_STATUS_PROCESSING | 10 |  |
| TASK_STATUS_PROCESSED | 11 |  |
| TASK_STATUS_COMPLETED | 4 |  |
| TASK_STATUS_ERROR | 5 |  |
| TASK_STATUS_FAILED | 6 |  |
| TASK_STATUS_TIMEOUT | 7 |  |
| TASK_STATUS_CANCELING | 8 |  |
| TASK_STATUS_CANCELED | 9 |  |


 

 

 



<a name="worker_service-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## worker_service.proto



<a name="ArmoniK-api-grpc-v1-HealthCheckReply"></a>

### HealthCheckReply



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| status | [HealthCheckReply.ServingStatus](#ArmoniK-api-grpc-v1-HealthCheckReply-ServingStatus) |  |  |






<a name="ArmoniK-api-grpc-v1-ProcessReply"></a>

### ProcessReply



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| request_id | [string](#string) |  |  |
| output | [Output](#ArmoniK-api-grpc-v1-Output) |  |  |
| result | [ProcessReply.Result](#ArmoniK-api-grpc-v1-ProcessReply-Result) |  |  |
| create_small_task | [ProcessReply.CreateSmallTaskRequest](#ArmoniK-api-grpc-v1-ProcessReply-CreateSmallTaskRequest) |  |  |
| create_large_task | [ProcessReply.CreateLargeTaskRequest](#ArmoniK-api-grpc-v1-ProcessReply-CreateLargeTaskRequest) |  |  |
| resource | [ProcessReply.DataRequest](#ArmoniK-api-grpc-v1-ProcessReply-DataRequest) |  |  |
| common_data | [ProcessReply.DataRequest](#ArmoniK-api-grpc-v1-ProcessReply-DataRequest) |  |  |
| direct_data | [ProcessReply.DataRequest](#ArmoniK-api-grpc-v1-ProcessReply-DataRequest) |  |  |






<a name="ArmoniK-api-grpc-v1-ProcessReply-CreateLargeTaskRequest"></a>

### ProcessReply.CreateLargeTaskRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| init_request | [ProcessReply.CreateLargeTaskRequest.InitRequest](#ArmoniK-api-grpc-v1-ProcessReply-CreateLargeTaskRequest-InitRequest) |  |  |
| init_task | [InitTaskRequest](#ArmoniK-api-grpc-v1-InitTaskRequest) |  |  |
| task_payload | [DataChunk](#ArmoniK-api-grpc-v1-DataChunk) |  |  |






<a name="ArmoniK-api-grpc-v1-ProcessReply-CreateLargeTaskRequest-InitRequest"></a>

### ProcessReply.CreateLargeTaskRequest.InitRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task_options | [TaskOptions](#ArmoniK-api-grpc-v1-TaskOptions) |  |  |






<a name="ArmoniK-api-grpc-v1-ProcessReply-CreateSmallTaskRequest"></a>

### ProcessReply.CreateSmallTaskRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| task_options | [TaskOptions](#ArmoniK-api-grpc-v1-TaskOptions) |  |  |
| task_requests | [TaskRequest](#ArmoniK-api-grpc-v1-TaskRequest) | repeated |  |






<a name="ArmoniK-api-grpc-v1-ProcessReply-DataRequest"></a>

### ProcessReply.DataRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| key | [string](#string) |  |  |






<a name="ArmoniK-api-grpc-v1-ProcessReply-Result"></a>

### ProcessReply.Result



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| init | [InitKeyedDataStream](#ArmoniK-api-grpc-v1-InitKeyedDataStream) |  |  |
| data | [DataChunk](#ArmoniK-api-grpc-v1-DataChunk) |  |  |






<a name="ArmoniK-api-grpc-v1-ProcessRequest"></a>

### ProcessRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| compute | [ProcessRequest.ComputeRequest](#ArmoniK-api-grpc-v1-ProcessRequest-ComputeRequest) |  |  |
| create_task | [ProcessRequest.CreateTask](#ArmoniK-api-grpc-v1-ProcessRequest-CreateTask) |  |  |
| resource | [ProcessRequest.DataReply](#ArmoniK-api-grpc-v1-ProcessRequest-DataReply) |  |  |
| common_data | [ProcessRequest.DataReply](#ArmoniK-api-grpc-v1-ProcessRequest-DataReply) |  |  |
| direct_data | [ProcessRequest.DataReply](#ArmoniK-api-grpc-v1-ProcessRequest-DataReply) |  |  |






<a name="ArmoniK-api-grpc-v1-ProcessRequest-ComputeRequest"></a>

### ProcessRequest.ComputeRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| init_request | [ProcessRequest.ComputeRequest.InitRequest](#ArmoniK-api-grpc-v1-ProcessRequest-ComputeRequest-InitRequest) |  |  |
| payload | [DataChunk](#ArmoniK-api-grpc-v1-DataChunk) |  |  |
| init_data | [ProcessRequest.ComputeRequest.InitData](#ArmoniK-api-grpc-v1-ProcessRequest-ComputeRequest-InitData) |  |  |
| data | [DataChunk](#ArmoniK-api-grpc-v1-DataChunk) |  |  |






<a name="ArmoniK-api-grpc-v1-ProcessRequest-ComputeRequest-InitData"></a>

### ProcessRequest.ComputeRequest.InitData



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| key | [string](#string) |  |  |
| last_data | [bool](#bool) |  |  |






<a name="ArmoniK-api-grpc-v1-ProcessRequest-ComputeRequest-InitRequest"></a>

### ProcessRequest.ComputeRequest.InitRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| configuration | [Configuration](#ArmoniK-api-grpc-v1-Configuration) |  |  |
| session_id | [string](#string) |  |  |
| task_id | [string](#string) |  |  |
| task_options | [ProcessRequest.ComputeRequest.InitRequest.TaskOptionsEntry](#ArmoniK-api-grpc-v1-ProcessRequest-ComputeRequest-InitRequest-TaskOptionsEntry) | repeated |  |
| expected_output_keys | [string](#string) | repeated |  |
| payload | [DataChunk](#ArmoniK-api-grpc-v1-DataChunk) |  |  |






<a name="ArmoniK-api-grpc-v1-ProcessRequest-ComputeRequest-InitRequest-TaskOptionsEntry"></a>

### ProcessRequest.ComputeRequest.InitRequest.TaskOptionsEntry



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| key | [string](#string) |  |  |
| value | [string](#string) |  |  |






<a name="ArmoniK-api-grpc-v1-ProcessRequest-CreateTask"></a>

### ProcessRequest.CreateTask



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| reply_id | [string](#string) |  |  |
| reply | [CreateTaskReply](#ArmoniK-api-grpc-v1-CreateTaskReply) |  |  |






<a name="ArmoniK-api-grpc-v1-ProcessRequest-DataReply"></a>

### ProcessRequest.DataReply



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| reply_id | [string](#string) |  |  |
| init | [ProcessRequest.DataReply.Init](#ArmoniK-api-grpc-v1-ProcessRequest-DataReply-Init) |  |  |
| data | [DataChunk](#ArmoniK-api-grpc-v1-DataChunk) |  |  |






<a name="ArmoniK-api-grpc-v1-ProcessRequest-DataReply-Init"></a>

### ProcessRequest.DataReply.Init



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| key | [string](#string) |  |  |
| data | [DataChunk](#ArmoniK-api-grpc-v1-DataChunk) |  |  |
| error | [string](#string) |  |  |





 


<a name="ArmoniK-api-grpc-v1-HealthCheckReply-ServingStatus"></a>

### HealthCheckReply.ServingStatus


| Name | Number | Description |
| ---- | ------ | ----------- |
| UNKNOWN | 0 |  |
| SERVING | 1 |  |
| NOT_SERVING | 2 |  |


 

 


<a name="ArmoniK-api-grpc-v1-Worker"></a>

### Worker


| Method Name | Request Type | Response Type | Description |
| ----------- | ------------ | ------------- | ------------|
| Process | [ProcessRequest](#ArmoniK-api-grpc-v1-ProcessRequest) stream | [ProcessReply](#ArmoniK-api-grpc-v1-ProcessReply) stream |  |
| HealthCheck | [Empty](#ArmoniK-api-grpc-v1-Empty) | [HealthCheckReply](#ArmoniK-api-grpc-v1-HealthCheckReply) |  |

 



## Scalar Value Types

| .proto Type | Notes | C++ | Java | Python | Go | C# | PHP | Ruby |
| ----------- | ----- | --- | ---- | ------ | -- | -- | --- | ---- |
| <a name="double" /> double |  | double | double | float | float64 | double | float | Float |
| <a name="float" /> float |  | float | float | float | float32 | float | float | Float |
| <a name="int32" /> int32 | Uses variable-length encoding. Inefficient for encoding negative numbers – if your field is likely to have negative values, use sint32 instead. | int32 | int | int | int32 | int | integer | Bignum or Fixnum (as required) |
| <a name="int64" /> int64 | Uses variable-length encoding. Inefficient for encoding negative numbers – if your field is likely to have negative values, use sint64 instead. | int64 | long | int/long | int64 | long | integer/string | Bignum |
| <a name="uint32" /> uint32 | Uses variable-length encoding. | uint32 | int | int/long | uint32 | uint | integer | Bignum or Fixnum (as required) |
| <a name="uint64" /> uint64 | Uses variable-length encoding. | uint64 | long | int/long | uint64 | ulong | integer/string | Bignum or Fixnum (as required) |
| <a name="sint32" /> sint32 | Uses variable-length encoding. Signed int value. These more efficiently encode negative numbers than regular int32s. | int32 | int | int | int32 | int | integer | Bignum or Fixnum (as required) |
| <a name="sint64" /> sint64 | Uses variable-length encoding. Signed int value. These more efficiently encode negative numbers than regular int64s. | int64 | long | int/long | int64 | long | integer/string | Bignum |
| <a name="fixed32" /> fixed32 | Always four bytes. More efficient than uint32 if values are often greater than 2^28. | uint32 | int | int | uint32 | uint | integer | Bignum or Fixnum (as required) |
| <a name="fixed64" /> fixed64 | Always eight bytes. More efficient than uint64 if values are often greater than 2^56. | uint64 | long | int/long | uint64 | ulong | integer/string | Bignum |
| <a name="sfixed32" /> sfixed32 | Always four bytes. | int32 | int | int | int32 | int | integer | Bignum or Fixnum (as required) |
| <a name="sfixed64" /> sfixed64 | Always eight bytes. | int64 | long | int/long | int64 | long | integer/string | Bignum |
| <a name="bool" /> bool |  | bool | boolean | boolean | bool | bool | boolean | TrueClass/FalseClass |
| <a name="string" /> string | A string must always contain UTF-8 encoded or 7-bit ASCII text. | string | String | str/unicode | string | string | string | String (UTF-8) |
| <a name="bytes" /> bytes | May contain any arbitrary sequence of bytes. | string | ByteString | str | []byte | ByteString | string | String (ASCII-8BIT) |

