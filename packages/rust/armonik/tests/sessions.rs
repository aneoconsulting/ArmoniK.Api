use armonik::server::SessionsServiceExt;
use armonik::sessions;

#[macro_use]
mod common;

/// Every RPC that answers with a session echoes the id back and tags the reply
/// with its own sentinel partition, so a response identifies the handler that
/// produced it.
fn session(session_id: String, sentinel: &str) -> sessions::Raw {
    sessions::Raw {
        session_id,
        partition_ids: vec![String::from(sentinel)],
        ..Default::default()
    }
}

rpc_tests! {
    client: into_sessions;
    server: SessionsService, sessions_server;
    mock: "Sessions";

    rpc unary list {
        request: sessions::list::Request {
            filters: sessions::filter::Or::default(),
            sort: sessions::Sort::default(),
            with_task_options: true,
            page: 3,
            page_size: 12,
        },
        respond: |request| sessions::list::Response {
            sessions: vec![sessions::Raw {
                session_id: String::from("rpc-list-output"),
                ..Default::default()
            }],
            page: request.page,
            page_size: request.page_size,
            total: 1337,
        },
        convenience: list(
            sessions::filter::Or::default(),
            sessions::Sort::default(),
            true,
            3,
            12,
        ),
        check: |response| {
            assert_eq!(response.page, 3);
            assert_eq!(response.page_size, 12);
            assert_eq!(response.total, 1337);
            assert_eq!(response.sessions[0].session_id, "rpc-list-output");
        },
    }

    rpc unary get {
        request: sessions::get::Request {
            session_id: String::from("rpc-get-input"),
        },
        respond: |request| sessions::get::Response {
            session: session(request.session_id, "rpc-get-output"),
        },
        convenience: get("rpc-get-input"),
        project: |response| response.session,
        check: |session| {
            assert_eq!(session.session_id, "rpc-get-input");
            assert_eq!(session.partition_ids[0], "rpc-get-output");
        },
    }

    rpc unary cancel {
        request: sessions::cancel::Request {
            session_id: String::from("rpc-cancel-input"),
        },
        respond: |request| sessions::cancel::Response {
            session: session(request.session_id, "rpc-cancel-output"),
        },
        convenience: cancel("rpc-cancel-input"),
        project: |response| response.session,
        check: |session| {
            assert_eq!(session.session_id, "rpc-cancel-input");
            assert_eq!(session.partition_ids[0], "rpc-cancel-output");
        },
    }

    rpc unary pause {
        request: sessions::pause::Request {
            session_id: String::from("rpc-pause-input"),
        },
        respond: |request| sessions::pause::Response {
            session: session(request.session_id, "rpc-pause-output"),
        },
        convenience: pause("rpc-pause-input"),
        project: |response| response.session,
        check: |session| {
            assert_eq!(session.session_id, "rpc-pause-input");
            assert_eq!(session.partition_ids[0], "rpc-pause-output");
        },
    }

    rpc unary resume {
        request: sessions::resume::Request {
            session_id: String::from("rpc-resume-input"),
        },
        respond: |request| sessions::resume::Response {
            session: session(request.session_id, "rpc-resume-output"),
        },
        convenience: resume("rpc-resume-input"),
        project: |response| response.session,
        check: |session| {
            assert_eq!(session.session_id, "rpc-resume-input");
            assert_eq!(session.partition_ids[0], "rpc-resume-output");
        },
    }

    rpc unary close {
        request: sessions::close::Request {
            session_id: String::from("rpc-close-input"),
        },
        respond: |request| sessions::close::Response {
            session: session(request.session_id, "rpc-close-output"),
        },
        convenience: close("rpc-close-input"),
        project: |response| response.session,
        check: |session| {
            assert_eq!(session.session_id, "rpc-close-input");
            assert_eq!(session.partition_ids[0], "rpc-close-output");
        },
    }

    rpc unary purge {
        request: sessions::purge::Request {
            session_id: String::from("rpc-purge-input"),
        },
        respond: |request| sessions::purge::Response {
            session: session(request.session_id, "rpc-purge-output"),
        },
        convenience: purge("rpc-purge-input"),
        project: |response| response.session,
        check: |session| {
            assert_eq!(session.session_id, "rpc-purge-input");
            assert_eq!(session.partition_ids[0], "rpc-purge-output");
        },
    }

    rpc unary delete {
        request: sessions::delete::Request {
            session_id: String::from("rpc-delete-input"),
        },
        respond: |request| sessions::delete::Response {
            session: session(request.session_id, "rpc-delete-output"),
        },
        convenience: delete("rpc-delete-input"),
        project: |response| response.session,
        check: |session| {
            assert_eq!(session.session_id, "rpc-delete-input");
            assert_eq!(session.partition_ids[0], "rpc-delete-output");
        },
    }

    rpc unary create {
        request: sessions::create::Request {
            partition_ids: vec![String::from("rpc-create-input")],
            default_task_options: Default::default(),
        },
        respond: |_request| sessions::create::Response {
            session_id: String::from("rpc-create-output"),
        },
        convenience: create([String::from("rpc-create-input")], Default::default()),
        project: |response| response.session_id,
        check: |session_id| {
            assert_eq!(session_id, "rpc-create-output");
        },
    }

    rpc unary stop_submission {
        request: sessions::stop_submission::Request {
            // Deliberately unequal: two `bool` parameters in a row are the one pair the
            // signature cannot keep apart, so the values have to.
            session_id: String::from("rpc-stop-input"),
            client: true,
            worker: false,
        },
        respond: |request| sessions::stop_submission::Response {
            session: session(request.session_id, "rpc-stop-output"),
        },
        convenience: stop_submission("rpc-stop-input", true, false),
        project: |response| response.session,
        check: |session| {
            assert_eq!(session.session_id, "rpc-stop-input");
            assert_eq!(session.partition_ids[0], "rpc-stop-output");
        },
    }
}
