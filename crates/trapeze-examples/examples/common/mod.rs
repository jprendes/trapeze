trapeze::include_protos!([
    "protos/agent.proto",
    "protos/health.proto",
    "protos/streaming.proto"
]);

pub use ttrpc::test::streaming;

#[cfg(unix)]
pub const ADDRESS: &str = "unix:///tmp/ttrpc-test";

#[cfg(windows)]
pub const ADDRESS: &str = r"\\.\pipe\ttrpc-test";
