pub const ADDRESS: &str = "/tmp/ttrpc-test";

trapeze::include_protos!([
    "protos/agent.proto",
    "protos/health.proto",
    "protos/streaming.proto"
]);

pub use ttrpc::test::streaming;
