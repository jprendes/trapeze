use std::time::Duration;

use tokio::net::UnixStream;
use trapeze::Client;

use grpc::{AgentService as _, Health as _};

mod common;
use common::*;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let conn = UnixStream::connect(ADDRESS).await?;

    let client = Client::new(conn).with_metadata([
        ("key-1", "value-1-1"),
        ("key-1", "value-1-2"),
        ("key-2", "value-2"),
    ]);
    client.start();

    let now = std::time::Instant::now();

    let t1 = tokio::spawn({
        let client = client.clone();
        async move {
            let client = client
                .with_metadata(None)
                .with_timeout(Duration::from_millis(20));
            let req = grpc::CheckRequest::default();
            let res = client.check(req).await;

            println!(
                "> health.check() -> {:#?} ended: ({:?})",
                res,
                now.elapsed(),
            );
        }
    });

    let t2 = tokio::spawn({
        let client = client.clone();
        async move {
            let req = grpc::ListInterfacesRequest::default();
            let res = client.list_interfaces(req).await;

            println!(
                "> agent.list_interfaces() -> {:#?} ended: ({:?})",
                res,
                now.elapsed(),
            );
        }
    });

    let t3 = tokio::spawn({
        let client = client.clone();
        async move {
            tokio::time::sleep(Duration::from_millis(22)).await;

            let req = grpc::OnlineCpuMemRequest::default();
            let res = client.online_cpu_mem(req).await;

            println!(
                "> agent.online_cpu_mem() -> {:#?} ended: ({:?})",
                res,
                now.elapsed()
            );

            let req = grpc::CheckRequest::default();
            let res = client.version(req).await;
            println!("> health.version() -> {:#?} ({:?})", res, now.elapsed());
        }
    });

    let _ = tokio::join!(t1, t2, t3);

    Ok(())
}
