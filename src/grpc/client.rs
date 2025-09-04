use hello_world::HelloRequest;
use hello_world::greeter_client::GreeterClient;

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = GreeterClient::connect("http://0.0.0.0:50051").await?;

    let request = tonic::Request::new(HelloRequest {
        name: "Tonic".into(),
    });

    let response = client.say_hello(request).await?;

    println!("RESPONSE={response:?}");
    let request2 = tonic::Request::new(GoodbyRequest {
        name: "Tonic".into(),
    });
    let response2 = client.say_goodbye(request2).await?.into_inner();
    println!(
        "GoodBye : {}, detail : {}",
        response2.message, response2.detail
    );
    Ok(())
}
