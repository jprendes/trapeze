use std::time::{Duration, Instant};

use futures::TryStreamExt;
use tokio::time::sleep;
use trapeze::stream::stream;
use trapeze::{Client, ClientExt as _};

mod common;

use common::{grpc, streaming, ADDRESS};
use grpc::*;
use streaming::*;

async fn grpc_check(client: Client, start: Instant) {
    let client = client
        .with_metadata(None)
        .with_timeout(Duration::from_millis(20));
    let req = CheckRequest::default();
    let res = client.check(req).await;

    println!(
        "> health.check() -> {:?} ended: ({:?})",
        res,
        start.elapsed(),
    );
}

async fn grpc_list_interfaces(client: Client, start: Instant) {
    let req = ListInterfacesRequest::default();
    let res = client.list_interfaces(req).await;

    let res = match res {
        Ok(val) if val.interfaces.len() <= 3 => {
            let ifaces = val
                .interfaces
                .iter()
                .map(|iface| format!("Interface {{ name: {:?}, .. }}", iface.name))
                .collect::<Vec<_>>()
                .join(", ");
            format!("Ok(Interfaces {{ interfaces: [{ifaces}] }})")
        }
        Ok(val) => {
            format!(
                "Ok(Interfaces {{ interfaces: [{} × Interface] }})",
                val.interfaces.len()
            )
        }
        Err(err) => format!("{:?}", Err::<(), _>(err)),
    };

    println!(
        "> agent.list_interfaces() -> {res} ended: ({:?})",
        start.elapsed(),
    );
}

async fn grpc_online_cpu_mem(client: Client, start: Instant) {
    sleep(Duration::from_millis(22)).await;

    let req = OnlineCpuMemRequest::default();
    let res = client.online_cpu_mem(req).await;

    println!(
        "> agent.online_cpu_mem() -> {:?} ended: ({:?})",
        res,
        start.elapsed()
    );

    let req = CheckRequest::default();
    let res = client.version(req).await;
    println!("> health.version() -> {:?} ({:?})", res, start.elapsed());
}

async fn streaming_echo(client: Client, start: Instant) {
    let req = EchoPayload {
        seq: 1,
        msg: "Echo Me".to_string(),
    };
    let res = client.echo(req).await;

    println!(
        "> streaming.echo() -> {:?} ended: ({:?})",
        res,
        start.elapsed(),
    );
}

async fn streaming_echo_stream(client: Client, start: Instant) {
    let req = stream! {
        for seq in (0..100).step_by(2) {
            let msg = format!("{seq}: Echo in a stream");
            yield EchoPayload { seq, msg };
        }
    };
    let strm = client.echo_stream(req);

    let res = strm.try_collect::<Vec<_>>().await;

    let res = match res {
        Ok(val) => format!("Ok([{} × EchoPayload])", val.len()),
        Err(err) => format!("{:?}", Err::<(), _>(err)),
    };

    println!(
        "> streaming.echo_stream() -> {res} ended: ({:?})",
        start.elapsed(),
    );
}

async fn streaming_sum_stream(client: Client, start: Instant) {
    let req = {
        stream! {
            yield Part { add: 0 };

            for i in -99..=100 {
                let add = i;
                yield Part { add };
            }

            yield Part { add: 0 };
        }
    };
    let res = client.sum_stream(req).await;

    println!(
        "> streaming.sum_stream() -> {:?} ended: ({:?})",
        res,
        start.elapsed(),
    );
}

async fn streaming_divide_stream(client: Client, start: Instant) {
    let req = Sum { sum: 392, num: 4 };
    let strm = client.divide_stream(req);

    let res = strm.try_collect::<Vec<_>>().await;

    let res = match res {
        Ok(val) => format!("Ok([{} × Part])", val.len()),
        Err(err) => format!("{:?}", Err::<(), _>(err)),
    };

    println!(
        "> streaming.divide_stream() -> {res} ended: ({:?})",
        start.elapsed(),
    );
}

async fn streaming_echo_null(client: Client, start: Instant) {
    let req = {
        stream! {
            for seq in 0..100 {
                let msg = "non-empty empty".into();
                yield EchoPayload { seq, msg };
            }
        }
    };
    let res = client.echo_null(req).await;

    println!(
        "> streaming.echo_null() -> {:?} ended: ({:?})",
        res,
        start.elapsed(),
    );
}

async fn streaming_echo_null_stream(client: Client, start: Instant) {
    let req = {
        stream! {
            for seq in 0..100 {
                let msg = "non-empty empty".into();
                yield EchoPayload { seq, msg };
            }
        }
    };
    let strm = client.echo_null_stream(req);

    let res = strm.try_collect::<Vec<_>>().await;

    let res = match res {
        Ok(val) => format!("Ok([{} × ()])", val.len()),
        Err(err) => format!("{:?}", Err::<(), _>(err)),
    };

    println!(
        "> streaming.echo_null_stream() -> {res} ended: ({:?})",
        start.elapsed(),
    );
}

async fn streaming_echo_default_value(client: Client, start: Instant) {
    let req = EchoPayload::default();
    let strm = client.echo_default_value(req);

    let res = strm.try_collect::<Vec<_>>().await;

    let res = match res {
        Ok(val) => format!("Ok([{} × EchoPayload])", val.len()),
        Err(err) => format!("{:?}", Err::<(), _>(err)),
    };

    println!(
        "> streaming.echo_default_value() -> {res} ended: ({:?})",
        start.elapsed(),
    );
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let client = Client::connect(ADDRESS).await.unwrap();
    let client = client.with_metadata([
        ("key-1", "value-1-1"),
        ("key-1", "value-1-2"),
        ("key-2", "value-2"),
    ]);

    let start = std::time::Instant::now();

    let _ = tokio::join!(
        grpc_check(client.clone(), start),
        grpc_list_interfaces(client.clone(), start),
        grpc_online_cpu_mem(client.clone(), start),
        streaming_echo(client.clone(), start),
        streaming_echo_stream(client.clone(), start),
        streaming_sum_stream(client.clone(), start),
        streaming_divide_stream(client.clone(), start),
        streaming_echo_null(client.clone(), start),
        streaming_echo_null_stream(client.clone(), start),
        streaming_echo_default_value(client.clone(), start),
    );
}
