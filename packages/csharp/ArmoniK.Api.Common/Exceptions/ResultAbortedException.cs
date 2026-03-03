// This file is part of the ArmoniK project
// 
// Copyright (C) ANEO, 2021-2026. All rights reserved.
// 
// Licensed under the Apache License, Version 2.0 (the "License")
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// 
//     http://www.apache.org/licenses/LICENSE-2.0
// 
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

using System;
using System.Collections.Generic;

namespace ArmoniK.Api.Common.Exceptions;

/// <summary>
///   Exception raised when results are aborted
/// </summary>
public class ResultAbortedException : Exception
{
  /// <summary>
  ///   Initializes a new instance of the <see cref="ResultAbortedException" /> with the specified error message
  /// </summary>
  /// <param name="message">The error message</param>
  /// <param name="abortedResultId">The aborted Result's id</param>
  /// <param name="completedResultIds">The completed Results's Ids</param>
  public ResultAbortedException(string       message,
                                string       abortedResultId,
                                List<string> completedResultIds)
    : base(message)
  {
    AbortedResultId    = abortedResultId;
    CompletedResultIds = completedResultIds;
  }

  /// <summary>
  ///    The aborted Result's id
  /// </summary>
  public string AbortedResultId { get; }

  /// <summary>
  ///    The already completed Results's Ids when the exception is raised
  /// </summary>
  public IEnumerable<string> CompletedResultIds { get; }
}
