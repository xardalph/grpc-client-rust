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
    println!("starting process on 50051");
    let conn = tonic::transport::Endpoint::new("http://127.0.0.1:50051")?
        .connect()
        .await?;

    let mut client = ServerReflectionClient::new(conn);
    println!("started process on 50051");
    let list_services_request = ServerReflectionRequest {
        host: "host".into(),
        //message_request: Some(MessageRequest::FileByFilename(
        //    "helloworld.Greeter".to_string(),
        //)),
        
        message_request: Some(MessageRequest::FileContainingSymbol(
            "helloworld.Greeter".into(),
        )),
    };
    println!("query : {:?}", list_services_request);
    //message_request: Some(MessageRequest::FileByFilename("helloworld.Greeter".to_string())),
    let request_stream = tokio_stream::once(list_services_request);
    let mut inbound = client
        .server_reflection_info(request_stream)
        .await?
        .into_inner();
    println!("response : {:?}", inbound.message());
    while let Some(recv) = inbound.next().await {
        match recv {
            Ok(resp) => parse_response(resp),
            Err(e) => println!("\tdid not receive response due to error: `{}`", e),
        }
    }

    Ok(())
}

