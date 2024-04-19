(
    "/__service_package__.__service_proto_name__/__method_proto_name__",
    {
        let target = std::sync::Arc::clone(&target);
        std::sync::Arc::new(trapeze::__codegen_prelude::__method_wrapper__::new(move |input| {
            let target = std::sync::Arc::clone(&target);
            __method_output_handler__
        }))
    },
),
