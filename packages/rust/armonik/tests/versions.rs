use armonik::server::VersionsServiceExt;
use armonik::versions;

#[macro_use]
mod common;

rpc_tests! {
    client: into_versions;
    server: VersionsService, versions_server;
    mock: "Versions";

    rpc unary list {
        request: versions::list::Request {},
        respond: |_request| versions::list::Response {
            core: String::from("rpc-list-output"),
            api: String::from("rpc-list-api-output"),
        },
        convenience: list(),
        check: |response| {
            assert_eq!(response.core, "rpc-list-output");
            assert_eq!(response.api, "rpc-list-api-output");
        },
    }
}
