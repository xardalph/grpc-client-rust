use grpc_ease::reflection::ReflectionClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let endpoint = "http://127.0.0.1:5

    let mut reflection_client = ReflectionClient::new(endpoint.to_string()).await?;
    let services = reflection_client.list_services().await?;

    for service in services {
        println!("Service: {}", service.service);
        println!("Package: {}", service.package);
        for method in service.methods {
            println!("  RPC Method: {}", method.name);
        }
    }

    Ok(())
}
