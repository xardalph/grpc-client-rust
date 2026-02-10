use prost::Message;
use prost_types::FileDescriptorProto;
use rand::distr::uniform::UniformFloat;
use std::error::Error;
use thiserror::Error;
use tokio_stream::StreamExt;
use tonic::{IntoRequest, Request, client::Grpc, transport::Channel};
use tonic_reflection::pb::v1::{
    ServerReflectionRequest, server_reflection_client::ServerReflectionClient,
    server_reflection_request::MessageRequest, server_reflection_response::MessageResponse,
};
use tower::ready_cache::cache::Equivalent;
use tracing::log::{Level, debug, error, info, log_enabled};

/// A Grpc client that integrate both the reflection and the standard grpc client on the same channel
/// ServerReflectionClient does not seem to allow to retrieve the inner client so we need to duplicate it
/// the channel support the only tcp connection of this, so this should not be too costly or seen in the server log.
/// support only v1 reflection api for now.
pub struct Client {
    reflection_client: ServerReflectionClient<Channel>,
    cache: (), // todo : implement a cache system with file/in memory storage. (at least file for the client)
    pub client: Grpc<Channel>,
}
#[derive(Debug, Clone)]
pub struct GrpcFilter {
    file: Option<String>,
    service: Option<String>,
    method: Option<String>,
}
#[derive(Error, Debug)]
pub enum GrpcClientError {
    #[error("Failed to create grpc client")]
    GrpcClientCreationError(#[from] tonic::transport::Error),
    #[error("tonic error {0}")]
    ReflectionRequestError(#[from] tonic::Status),
    #[error("Empty response")]
    EmptyResponse(String),
    #[error("Decode error : {0}")]
    DecodeError(#[from] prost::DecodeError),
    #[error("bad grpc message received : {0}")]
    BadMessageType(String),
    #[error("Connection failed {0}")]
    ConnectionFailed(String),
}
impl Client {
    /// Create a new GrpcClient, given a channel (which will be cloned)
    pub async fn new(url: String) -> Result<Self, GrpcClientError> {
        let channel = Channel::from_shared(url).unwrap().connect().await?;

        let mut client = Self {
            reflection_client: ServerReflectionClient::new(channel.clone()),
            cache: (),
            client: Grpc::new(channel),
        };
        client.client.ready().await?;
        Ok(client)
    }

    /// send a reflection request and wait for the response
    /// Should probably be private
    pub async fn make_reflection_request(
        &mut self,
        message: MessageRequest,
    ) -> Result<MessageResponse, GrpcClientError> {
        let request = ServerReflectionRequest {
            //@TODO : find why we can configure that, maybe it should be exposed as an option
            host: "".to_string(),
            message_request: Some(message),
        };
        let request = Request::new(tokio_stream::once(request));
        let mut inbound = self
            .reflection_client
            .server_reflection_info(request)
            .await?
            .into_inner();

        if let Some(response) = inbound.next().await {
            return Ok(response?.message_response.expect("some MessageResponse"));
        }

        return Err(GrpcClientError::EmptyResponse(
            "No response received".to_string(),
        ));
    }

    /// show services exposed by a grpc server on stdout.
    /// Filter which service to show by an include filter vector
    pub async fn list_services_to_stdout(
        &mut self,
        filters_string: &Vec<String>,
    ) -> Result<(), GrpcClientError> {
        // let's first transform the vector of string to something more usable with Option to manage the empty value
        let mut filter = vec![];
        for f in filters_string {
            let owned = f.to_owned();
            let mut v = owned.split(".");

            filter.push(GrpcFilter {
                // TODO : empty string should be changed to None instead of Some("")
                file: v.next().map(|s| s.to_string()).take_if(
                    |v| {
                        if *v == "" { false } else { true }
                    },
                ),
                service: v
                    .next()
                    .map(|s| s.to_string())
                    .take_if(|v| if *v == "" { false } else { true }),
                method: v
                    .next()
                    .map(|s| s.to_string())
                    .take_if(|v| if *v == "" { false } else { true }),
            });
        }

        let files = self.get_proto_files().await?;
        debug!("filter : {:?}", filter);
        for f in files {
            debug!("checking file '{}'", &f.package());
            if filter_file(&filter, &f.package.clone().unwrap_or_default()) {
                continue;
            }
            println!("file {} :", &f.package());
            for s in &f.service {
                debug!("checking service '{}'", s.name());
                if filter_service(&filter, s.name()) {
                    continue;
                }
                println!("  service {:?}", &s.name());
                for m in &s.method {
                    debug!("checking method '{}'", m.name());

                    if filter_method(&filter, m.name()) {
                        continue;
                    }
                    println!(
                        "    method : {}, input: {}, output : {}",
                        &m.name(),
                        &m.input_type(),
                        &m.output_type()
                    );
                    // let's show the input_type message definition and the output_type message here
                    let _ = self.print_grpc_message(&f, &m.input_type());

                    while let Some(o) = &m.options {
                        println!("   option : {:#?}", &o);
                    }
                }
            }
        }
        Ok(())
    }
    fn print_grpc_message(
        self: &Self,
        file: &FileDescriptorProto,
        msg_name: &str,
    ) -> Result<(), Box<dyn Error>> {
        println!("     searching message {}", msg_name);
        for msg in &file.message_type {
            if format!(".{}.{}", file.package(), msg.name()) != msg_name {
                //println!(".{}.{} != {}", file.package(), &msg.name(), &msg_name);
                continue;
            }
            println!(" msg {}", msg.name());
            for m in &msg.field {
                println!(
                    "        Field '{}' is of type {}",
                    m.json_name(),
                    &m.r#type().as_str_name()
                )
            }
            // println!("\n{:#?}\n\n", &msg);
        }
        Ok(())
    }
    /// get protobuf file from a remote server with reflection v1 api.
    /// For now v1_alpha is not accepted, it would be nice to do the work to make it parametrable on runtime
    ///
    ///```
    /// let mut client = lib::grpc_client::GrpcClient::new(url).await?;
    /// let proto_files = client.get_proto_files().await?;
    ///```
    pub async fn get_proto_files(
        &mut self,
    ) -> Result<Vec<prost_types::FileDescriptorProto>, GrpcClientError> {
        let response = self
            .make_reflection_request(MessageRequest::ListServices(String::new()))
            .await?;

        if let MessageResponse::ListServicesResponse(services_response) = response {
            let mut proto_files = Vec::new();

            for service in services_response.service {
                let mut descriptors = self
                    .get_file_descriptor_from_symbol(service.name.clone())
                    .await?;
                proto_files.append(&mut descriptors);
            }
            return Ok(proto_files);
        } else {
            return Err(GrpcClientError::BadMessageType(
                "Expected a ListServicesResponse variant".to_string(),
            ));
        }
    }
    /// internal, used to get a single file descriptor proto from a symbol
    async fn get_file_descriptor_from_symbol(
        &mut self,
        symbol: String,
    ) -> Result<Vec<prost_types::FileDescriptorProto>, GrpcClientError> {
        let response = self
            .make_reflection_request(MessageRequest::FileContainingSymbol(symbol))
            .await?;

        if let MessageResponse::FileDescriptorResponse(descriptor_response) = response {
            let mut descriptors = Vec::new();
            for file_descriptor_proto in descriptor_response.file_descriptor_proto {
                let file_descriptor =
                    prost_types::FileDescriptorProto::decode(&file_descriptor_proto[..])?;
                descriptors.push(file_descriptor);
            }
            return Ok(descriptors);
        } else {
            return Err(GrpcClientError::BadMessageType(
                "Expected a FileDescriptorResponse variant".to_string(),
            ));
        }
    }
}

// todo : put this as functionnal design in an impl grpcfilter.
/// return false if filter is empty or match the value
pub fn filter_file(filter: &Vec<GrpcFilter>, val: &String) -> bool {
    if filter.is_empty()
        || filter
            .iter()
            .any(|f| f.file.as_ref().map_or(true, |f| f == val))
    {
        return false;
    }
    return true;
}
/// return false if filter is empty or match the value
pub fn filter_method(filter: &Vec<GrpcFilter>, val: &str) -> bool {
    if filter.is_empty()
        || filter
            .iter()
            .any(|f| f.method.as_ref().map_or(true, |f| f == val))
    {
        return false;
    }
    return true;
}
/// return false if filter is empty or match the value
pub fn filter_service(filter: &Vec<GrpcFilter>, val: &str) -> bool {
    if filter.is_empty()
        || filter
            .iter()
            .any(|f| f.service.as_ref().map_or(true, |f| f == val))
    {
        return false;
    }
    return true;
}
