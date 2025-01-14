//!
//! IPP print protocol implementation for Rust
//!
//! Usage examples:
//!
//!```rust,no_run
//! // using raw API
//! use ipp::client::IppClientBuilder;
//! use ipp::proto::{ipp::Operation, request::IppRequestResponse, IppVersion};
//! use tokio::runtime::Runtime;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut runtime = Runtime::new()?;
//!     let uri = "http://localhost:631/printers/test-printer";
//!     let req = IppRequestResponse::new(
//!         IppVersion::Ipp11,
//!         Operation::GetPrinterAttributes,
//!         Some(uri)
//!     );
//!     let client = IppClientBuilder::new(&uri).build();
//!     let resp = runtime.block_on(client.send_request(req))?;
//!     if resp.header().operation_status <= 2 {
//!         println!("result: {:?}", resp.attributes());
//!     }
//!     Ok(())
//! }
//!```
//!```rust,no_run
//! // using operations API
//! use ipp::proto::{IppOperationBuilder, ipp::DelimiterTag};
//! use ipp::client::IppClientBuilder;
//! use tokio::runtime::Runtime;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut runtime = Runtime::new()?;
//!     let operation = IppOperationBuilder::get_printer_attributes().build();
//!     let client = IppClientBuilder::new("http://localhost:631/printers/test-printer").build();
//!     let attrs = runtime.block_on(client.send(operation))?;
//!     for (_, v) in attrs.groups_of(DelimiterTag::PrinterAttributes)[0].attributes() {
//!         println!("{}: {}", v.name(), v.value());
//!     }
//!     Ok(())
//! }
//!```

pub use ipp_proto as proto;

#[cfg(feature = "client")]
pub use ipp_client as client;

#[cfg(feature = "server")]
pub use ipp_server as server;

#[cfg(feature = "util")]
pub use ipp_util as util;
