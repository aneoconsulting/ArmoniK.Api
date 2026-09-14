use armonik::server::TasksServiceExt;
use armonik::tasks;

#[macro_use]
mod common;

rpc_tests! {
    client: into_tasks;
    server: TasksService, tasks_server;
    mock: "Tasks";

    rpc unary list {
        request: tasks::list::Request {
            filters: tasks::filter::Or::default(),
            sort: tasks::Sort::default(),
            with_errors: false,
            page: 3,
            page_size: 12,
        },
        respond: |request| tasks::list::Response {
            tasks: vec![tasks::Summary {
                task_id: String::from("rpc-list-output"),
                ..Default::default()
            }],
            page: request.page,
            page_size: request.page_size,
            total: 1337,
        },
        convenience: list(
            tasks::filter::Or::default(),
            tasks::Sort::default(),
            false,
            3,
            12,
        ),
        check: |response| {
            assert_eq!(response.page, 3);
            assert_eq!(response.page_size, 12);
            assert_eq!(response.total, 1337);
            assert_eq!(response.tasks[0].task_id, "rpc-list-output");
        },
    }

    rpc unary list_detailed {
        request: tasks::list_detailed::Request {
            filters: tasks::filter::Or::default(),
            sort: tasks::Sort::default(),
            with_errors: false,
            page: 3,
            page_size: 12,
        },
        respond: |request| tasks::list_detailed::Response {
            tasks: vec![tasks::Raw {
                task_id: String::from("rpc-list-detailed-output"),
                ..Default::default()
            }],
            page: request.page,
            page_size: request.page_size,
            total: 1338,
        },
        convenience: list_detailed(
            tasks::filter::Or::default(),
            tasks::Sort::default(),
            false,
            3,
            12,
        ),
        check: |response| {
            assert_eq!(response.page, 3);
            assert_eq!(response.page_size, 12);
            assert_eq!(response.total, 1338);
            assert_eq!(response.tasks[0].task_id, "rpc-list-detailed-output");
        },
    }

    rpc unary get {
        request: tasks::get::Request {
            task_id: String::from("rpc-get-input"),
        },
        respond: |request| tasks::get::Response {
            task: tasks::Raw {
                session_id: String::from("rpc-get-output"),
                task_id: request.task_id,
                ..Default::default()
            },
        },
        convenience: get("rpc-get-input"),
        project: |response| response.task,
        check: |task| {
            assert_eq!(task.task_id, "rpc-get-input");
            assert_eq!(task.session_id, "rpc-get-output");
        },
    }

    rpc unary cancel {
        request: tasks::cancel::Request {
            task_ids: vec![String::from("rpc-cancel-input")],
        },
        respond: |request| tasks::cancel::Response {
            tasks: request
                .task_ids
                .into_iter()
                .map(|task_id| tasks::Summary {
                    session_id: String::from("rpc-cancel-output"),
                    task_id,
                    ..Default::default()
                })
                .collect(),
        },
        convenience: cancel(["rpc-cancel-input"]),
        project: |response| response.tasks,
        check: |summaries| {
            assert_eq!(summaries[0].task_id, "rpc-cancel-input");
            assert_eq!(summaries[0].session_id, "rpc-cancel-output");
        },
    }

    rpc unary get_result_ids {
        request: tasks::get_result_ids::Request {
            task_ids: vec![String::from("rpc-get-result-ids-input")],
        },
        respond: |request| tasks::get_result_ids::Response {
            task_results: request
                .task_ids
                .into_iter()
                .map(|task_id| (task_id, vec![String::from("rpc-get-result-ids-output")]))
                .collect(),
        },
        convenience: get_result_ids(["rpc-get-result-ids-input"]),
        project: |response| response.task_results,
        check: |task_results| {
            assert_eq!(
                task_results["rpc-get-result-ids-input"][0],
                "rpc-get-result-ids-output"
            );
        },
    }

    rpc unary count_status {
        request: tasks::count_status::Request {
            filters: tasks::filter::Or::default(),
        },
        respond: |_request| tasks::count_status::Response {
            status: vec![armonik::StatusCount {
                status: armonik::TaskStatus::Creating,
                count: 1337,
            }],
        },
        convenience: count_status(tasks::filter::Or::default()),
        project: |response| response.status,
        check: |counts| {
            assert_eq!(counts[0].status, armonik::TaskStatus::Creating);
            assert_eq!(counts[0].count, 1337);
        },
    }

    rpc unary submit {
        request: tasks::submit::Request {
            session_id: String::from("session-id"),
            task_options: None,
            items: vec![tasks::submit::RequestItem {
                payload_id: String::from("rpc-submit-input"),
                ..Default::default()
            }],
        },
        respond: |request| tasks::submit::Response {
            items: request
                .items
                .into_iter()
                .map(|item| tasks::submit::ResponseItem {
                    task_id: String::from("rpc-submit-output"),
                    payload_id: item.payload_id,
                    ..Default::default()
                })
                .collect(),
        },
        convenience: submit(
            "session-id",
            None,
            [tasks::submit::RequestItem {
                payload_id: String::from("rpc-submit-input"),
                ..Default::default()
            }],
        ),
        project: |response| response.items,
        check: |items| {
            assert_eq!(items[0].payload_id, "rpc-submit-input");
            assert_eq!(items[0].task_id, "rpc-submit-output");
        },
    }
}
