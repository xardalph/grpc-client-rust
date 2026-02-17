//use crate::lib::dynamic_codec::DynamicCodec;
mod grpc_client;
use prost_reflect::{DescriptorPool, DynamicMessage};

use clap::{Parser, Subcommand};
use grpc_client::Client;
use grpc_client::client::GrpcFilters;

use std::error::Error;
use tonic::Request;

use tracing::log::{Level, debug, error, info, log_enabled};
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
    command: Option<Commands>,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    /// list grpc services
    List {
        /// detail grpc services asked, by default all of them
        /// Filter is inclusive and non regexp
        /// ex: filename.servicename.methodname
        /// each part is optionnal, so ".servicename." work to filter only on the servicename, but not "servicename"
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
        self.command
            .clone()
            .unwrap_or(Commands::List { list: vec![] })
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
    env_logger::init();
    info!("Starting the program");

    let mut client = Client::new(cli.url.clone()).await?;

    //println!("{:?}", proto_files);
    match cli.command() {
        Commands::List { list } => {
            println!("List {:?}", list);
            let filters = GrpcFilters::new(list);

            client.list_services_to_stdout(filters).await?;
            Ok(())
        }
        Commands::Get {
            service,
            method,
            arguments,
        } => {
            let response = client.request(service, method, arguments);
            // Convert DynamicMessage → JSON string
            let json = serde_json::to_string_pretty(&response.await?)?;
            println!("Response as JSON:\n{}", json);
            //println!("{:?}", response); */
            Ok(())
        }
    }
}
