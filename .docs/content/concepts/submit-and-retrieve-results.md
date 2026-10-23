# Submit a task and retrieve its result

The core ArmoniK workflow: create a session, submit a task with an input payload, wait for it to finish, and
download its output. This walks through it end-to-end in Python — see the [glossary](glossary.md) for what each
term means.

```{important}
This requires a reachable ArmoniK control plane (see the [Quickstart](../getting-started/quickstart.md)) and a
compute plane with at least one worker attached to the target partition, otherwise the task will sit `SUBMITTED`
and never complete.
```

```python
import time
from datetime import timedelta

from armonik.client import ArmoniKSessions, ArmoniKTasks, ArmoniKResults
from armonik.common import TaskDefinition, TaskOptions, TaskStatus, create_channel

channel = create_channel("localhost:5001")
sessions_client = ArmoniKSessions(channel)
tasks_client = ArmoniKTasks(channel)
results_client = ArmoniKResults(channel)

# 1. Create a session. TaskOptions set here are the defaults for every task
#    submitted to it, unless a task overrides them.
task_options = TaskOptions(max_duration=timedelta(seconds=300), priority=1, max_retries=3)
session_id = sessions_client.create_session(task_options)

# 2. Create the results the task needs: one payload with data already attached,
#    and one empty output result the worker will fill in.
payload = results_client.create_results({"payload": b"my input data"}, session_id)
output = results_client.create_results_metadata(["output"], session_id)

# 3. Submit the task, referencing those result IDs.
task = tasks_client.submit_tasks(
    session_id,
    [
        TaskDefinition(
            payload_id=payload["payload"].result_id,
            expected_output_ids=[output["output"].result_id],
        )
    ],
)[0]

# 4. Wait for it to reach a terminal status.
while True:
    status = tasks_client.get_task(task.id).status
    if status in (TaskStatus.COMPLETED, TaskStatus.ERROR):
        break
    time.sleep(1)

# 5. Download the result.
data = results_client.download_result_data(output["output"].result_id, session_id)
```

`submit_tasks` and `create_results`/`create_results_metadata` all accept lists, so the same shape submits many
tasks or results in one call — see the batching (`chunk_size`) parameters in the
[reference](../api-reference/overview.md) for large workloads.

## Gotchas

- **Polling vs. events**: the loop above polls `GetTask` once a second, which is fine for a handful of tasks. For
  larger volumes, subscribe to the `Events` service (see the [glossary](glossary.md#events)) instead of polling
  every task individually.
- **Retries are automatic, at the channel level**: the C#/C++ clients retry `Unavailable`, `Aborted` and `Unknown`
  gRPC statuses with exponential backoff (5 attempts, 1s initial backoff, ×1.5 multiplier by default — see
  [Environment Variables](../how-to/envars/index.rst)). A task itself is retried up to `max_retries` times by the
  control plane if it fails during execution — these are two different retry mechanisms.
- **Submission is not idempotent**: calling `submit_tasks` twice with the same `TaskDefinition` creates two
  separate tasks, each with its own ID. Creating results is likewise not deduplicated by name — create each result
  once and hold on to its ID.
- **`expected_output_ids` must be non-empty**: a `TaskDefinition` with no declared outputs raises a `ValueError`
  client-side before it's even sent — every task must produce at least one result.
- **Connection issues**: see [Troubleshooting](../troubleshooting/index.md) for `UNAVAILABLE`/certificate-mismatch
  errors when the control plane isn't reachable or TLS isn't configured correctly.
