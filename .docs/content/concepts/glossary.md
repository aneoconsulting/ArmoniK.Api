# Glossary

Terms used throughout the how-to guides and API reference, defined once here.

## Control plane

The ArmoniK component (implemented in [ArmoniK.Core](https://github.com/aneoconsulting/ArmoniK.Core); see the
[ArmoniK documentation](https://armonik.readthedocs.io/en/latest/index.html) for how to deploy it) that clients
connect to. It exposes the [Sessions](../api-reference/proto.md), Tasks, Results and Partitions services, schedules
submitted tasks onto the compute plane, and tracks their status and results.

## Compute plane

The pool of worker processes that actually execute tasks. Workers implement the `Worker` gRPC service
(`Process`, `HealthCheck`) so the control plane can dispatch work to them, and call back into the control plane's
`Agent` service while processing a task (to create sub-tasks or results).

## Session

A submission context, created with `Sessions.CreateSession`. A session fixes the default `TaskOptions` (duration,
priority, retries, partition) and the list of partitions its tasks may be submitted to. Every task and result
belongs to exactly one session.

## Task

A unit of computation submitted with `Tasks.SubmitTasks`. A task references a payload (an input result), any
number of data-dependency results it needs before it can run, and the result IDs it is expected to produce. Its
`TaskOptions` (inherited from the session unless overridden) control its duration limit, priority, retry count and
which partition it runs in.

## Result

A named piece of data, identified by a `result_id`, managed by the `Results` service. A result can be created
empty and uploaded to later (`CreateResultsMetaData` + `UploadResultData`) or created with its data in one call
(`CreateResults`). Tasks reference results both as inputs (data dependencies / payload) and as declared outputs
(expected output IDs).

## Partition

A named pool of compute resources, managed by the `Partitions` service. Sessions declare which partitions they can
submit to; partitions are how workloads are isolated or prioritized across different pools of workers.

## Submitter

```{note}
Deprecated. Every RPC in the `Submitter` service is marked `deprecated` in the proto. Use `Sessions`, `Tasks`,
`Results` and `Partitions` instead — this glossary and the concepts guides only describe those.
```

## Events

A streaming service (`Events.GetEvents`) for being notified about task and result changes without polling — useful
when [waiting on a result](submit-and-retrieve-results.md) at scale.

## Agent vs. Submitter vs. client services

Two different gRPC surfaces exist in the API:

- **Client-facing**: `Sessions`, `Tasks`, `Results`, `Partitions`, `Versions`, `Events` — what a client library
  (Python, C#, Angular, C++) calls to submit work and read results. This is what the rest of these docs cover.
- **Worker-facing**: `Agent` (called by a worker while it's processing a task, to create its own sub-tasks/results)
  and `Worker` (called by the control plane to dispatch a task to a worker). Relevant only if you're implementing a
  worker, not a client.
