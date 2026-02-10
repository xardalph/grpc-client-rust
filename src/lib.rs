pub mod grpc_client;

#[derive(Debug, Clone)]
pub struct GrpcFilter {
    file: Option<String>,
    service: Option<String>,
    method: Option<String>,
}
