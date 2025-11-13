use prost::Message;
use prost_types::FileDescriptorProto;
use std::error::Error;
use tokio_stream::StreamExt;
use tonic::{Request, client::Grpc, transport::Channel};
use tonic_reflection::pb::v1::{
    ServerReflectionRequest, server_reflection_client::ServerReflectionClient,
    server_reflection_request::MessageRequest, server_reflection_response::MessageResponse,
};

use crate::Cli;
/// A Grpc client that integrate both the reflection and the standard grpc client on the same channel
/// ServerReflectionClient does not seem to allow to retrieve the inner client so we need to duplicate it
/// the channel support the only tcp connection of this, so this should not be too costly or seen in the server log.
pub struct GrpcClient {
    reflection_client: ServerReflectionClient<Channel>,
    pub client: Grpc<Channel>,
}

impl GrpcClient {
    /// Create a [`GrpcClient`] from a string and await for connection to be established,
    ///
    /// ```
    /// # use lib::grpc_client;
    /// grpc_client::new("https://example.com");
    /// ```
    pub async fn new(endpoint: String) -> Result<Self, Box<dyn Error>> {
        let channel = Channel::from_shared(endpoint)?.connect().await?;
        let mut s = Self {
            reflection_client: ServerReflectionClient::new(channel.clone()),
            client: tonic::client::Grpc::new(channel),
        };
        s.client.ready().await?;
        return Ok(s);
    }
    /// send a reflection request and wait for the response
    /// Should probably be private
    pub async fn make_reflection_request(
        &mut self,
        request: ServerReflectionRequest,
    ) -> Result<MessageResponse, Box<dyn Error>> {
        let request = Request::new(tokio_stream::once(request));
        let mut inbound = self
            .reflection_client
            .server_reflection_info(request)
            .await?
            .into_inner();

        if let Some(response) = inbound.next().await {
            return Ok(response?.message_response.expect("some MessageResponse"));
        }

        return Err("No response received".into());
    }
    /// show services exposed by a grpc server on stdout.
    /// Filter which service to show by an include filter vector
    pub async fn list_services_to_stdout(
        &mut self,
        filter: &Vec<String>,
    ) -> Result<(), Box<dyn Error>> {
        println!("ok on peux lister les services ici, @TODO");
        let files = self.get_proto_files().await?;
        for f in files {
            if filter.is_empty() && !filter.contains(&f.name.clone().unwrap_or_default()) {
                continue;
            }
            for message in &f.message_type {
                println!(
                    "{} : {:#?}",
                    message.name.clone().unwrap_or_default(),
                    message.field
                )
            }
            println!("file {} :", &f.name());
            for s in &f.service {
                println!("  service {:?}", &s.name());
                for m in &s.method {
                    println!(
                        "    method : {}, input: {}, output : {}",
                        &m.name(),
                        &m.input_type(),
                        &m.output_type()
                    );
                    // let's show the input_type message definition and the output_type message here
                    self.print_grpc_message(&f, &m.input_type());

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
            if format!("{}.{}", file.package(), msg.name()) != msg_name {
                println!("{} {} != {}", file.package(), &msg.name(), &msg_name);
                continue;
            }
            println!(" msg {}", msg.name());
            for m in &msg.field {
                println!(
                    "        Field {}: is of type {}",
                    m.json_name(),
                    &m.r#type().as_str_name()
                )
            }
            // println!("\n{:#?}\n\n", &msg);
        }
        Ok(())
    }
    /// get protobuf file from a remote server with reflection v1 api.
    /// For now v1_alpha is not accepted, it would be nice to to the work to make it parametrable on runtime
    ///
    ///```
    /// let mut client = lib::grpc_client::GrpcClient::new(url).await?;
    /// let proto_files = client.get_proto_files().await?;
    ///```
    pub async fn get_proto_files(
        &mut self,
    ) -> Result<Vec<prost_types::FileDescriptorProto>, Box<dyn Error>> {
        let response = self
            .make_reflection_request(ServerReflectionRequest {
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
            return Ok(proto_files);
        } else {
            return Err("Expected a ListServicesResponse variant".into());
        }
    }
    async fn get_file_descriptor_from_symbol(
        &mut self,
        symbol: String,
    ) -> Result<Vec<prost_types::FileDescriptorProto>, Box<dyn Error>> {
        let response = self
            .make_reflection_request(ServerReflectionRequest {
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
            return Ok(descriptors);
        } else {
            return Err("Expected a FileDescriptorResponse variant".into());
        }
    }
}
