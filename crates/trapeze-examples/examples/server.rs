use tokio::fs::remove_file;
use tokio::net::UnixListener;
use tokio::signal::ctrl_c;
use tokio::time::sleep;
use trapeze::{context::get_context, Code, Result, ServerBuilder, Status};
use types::Interface;

use grpc::{AgentService as _, Health as _};

mod common;
use common::*;

#[derive(Clone, Default)]
struct HealthService;
impl grpc::Health for HealthService {
    async fn check(&self, _req: grpc::CheckRequest) -> Result<grpc::HealthCheckResponse> {
        println!("> check() - {:?}", get_context());
        sleep(std::time::Duration::from_secs(10)).await;
        Err(Status::new(Code::NotFound, "Just for fun"))
    }

    async fn version(&self, _req: grpc::CheckRequest) -> Result<grpc::VersionCheckResponse> {
        println!("> version() - {:?}", get_context());
        Ok(grpc::VersionCheckResponse {
            agent_version: "mock.0.1".to_string(),
            grpc_version: "0.0.1".to_string(),
        })
    }
}

#[derive(Clone, Default)]
struct AgentService;
impl grpc::AgentService for AgentService {
    async fn list_interfaces(&self, _req: grpc::ListInterfacesRequest) -> Result<grpc::Interfaces> {
        println!("> list_interfaces() - {:?}", get_context());
        Ok(grpc::Interfaces {
            interfaces: vec![
                Interface {
                    name: "fist".to_string(),
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

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let _ = remove_file(ADDRESS).await;
    let listener = UnixListener::bind(ADDRESS)?;

    tokio::spawn(async move {
        loop {
            if let Ok((conn, _)) = listener.accept().await {
                ServerBuilder::new()
                    .add_service(AgentService::service())
                    .add_service(HealthService::service())
                    .build(conn)
                    .start();
            }
        }
    });

    println!("Listening on {ADDRESS}");
    println!("Press Ctrl+C to exit.");

    ctrl_c().await.expect("Failed to wait for Ctrl+C.");

    let _ = remove_file(ADDRESS).await;

    Ok(())
}
