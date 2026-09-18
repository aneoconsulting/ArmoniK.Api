//! Build every payload of `shapes.json` as prost values.
//!
//! One function per message, mirroring the *rules* `ffi/schema/emit/payloads.py` states,
//! not its code: this is the independent side of the stage 1 check. Encoding is entirely
//! prost's, so what the comparison tests is `emit/wire.py`'s semantics.

use shapes_prost::shapes as p;
use shapes_values as v;
use std::collections::BTreeMap;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Full,
    /// P1.3: every field of every element dropped, so an element encodes to nothing.
    AllAbsent,
    /// P2.5: half the map values emptied, the adapter child removed, half the timestamps absent.
    HalfAbsent,
}

fn ts(idx: i64) -> p::Timestamp {
    let (seconds, nanos) = v::timestamp(idx);
    p::Timestamp { seconds, nanos }
}

fn dur(idx: i64) -> p::Duration {
    let (seconds, nanos) = v::duration(idx);
    p::Duration { seconds, nanos }
}

/// `absent()` for the `half_absent` mode: a Timestamp-typed message field with an even tag.
fn half_ts(mode: Mode, tag: u32, idx: i64) -> Option<p::Timestamp> {
    if mode == Mode::HalfAbsent && tag % 2 == 0 {
        None
    } else {
        Some(ts(idx))
    }
}

pub fn result_raw(idx: i64, mode: Mode) -> p::ResultRaw {
    if mode == Mode::AllAbsent {
        // `absent()` returns true for every field, so the element body is empty.
        return p::ResultRaw::default();
    }
    p::ResultRaw {
        session_id: v::guid("ResultRaw.session_id", idx),
        name: v::word("ResultRaw.name", idx),
        owner_task_id: v::guid("ResultRaw.owner_task_id", idx),
        status: v::enum_value(&v::RESULT_STATUS, idx),
        created_at: Some(ts(idx)),
        completed_at: Some(ts(idx)),
        result_id: v::guid("ResultRaw.result_id", idx),
        size: v::scalar_i64("ResultRaw.size", idx),
        created_by: v::guid("ResultRaw.created_by", idx),
        opaque_id: v::blob("ResultRaw.opaque_id", idx, 16),
        manual_deletion: v::scalar_bool("ResultRaw.manual_deletion", idx),
    }
}

pub fn task_options(path: &str, idx: i64, mode: Mode) -> p::TaskOptions {
    let mut options = BTreeMap::new();
    let kp = format!("{path}.options.key");
    let vp = format!("{path}.options.value");
    for k in 0..4i64 {
        let key = format!("k{:02}-{}", k, v::word(&kp, idx * 31 + k));
        let val = if mode == Mode::HalfAbsent && k % 2 == 0 {
            String::new()
        } else {
            v::word(&vp, idx * 31 + k)
        };
        options.insert(key, val);
    }
    p::TaskOptions {
        options,
        max_duration: Some(dur(idx)),
        max_retries: v::scalar_i32(&format!("{path}.max_retries"), idx),
        priority: v::scalar_i32(&format!("{path}.priority"), idx),
        partition_id: v::word(&format!("{path}.partition_id"), idx),
        application_name: v::word(&format!("{path}.application_name"), idx),
        application_version: v::word(&format!("{path}.application_version"), idx),
        application_namespace: v::word(&format!("{path}.application_namespace"), idx),
        application_service: v::word(&format!("{path}.application_service"), idx),
        engine_type: v::word(&format!("{path}.engine_type"), idx),
    }
}

fn guids(path: &str, idx: i64, n: i64) -> Vec<String> {
    (0..n).map(|j| v::guid(path, idx * 211 + j)).collect()
}

pub fn task_detailed(idx: i64, mode: Mode, repeats: i64) -> p::TaskDetailed {
    if mode == Mode::AllAbsent {
        return p::TaskDetailed::default();
    }
    p::TaskDetailed {
        id: v::guid("TaskDetailed.id", idx),
        session_id: v::guid("TaskDetailed.session_id", idx),
        owner_pod_id: v::guid("TaskDetailed.owner_pod_id", idx),
        parent_task_ids: guids("TaskDetailed.parent_task_ids", idx, repeats),
        data_dependencies: guids("TaskDetailed.data_dependencies", idx, repeats),
        expected_output_ids: guids("TaskDetailed.expected_output_ids", idx, repeats),
        retry_of_ids: guids("TaskDetailed.retry_of_ids", idx, repeats),
        status: v::enum_value(&v::TASK_STATUS, idx),
        status_message: v::sentence("TaskDetailed.status_message", idx),
        options: Some(task_options("TaskDetailed.options", idx, mode)),
        created_at: half_ts(mode, 11, idx),
        submitted_at: half_ts(mode, 12, idx),
        started_at: half_ts(mode, 13, idx),
        ended_at: half_ts(mode, 14, idx),
        pod_ttl: half_ts(mode, 15, idx),
        output: if mode == Mode::HalfAbsent {
            None
        } else {
            Some(p::TaskOutput {
                success: v::scalar_bool("TaskDetailed.output.success", idx),
                error: v::sentence("TaskDetailed.output.error", idx),
            })
        },
        pod_hostname: v::word("TaskDetailed.pod_hostname", idx),
        received_at: half_ts(mode, 18, idx),
        acquired_at: half_ts(mode, 19, idx),
        creation_to_end_duration: Some(dur(idx)),
        processing_to_end_duration: Some(dur(idx)),
        initial_task_id: v::guid("TaskDetailed.initial_task_id", idx),
        received_to_end_duration: Some(dur(idx)),
        processed_at: half_ts(mode, 24, idx),
        fetched_at: half_ts(mode, 25, idx),
        payload_id: v::guid("TaskDetailed.payload_id", idx),
        created_by: v::guid("TaskDetailed.created_by", idx),
    }
}

pub fn task_summary(idx: i64) -> p::TaskSummary {
    p::TaskSummary {
        id: v::guid("TaskSummary.id", idx),
        session_id: v::guid("TaskSummary.session_id", idx),
        options: Some(task_options("TaskSummary.options", idx, Mode::Full)),
        status: v::enum_value(&v::TASK_STATUS, idx),
        created_at: Some(ts(idx)),
        error: v::sentence("TaskSummary.error", idx),
        status_message: v::sentence("TaskSummary.status_message", idx),
        count_data_dependencies: v::scalar_i64("TaskSummary.count_data_dependencies", idx),
    }
}

/// Presence cycles per field so that an `optional` field is sometimes absent; one element
/// in seven is present AND zero, which is the case a by-value group cannot tell from absent
/// unless it is designed to.
fn explicit_present(tag: i64, idx: i64) -> bool {
    idx % (tag + 1) != 0
}

pub fn probe(idx: i64) -> p::Probe {
    use p::probe::Body;
    let body = match idx % 5 {
        0 => Body::AsInt(v::scalar_i64("Probe.as_int", idx)),
        1 => Body::AsText(v::word("Probe.as_text", idx)),
        2 => Body::AsBlob(v::blob("Probe.as_blob", idx, 16)),
        3 => Body::AsStamp(ts(idx)),
        _ => Body::AsNothing(p::Empty {}),
    };
    p::Probe {
        id: v::guid("Probe.id", idx),
        opt_count: explicit_present(2, idx).then(|| {
            if idx % 7 == 0 {
                0
            } else {
                v::scalar_i32("Probe.opt_count", idx)
            }
        }),
        opt_label: explicit_present(3, idx).then(|| {
            if idx % 7 == 0 {
                String::new()
            } else {
                v::word("Probe.opt_label", idx)
            }
        }),
        opt_flag: explicit_present(4, idx).then(|| {
            if idx % 7 == 0 {
                false
            } else {
                v::scalar_bool("Probe.opt_flag", idx)
            }
        }),
        body: Some(body),
    }
}

pub fn metrics_batch(idx: i64) -> p::MetricsBatch {
    let n = 30i64;
    p::MetricsBatch {
        id: v::guid("MetricsBatch.id", idx),
        ticks: (0..n)
            .map(|j| v::scalar_i64("MetricsBatch.ticks", idx * 97 + j))
            .collect(),
        values: (0..n)
            .map(|j| v::scalar_f64("MetricsBatch.values", idx * 97 + j))
            .collect(),
        codes: (0..n)
            .map(|j| v::scalar_i32("MetricsBatch.codes", idx * 97 + j))
            .collect(),
        flags: (0..n)
            .map(|j| v::scalar_bool("MetricsBatch.flags", idx * 97 + j))
            .collect(),
        // The one packed shape the real schema has. Cycled by the same rule the singular
        // enums use, so a run reaches the large value too, and a packed field is written
        // even when every member is the proto zero: the omit-when-zero rule is about a
        // leaf, not about a run.
        statuses: (0..n)
            .map(|j| v::enum_value(&v::TASK_STATUS, idx * 97 + j))
            .collect(),
    }
}

pub fn pair(idx: i64) -> p::Pair {
    p::Pair {
        key: v::word("Pair.key", idx),
        value: v::scalar_i32("Pair.value", idx),
    }
}

pub fn upload(bulk: usize) -> p::UploadResultDataMessage {
    p::UploadResultDataMessage {
        upload: Some(p::UploadResultData {
            session_id: v::guid("UploadResultData.session_id", 0),
            result_id: v::guid("UploadResultData.result_id", 0),
            data_chunk: v::bulk(bulk),
        }),
    }
}

pub fn list_results(count: i64, mode: Mode) -> p::ListResultsResponse {
    p::ListResultsResponse {
        results: (0..count).map(|j| result_raw(j, mode)).collect(),
        page: 1,
        total: count as i32,
    }
}

pub fn list_tasks(count: i64, mode: Mode, repeats: &[i64]) -> p::ListTasksDetailedResponse {
    p::ListTasksDetailedResponse {
        tasks: (0..count)
            .map(|j| task_detailed(j, mode, repeats[(j as usize) % repeats.len()]))
            .collect(),
        page: 1,
        total: count as i32,
    }
}

pub fn list_summaries(count: i64) -> p::ListTaskSummaryResponse {
    p::ListTaskSummaryResponse {
        tasks: (0..count).map(task_summary).collect(),
    }
}

pub fn list_probes(count: i64) -> p::ListProbeResponse {
    p::ListProbeResponse {
        probes: (0..count).map(probe).collect(),
    }
}

pub fn list_metrics(count: i64) -> p::ListMetricsResponse {
    p::ListMetricsResponse {
        batches: (0..count).map(metrics_batch).collect(),
    }
}

/// P7.1: the elements, in the order the wire carries them. prost writes each field
/// contiguously, so this value cannot reproduce the interleaving; stage 1 checks P7.1
/// by decoding instead.
pub fn dual(count: i64) -> p::DualResponse {
    p::DualResponse {
        left: (0..count).map(pair).collect(),
        right: (0..count).map(pair).collect(),
    }
}
