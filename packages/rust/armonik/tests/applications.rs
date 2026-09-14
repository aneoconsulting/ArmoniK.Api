use armonik::applications;
use armonik::server::ApplicationsServiceExt;

#[macro_use]
mod common;

rpc_tests! {
    client: into_applications;
    server: ApplicationsService, applications_server;
    mock: "Applications";

    rpc unary list {
        request: applications::list::Request {
            filters: applications::filter::Or::default(),
            sort: applications::Sort::default(),
            page: 3,
            page_size: 12,
        },
        respond: |request| applications::list::Response {
            applications: vec![applications::Raw {
                name: String::from("rpc-list-output"),
                ..Default::default()
            }],
            page: request.page,
            page_size: request.page_size,
            total: 1337,
        },
        convenience: list(
            applications::filter::Or::default(),
            applications::Sort::default(),
            3,
            12,
        ),
        check: |response| {
            assert_eq!(response.page, 3);
            assert_eq!(response.page_size, 12);
            assert_eq!(response.total, 1337);
            assert_eq!(response.applications[0].name, "rpc-list-output");
        },
    }
}
