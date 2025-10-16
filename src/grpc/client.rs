use grpc_ease::reflection::ReflectionClient;
//mod lib;
use prost::Message;
use tokio_stream::StreamExt;
use tonic_reflection::pb::v1::{
    ServerReflectionRequest, ServerReflectionResponse,
    server_reflection_client::ServerReflectionClient, server_reflection_request::MessageRequest,
    server_reflection_response::MessageResponse,
};

fn parse_response(resp: ServerReflectionResponse) {
    let message_response = resp.message_response.expect("message response");

    if let MessageResponse::ListServicesResponse(list_response) = message_response {
        for svc in list_response.service {
            println!("\tfound service: `{:?}`", svc);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = "http://127.0.0.1:50051";

    // let's try with reflection lib.
    //let client = persoreflection::ReflectionClient::new(url.to_string());

    let conn = tonic::transport::Endpoint::new(url)?.connect().await?;

    let mut client = ServerReflectionClient::new(conn);
    println!("started client.");

    let list_services_request = ServerReflectionRequest {
        host: "host".into(),
        //message_request: Some(MessageRequest::FileByFilename(
        //    "helloworld.Greeter".to_string(),
        //)),
        message_request: Some(MessageRequest::ListServices("".into())),
    };
    println!("query list service : {:?}", list_services_request);
    let request_stream = tokio_stream::once(list_services_request);
    let mut inbound = client
        .server_reflection_info(request_stream)
        .await?
        .into_inner();
    while let Some(recv) = inbound.next().await {
        match recv {
            Ok(resp) => parse_response(resp),
            Err(e) => println!("\tdid not receive response due to error: `{}`", e),
        }
    }
    let get_file_request = ServerReflectionRequest {
        host: "host".into(),
        //message_request: Some(MessageRequest::FileByFilename(
        //    "helloworld.Greeter".to_string(),
        //)),
        message_request: Some(MessageRequest::FileContainingSymbol(
            "helloworld.Greeter".into(),
        )),
    };
    println!("query : {:?}", get_file_request);
    let request_stream_file = tokio_stream::once(get_file_request);
    let mut inbound = client
        .server_reflection_info(request_stream_file)
        .await?
        .into_inner();

    while let Some(recv) = inbound.next().await {
        match recv {
            Ok(resp) => {
                let message_response = resp.message_response.expect("message response");
                if let MessageResponse::FileDescriptorResponse(descriptor_response) =
                    message_response
                {
                    let mut descriptors = Vec::new();
                    for file_descriptor_proto in descriptor_response.file_descriptor_proto {
                        let file_descriptor =
                            prost_types::FileDescriptorProto::decode(&file_descriptor_proto[..])?;
                        descriptors.push(file_descriptor);
                        println!("got a first file descriptor!");
                    }
                    println!("list of descroptor : {:#?}", descriptors);
                }
            }
            Err(e) => println!("\tdid not receive response due to error: `{}`", e),
        }
    }

    Ok(())
}
