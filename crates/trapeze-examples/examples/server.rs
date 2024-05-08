use futures::stream::StreamExt as _;
use tokio::fs::remove_file;
use tokio::pin;
use tokio::signal::ctrl_c;
use tokio::time::sleep;
use trapeze::prelude::Stream;
use trapeze::stream::try_stream;
use trapeze::{get_context, service, Code, Result, Server, Status};

mod common;

use common::{grpc, streaming, types, ADDRESS};
use grpc::*;
use streaming::*;
use types::*;

#[derive(Clone, Default)]
struct Services;

impl Health for Services {
    async fn check(&self, _req: CheckRequest) -> Result<HealthCheckResponse> {
        println!("> check() - {:?}", get_context());
        sleep(std::time::Duration::from_secs(10)).await;
        Err(Status::new(Code::NotFound, "Just for fun"))
    }

    async fn version(&self, _req: CheckRequest) -> Result<VersionCheckResponse> {
        println!("> version() - {:?}", get_context());
        Ok(VersionCheckResponse {
            agent_version: "mock.0.1".to_string(),
            grpc_version: "0.0.1".to_string(),
        })
    }
}

impl AgentService for Services {
    async fn list_interfaces(&self, _req: ListInterfacesRequest) -> Result<Interfaces> {
        println!("> list_interfaces() - {:?}", get_context());
        Ok(Interfaces {
            interfaces: vec![
                Interface {
                    name: "first".to_string(),
                    ..Default::default()
                },
                Interface {
                    name: "second".to_string(),
                    ..Default::default()
                },
            ],
        })
    }
}

impl Streaming for Services {
    async fn echo(&self, mut echo_payload: EchoPayload) -> trapeze::Result<EchoPayload> {
        println!("> echo() - {:?}", get_context());
        echo_payload.seq += 1;
        Ok(echo_payload)
    }

    fn echo_stream(
        &self,
        echo_payloads: impl Stream<Item = EchoPayload> + Send,
    ) -> impl Stream<Item = Result<EchoPayload>> + Send {
        println!("> echo_stream() - {:?}", get_context());
        try_stream! {
            for await mut echo_payload in echo_payloads {
                echo_payload.seq += 1;
                yield echo_payload;
            }
        }
    }

    async fn sum_stream(&self, parts: impl Stream<Item = Part> + Send) -> Result<Sum> {
        println!("> sum_stream() - {:?}", get_context());
        pin!(parts);
        let mut sum = Sum { num: 0, sum: 0 };
        while let Some(part) = parts.next().await {
            sum.num += 1;
            sum.sum += part.add;
        }
        Ok(sum)
    }

    fn divide_stream(&self, sum: Sum) -> impl Stream<Item = Result<Part>> + Send {
        println!("> divide_stream() - {:?}", get_context());
        try_stream! {
            let mut total = 0i32;
            let add = sum.sum / sum.num;

            for _ in 1..sum.num {
                total += add;
                yield Part { add };
            }

            let add = sum.sum - total;
            yield Part { add };
        }
    }

    async fn echo_null(&self, echo_payloads: impl Stream<Item = EchoPayload> + Send) -> Result<()> {
        println!("> echo_null() - {:?}", get_context());
        pin!(echo_payloads);
        let mut echo_payloads = echo_payloads.enumerate();
        while let Some((i, echo_payload)) = echo_payloads.next().await {
            let i = i as u32;
            if echo_payload.seq != i {
                Err(Status::new(Code::InvalidArgument, "Invalid sequence"))?;
            }
            if echo_payload.msg != "non-empty empty" {
                Err(Status::new(Code::InvalidArgument, "Invalid message"))?;
            }
        }
        Ok(())
    }

    fn echo_null_stream(
        &self,
        echo_payloads: impl Stream<Item = EchoPayload> + Send,
    ) -> impl Stream<Item = Result<()>> + Send {
        println!("> echo_null_stream() - {:?}", get_context());
        try_stream! {
            for await (i, echo_payload) in echo_payloads.enumerate() {
                let i = i as u32;
                if echo_payload.seq != i {
                    Err(Status::new(Code::InvalidArgument, "Invalid sequence"))?;
                }
                if echo_payload.msg != "non-empty empty" {
                    Err(Status::new(Code::InvalidArgument, "Invalid message"))?;
                }
                yield ();
            }
        }
    }

    fn echo_default_value(
        &self,
        echo_payload: EchoPayload,
    ) -> impl Stream<Item = Result<EchoPayload>> + Send {
        println!("> echo_default_value() - {:?}", get_context());
        try_stream! {
            if echo_payload.seq != 0 || !echo_payload.msg.is_empty() {
                return Err(Status::new(Code::Unknown, "Expect a request with empty payload to verify #208"))?;
            }

            yield echo_payload;
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let _ = remove_file(ADDRESS).await;

    let handle = Server::new()
        .register(service!(Services : Health + AgentService + Streaming))
        .bind(ADDRESS)
        .await
        .expect("Error binding listener");

    let ctrl_c = async move {
        ctrl_c().await.expect("Failed to wait for Ctrl+C.");
    };

    println!("Listening on {ADDRESS}");
    println!("Press Ctrl+C to exit.");

    ctrl_c.await;
    println!();
    println!("Shutting down server");

    handle.shutdown();
    handle.await.expect("Error shutting down server");
}
