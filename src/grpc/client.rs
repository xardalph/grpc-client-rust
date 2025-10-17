use grpc_ease::reflection::ReflectionClient;
mod lib;
use prost_reflect::DescriptorPool;
use prost_reflect::DynamicMessage;
use prost_types::FileDescriptorSet;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = "http://127.0.0.1:50051"; // one day let's take this as argument.
    let mut client = lib::ReflectionClient::new(url.to_string()).await?;
    let proto_files = client.get_proto_files().await.unwrap();
    //println!("{:?}", proto_files);
    let mut pool = DescriptorPool::new();
    pool.add_file_descriptor_protos(proto_files.into_iter())?;
    let service = pool
        .get_service_by_name("helloworld.Greeter")
        .expect("Service not found in proto pool");
    let method = service
        .methods()
        .find(|x| x.name() == "SayHello")
        .expect("no method found.");
    println!("method : {:#?}", method.input());
    let mut request_msg = DynamicMessage::new(method.input());
    request_msg.set_field_by_name("name", prost_reflect::Value::String("rust".to_string()));
    Ok(())
}
/*
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
 */
