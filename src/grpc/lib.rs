use prost::Message;
use std::error::Error;
use tokio_stream::StreamExt;
use tonic::{
    Request,
    transport::{Channel, Endpoint},
};
use tonic_reflection::pb::v1::{
    ServerReflectionRequest, server_reflection_client::ServerReflectionClient,
    server_reflection_request::MessageRequest, server_reflection_response::MessageResponse,
};
/// Represents information about an RPC method
#[derive(Debug)]
pub struct MethodInfo {
    /// The name of the RPC method
    pub name: String,
    /// The name of the request message
    pub request: String,
    /// The name of the response message
    pub response: String,
}

/// Represents information about a gRPC service, including its package name,
/// service name, and a list of RPC methods
#[derive(Debug)]
pub struct ServiceInfo {
    /// The package name of the gRPC service
    pub package: String,
    /// The name of the gRPC service
    pub service: String,
    /// A list of RPC methods available in the service
    pub methods: Vec<MethodInfo>,
}

pub struct ReflectionClient {
    client: ServerReflectionClient<Channel>,
}

impl ReflectionClient {
    pub async fn new(endpoint: String) -> Result<Self, Box<dyn Error>> {
        let channel = Channel::from_shared(endpoint)?.connect().await?;
        Ok(Self {
            client: ServerReflectionClient::new(channel),
        })
    }

    pub async fn connect(addr: &str) -> Result<Self, Box<dyn Error>> {
        let endpoint = Endpoint::new(addr.to_string())?.connect().await?;
        Ok(Self {
            client: ServerReflectionClient::new(endpoint),
        })
    }

    async fn make_request(
        &mut self,
        request: ServerReflectionRequest,
    ) -> Result<MessageResponse, Box<dyn Error>> {
        let request = Request::new(tokio_stream::once(request));
        let mut inbound = self
            .client
            .server_reflection_info(request)
            .await?
            .into_inner();

        if let Some(response) = inbound.next().await {
            return Ok(response?.message_response.expect("some MessageResponse"));
        }

        Err("No response received".into())
    }
    pub async fn get_proto_files(
        &mut self,
    ) -> Result<Vec<prost_types::FileDescriptorProto>, Box<dyn Error>> {
        let response = self
            .make_request(ServerReflectionRequest {
                host: "".to_string(),
                message_request: Some(MessageRequest::ListServices(String::new())),
            })
            .await?;

        if let MessageResponse::ListServicesResponse(services_response) = response {
            let mut proto_files = Vec::new();

            for service in services_response.service {
                let mut descriptors = self
                    .get_file_descriptor_from_symbol(service.name.clone())
                    .await?;
                proto_files.append(&mut descriptors);
            }
            Ok(proto_files)
        } else {
            Err("Expected a ListServicesResponse variant".into())
        }
    }
    async fn get_file_descriptor_from_symbol(
        &mut self,
        symbol: String,
    ) -> Result<Vec<prost_types::FileDescriptorProto>, Box<dyn Error>> {
        let response = self
            .make_request(ServerReflectionRequest {
                host: "".to_string(),
                message_request: Some(MessageRequest::FileContainingSymbol(symbol)),
            })
            .await?;

        if let MessageResponse::FileDescriptorResponse(descriptor_response) = response {
            let mut descriptors = Vec::new();
            for file_descriptor_proto in descriptor_response.file_descriptor_proto {
                let file_descriptor =
                    prost_types::FileDescriptorProto::decode(&file_descriptor_proto[..])?;
                descriptors.push(file_descriptor);
            }
            Ok(descriptors)
        } else {
            Err("Expected a FileDescriptorResponse variant".into())
        }
    }
}
