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

using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace ArmoniK.Api.Worker.Utils;

public class ApplicationLifeTimeManager
{
  private readonly IHostApplicationLifetime            lifetime_;
  private readonly ILogger<ApplicationLifeTimeManager> logger_;

  public ApplicationLifeTimeManager(ILogger<ApplicationLifeTimeManager> logger,
                                    IHostApplicationLifetime            lifetime)
  {
    logger_   = logger;
    lifetime_ = lifetime;
    lifetime_.ApplicationStopping.Register(GracefulTerminationStarted);
    lifetime_.ApplicationStopped.Register(GracefulTerminationFinished);
    lifetime_.ApplicationStarted.Register(ApplicationStarted);
  }

  private void GracefulTerminationStarted()
    => logger_.LogWarning("Application host is starting graceful termination");

  private void GracefulTerminationFinished()
    => logger_.LogWarning("Application host has finished graceful termination");

  private void ApplicationStarted()
    => logger_.LogWarning("Application host has finished starting");
}
