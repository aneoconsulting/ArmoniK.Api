// This file is part of the ArmoniK project
// 
// Copyright (C) ANEO, 2021-2022. All rights reserved.
//   W. Kirschenmann   <wkirschenmann@aneo.fr>
//   J. Gurhem         <jgurhem@aneo.fr>
//   D. Dubuc          <ddubuc@aneo.fr>
//   L. Ziane Khodja   <lzianekhodja@aneo.fr>
//   F. Lemaitre       <flemaitre@aneo.fr>
//   S. Djebbar        <sdjebbar@aneo.fr>
//   J. Fonseca        <jfonseca@aneo.fr>
// 
// Licensed under the Apache License, Version 2.0 (the "License")
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// 
//         http://www.apache.org/licenses/LICENSE-2.0
// 
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

using System;
using System.Collections.Generic;
using System.Threading.Tasks;

using ArmoniK.Api.gRPC.V1;
using ArmoniK.Api.gRPC.V1.Agent;

using JetBrains.Annotations;

namespace ArmoniK.Api.Worker.Worker;

[PublicAPI]
public interface ITaskHandler : IAsyncDisposable
{
  /// <summary>
  ///   Id of the session this task belongs to.
  /// </summary>
  string SessionId { get; }

  /// <summary>
  ///   Id of the task being processed.
  /// </summary>
  string TaskId { get; }

  /// <summary>
  ///   List of options provided when submitting the task.
  /// </summary>
  TaskOptions TaskOptions { get; }

  /// <summary>
  ///   The data provided when submitting the task.
  /// </summary>
  byte[] Payload { get; }

  /// <summary>
  ///   The data required to compute the task
  /// </summary>
  IReadOnlyDictionary<string, byte[]> DataDependencies { get; }

  /// <summary>
  ///   Lists the result that should be provided or delegated by this task.
  /// </summary>
  IList<string> ExpectedResults { get; }

  /// <summary>
  ///   The configuration parameters for the interaction with ArmoniK.
  /// </summary>
  Configuration? Configuration { get; }

  /// <summary>
  ///   This method allows to create subtasks.
  /// </summary>
  /// <param name="tasks">Lists the tasks to submit</param>
  /// <param name="taskOptions">The task options. If no value is provided, will use the default session options</param>
  /// <returns></returns>
  Task<CreateTaskReply> CreateTasksAsync(IEnumerable<TaskRequest> tasks,
                                         TaskOptions?             taskOptions = null);

  /// <summary>
  ///   NOT IMPLEMENTED
  ///   This method is used to retrieve data available system-wide.
  /// </summary>
  /// <param name="key">The data key identifier</param>
  /// <returns></returns>
  Task<byte[]> RequestResource(string key);

  /// <summary>
  ///   NOT IMPLEMENTED
  ///   This method is used to retrieve data provided when creating the session.
  /// </summary>
  /// <param name="key">The da ta key identifier</param>
  /// <returns></returns>
  Task<byte[]> RequestCommonData(string key);

  /// <summary>
  ///   NOT IMPLEMENTED
  ///   This method is used to retrieve data directly from the submission client.
  /// </summary>
  /// <param name="key"></param>
  /// <returns></returns>
  Task<byte[]> RequestDirectData(string key);

  /// <summary>
  ///   Send the results computed by the task
  /// </summary>
  /// <param name="key">The key identifier of the result.</param>
  /// <param name="data">The data corresponding to the result</param>
  /// <returns></returns>
  Task SendResult(string key,
                  byte[] data);
}
