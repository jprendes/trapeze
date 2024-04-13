pub const ADDRESS: &str = "/tmp/ttrpc-test";

trapeze::include_protos!(["agent.proto", "health.proto", "streaming.proto"]);

pub use ttrpc::test::streaming;
