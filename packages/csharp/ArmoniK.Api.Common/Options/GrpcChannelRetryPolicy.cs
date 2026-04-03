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

using JetBrains.Annotations;

namespace ArmoniK.Api.Common.Options;

/// <summary>
///   Retry policy options for a gRPC channel.
///   Use the fluent <c>With*</c> methods to override individual values at runtime.
/// </summary>
[PublicAPI]
public sealed class GrpcChannelRetryPolicy
{
  /// <summary>
  ///   Maximum number of gRPC retry attempts for transient failures.
  ///   Set to 0 or 1 to disable native gRPC retries.
  ///   See: https://learn.microsoft.com/en-us/aspnet/core/grpc/retries
  /// </summary>
  public int MaxAttempts { get; set; } = 5;

  /// <summary>
  ///   Initial backoff delay before the first retry attempt.
  /// </summary>
  public TimeSpan InitialBackoff { get; set; } = TimeSpan.FromSeconds(1);

  /// <summary>
  ///   Maximum backoff delay between retry attempts.
  /// </summary>
  public TimeSpan MaxBackoff { get; set; } = TimeSpan.FromSeconds(10);

  /// <summary>
  ///   Multiplier applied to the backoff after each retry attempt.
  /// </summary>
  public double BackoffMultiplier { get; set; } = 2.0;

  /// <summary>
  ///   Sets the maximum number of gRPC retry attempts.
  /// </summary>
  /// <param name="value">Maximum retry attempts.</param>
  /// <returns>The same instance.</returns>
  public GrpcChannelRetryPolicy WithMaxAttempts(int value)
  {
    MaxAttempts = value;
    return this;
  }

  /// <summary>
  ///   Sets the initial backoff delay before the first retry attempt.
  /// </summary>
  /// <param name="value">Initial backoff duration.</param>
  /// <returns>The same instance.</returns>
  public GrpcChannelRetryPolicy WithInitialBackoff(TimeSpan value)
  {
    InitialBackoff = value;
    return this;
  }

  /// <summary>
  ///   Sets the maximum backoff delay between retry attempts.
  /// </summary>
  /// <param name="value">Maximum backoff duration.</param>
  /// <returns>The same instance.</returns>
  public GrpcChannelRetryPolicy WithMaxBackoff(TimeSpan value)
  {
    MaxBackoff = value;
    return this;
  }

  /// <summary>
  ///   Sets the multiplier applied to the backoff after each retry attempt.
  /// </summary>
  /// <param name="value">Backoff multiplier.</param>
  /// <returns>The same instance.</returns>
  public GrpcChannelRetryPolicy WithBackoffMultiplier(double value)
  {
    BackoffMultiplier = value;
    return this;
  }
}
