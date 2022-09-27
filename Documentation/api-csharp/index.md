---
uid: armonik_api_csharp
---

# ArmoniK API C#

This page lists the namespaces in the C# projects of this repository.
In each namespace, the included classes are available.

## ArmoniK.Api.Client.Options
In this namespace, there are options classes to configure the client connection to ArmoniK control plane.

## ArmoniK.Api.Client.Submitter
This namespace includes some utilitarian classes for interaction with ArmoniK control plane.

## ArmoniK.Api.Common.Channel.Utils, ArmoniK.Api.Common.Options
They contain some classes to create and configure (though options) Grpc Channels between ArmoniK workers and polling agents.

# ArmoniK.Api.Common.Utils
It contains helpers that are widely used in ArmoniK.

# ArmoniK.Api.Worker.Tests
This namespace contains the test classes for the worker.

# ArmoniK.Api.Worker.Worker, ArmoniK.Api.Worker.Utils
They contain helper classes to create a .Net 6 worker that implements ArmoniK interfaces and executes the computations submitted to the control plane.
