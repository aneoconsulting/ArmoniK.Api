//! RPC identity: service markers, call-kind markers, and the [`Rpc`] trait
//! tying each request type to the RPC it initiates.
//!
//! Spike scope: the `Results` service is hand-written below, standing in for
//! what `service!` will emit for every service (see `DESIGN-rpc.md` §3.3).

/// A proto service. One marker type per service.
pub trait Service {
    /// Fully-qualified proto service name.
    const NAME: &'static str;
}

/// Marker: unary RPC.
pub struct Unary;

/// Marker: server-streaming RPC.
pub struct ServerStream;

/// Marker: client-streaming RPC.
pub struct ClientStream;

/// A request type that identifies exactly one RPC.
///
/// Every RPC has a globally unique Rust request type (where the proto shares
/// one message across RPCs, the crate defines a distinct wire-compatible
/// struct per site), so the request type alone determines the service, the
/// method, the path and the response type.
pub trait Rpc: prost::Message + Default + std::fmt::Debug + 'static {
    /// The service this RPC belongs to.
    type Service: Service;
    /// [`Unary`], [`ServerStream`] or [`ClientStream`].
    type Kind;
    /// The response message, or the stream *item* for server-streaming RPCs.
    type Response: prost::Message + Default + std::fmt::Debug + 'static;

    /// Method name, as in the proto (`"ListResults"`).
    const METHOD: &'static str;
    /// Request path: `/package.Service/Method`.
    const PATH: &'static str;
    /// Telemetry label (`"Results::list"`).
    const LABEL: &'static str;
}

/// Service marker types.
pub mod services {
    /// The ResultsService provides methods for interacting with results.
    pub struct Results;

    impl super::Service for Results {
        const NAME: &'static str = "armonik.api.grpc.v1.results.Results";
    }
}

macro_rules! results_rpc {
    ($kind:ty, $module:ident, $method:literal) => {
        impl Rpc for crate::results::$module::Request {
            type Service = services::Results;
            type Kind = $kind;
            type Response = crate::results::$module::Response;

            const METHOD: &'static str = $method;
            const PATH: &'static str = concat!("/armonik.api.grpc.v1.results.Results/", $method);
            const LABEL: &'static str = concat!("Results::", stringify!($module));
        }
    };
}

results_rpc!(Unary, list, "ListResults");
results_rpc!(Unary, get, "GetResult");
results_rpc!(Unary, get_owner_task_id, "GetOwnerTaskId");
results_rpc!(Unary, create_metadata, "CreateResultsMetaData");
results_rpc!(Unary, create, "CreateResults");
results_rpc!(Unary, import, "ImportResultsData");
results_rpc!(Unary, delete_data, "DeleteResultsData");
results_rpc!(Unary, get_service_configuration, "GetServiceConfiguration");
results_rpc!(ServerStream, download, "DownloadResultData");
results_rpc!(ClientStream, upload, "UploadResultData");
