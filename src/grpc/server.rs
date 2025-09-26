use hello_world::greeter_server::{Greeter, GreeterServer};
use hello_world::{GoodbyReply, HelloReply, HelloRequest};
use tonic::{Request, Response, Status, transport::Server};

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[derive(Default)]
pub struct TheGreeter {}
pub mod proto {
    tonic::include_proto!("helloworld");

    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("helloworld_descriptor");
}
#[tonic::async_trait]
impl Greeter for TheGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        match request.remote_addr() {
            Some(val) => println!("Got a request from {:?}", val),
            None => println!("Got a request from a connexion that don't have any IP address."),
        };

        let reply = hello_world::HelloReply {
            message: format!("Hello {}!", request.into_inner().name),
        };
        Ok(Response::new(reply))
    }
    async fn say_goodbye(
        &self,
        request: tonic::Request<hello_world::GoodbyRequest>,
    ) -> Result<Response<GoodbyReply>, Status> {
        println!("Got a request from {:?}", request.remote_addr());

        let reply = hello_world::GoodbyReply {
            message: format!("Goodbye {}!", request.into_inner().name),
            detail: "text".to_string(),
        };
        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "0.0.0.0:50051".parse().unwrap();
    let service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
        .build_v1()
        .unwrap();

    let greeter = TheGreeter::default();

    println!("GreeterServer listening on {addr}");

    Server::builder()
        .add_service(GreeterServer::new(greeter))
        .add_service(service)
        .serve(addr)
        .await?;

    Ok(())
}
