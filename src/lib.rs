//! A Grpc client that integrate both the reflection and the standard tonic grpc client on the same channel
//!
//! # Example
//!
//! ```no_run
//! # tokio_test::block_on(async {
//! use grpc_client::{Client,GrpcFilters};
//! let mut client = Client::new("https://localhost:8080".to_string()).await.unwrap();
//! let proto_files = client.get_proto_files().await.unwrap();
//! // this empty filter will not filter anything, see GrpcFilters::new for more details
//! let filters = GrpcFilters::new_empty();
//! // show all services details to stdout
//! client.list_services_to_stdout(filters).await.unwrap();
//! // once you know the details of the call you want, you can call it using the same client with strings parameters
//! let response = client.request(&"filename.service", &"method", vec![("argumentname1".to_string(), "value".to_string())]);
//! # })
//! ```
pub mod client;
pub mod dynamic_codec;
pub use client::Client;
pub use client::GrpcFilters;
