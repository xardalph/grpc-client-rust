mod lib;
use crate::lib::dynamic_codec::DynamicCodec;
use clap::{Parser, Subcommand};
use prost_reflect::DescriptorPool;
use prost_reflect::DynamicMessage;
use std::error::Error;
use tonic::Request;

use crate::lib::dynamic_codec::DynamicCodec;
use std::path::PathBuf;

use crate::dynamic_codec::DynamicCodec;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// required url to connect to (https://localhost:50051)
    #[arg(short, long, value_parser = parse_url)]
    url: String,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,

    /// either list all service, list one service, or make a request
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// list grpc services
    List {
        /// detail grpc services asked, by default all of them
        #[clap(value_parser, num_args = 1.., value_delimiter = ' ')]
        list: Vec<String>,
    },
    /// send a grpc request
    Get {
        /// Grpc service to use
        service: String,
        /// Grpc method to search and execute
        method: String,
        /// Tuples of method arguments, ex : -a argName=value -a argName2=val2
        #[arg(short = 'a', value_parser = parse_key_val::<String, String>)]
        arguments: Vec<(String, String)>,
    },
}
impl Cli {
    pub fn command(&self) -> Commands {
        self.command.clone().unwrap_or(Commands::List)
    }
}
fn parse_url(s: &str) -> Result<String, String> {
    if s.starts_with("http://") || s.starts_with("https://") {
        Ok(s.to_string())
    } else {
        // Prepend https:// if missing
        Ok(format!("https://{}", s))
    }
}
// Parse a single key-value pair to be used by clap.
// taken from https://github.com/clap-rs/clap/blob/f45a32ec2c1506faf319d914d985927ed47b0b5e/examples/typed-derive.rs
fn parse_key_val<T, U>(s: &str) -> Result<(T, U), Box<dyn Error + Send + Sync + 'static>>
where
    T: std::str::FromStr,
    T::Err: Error + Send + Sync + 'static,
    U: std::str::FromStr,
    U::Err: Error + Send + Sync + 'static,
{
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=value: no `=` found in `{s}`"))?;
    Ok((s[..pos].parse()?, s[pos + 1..].parse()?))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let mut client = lib::grpc_client::GrpcClient::new(cli.url.clone())
        .await
        .unwrap();
    client.client.ready().await?;
    let proto_files = client.get_proto_files().await.unwrap();
    //println!("{:?}", proto_files);
    match cli.command() {
            Commands::List { list } => {
                println!("List {:?}", list);
                client.list_services_to_stdout(list).await?;
                Ok(())
            }
Commands::Get {
            service,
            method,
            arguments,
        } => {
            let mut pool = DescriptorPool::new();
            pool.add_file_descriptor_protos(proto_files.into_iter())?;
            let service_pool = pool
                .get_service_by_name(service.as_str())
                .ok_or(format!("no service {} found.", service.as_str()))?;
            let method = service_pool
                .methods()
                .find(|x| x.name() == method.as_str())
                .ok_or(format!("no grpc method {} found.", method.as_str()))?;

            let mut request_msg = DynamicMessage::new(method.input());
            for arg in arguments {
                if request_msg.get_field_by_name(&arg.0) == None {
                    return Err(
                        format!("field '{}' don't exist for message {}", arg.0, service,).into(),
                    );
                }
                request_msg.set_field_by_name(&arg.0, prost_reflect::Value::String(arg.1));
                println!("fields : {:#?}", request_msg.get_field_by_name("name"));
            }

            let path = format!("/{}/{}", method.parent_service().full_name(), method.name());
            // Create our DynamicCodec for the output type
            let codec = DynamicCodec {
                pool: pool.clone(),
                message_name: method.output().full_name().to_string(),
            };
            let req = Request::new(request_msg);
            println!("sending unary request.");
            let response = client.client.unary(req, path.parse()?, codec).await?;
            let dyn_msg = response.into_inner();

            // Convert DynamicMessage → JSON string
            let json = serde_json::to_string_pretty(&dyn_msg)?;
            println!("Response as JSON:\n{}", json);
            //println!("{:?}", response);
            Ok(())
        }
}
