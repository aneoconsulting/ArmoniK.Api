# C# packages

This page lists the namespaces in the C# projects of this repository and the classes available in each.

## ArmoniK.Api.Client.Options

This namespace contains options classes to configure the client connection to the ArmoniK control plane.

## ArmoniK.Api.Client.Submitter

This namespace includes some utilitarian classes for interacting with the ArmoniK control plane.
It also contains the generated gRPC classes built from the protobuf files used by the client to connect to the ArmoniK control plane.

## ArmoniK.Api.Common.Channel.Utils, ArmoniK.Api.Common.Options

They contain classes to create and configure (through options) gRPC channels between ArmoniK workers and polling agents.

## ArmoniK.Api.Core

It includes the generated gRPC classes built from the protobuf files used by [ArmoniK.Core](https://github.com/aneoconsulting/ArmoniK.Core).

## ArmoniK.Api.Common.Utils

It contains helpers that are widely used in ArmoniK.

## ArmoniK.Api.Worker.Tests

This namespace contains the test classes for the worker.

## ArmoniK.Api.Worker.Worker, ArmoniK.Api.Worker.Utils

They contain helper classes to create a .Net 6 worker that implements ArmoniK interfaces and executes the computations submitted to the control plane.
It also includes the generated gRPC classes built from the protobuf files used by the workers that execute the computations.
