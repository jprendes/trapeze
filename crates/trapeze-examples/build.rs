fn main() {
    trapeze_codegen::Config::new()
        .include_file("mod.rs")
        .compile_protos(
            &[
                "protos/agent.proto",
                "protos/health.proto",
                "protos/streaming.proto",
            ],
            &["protos/"],
        )
        .unwrap();
}
