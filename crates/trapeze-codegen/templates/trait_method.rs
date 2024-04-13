__method_comments__
fn __method_name__(
    &self,
    ___method_input_name__: __method_input_type__,
) -> __method_output_type__ {
    let service = "__service_package__.__service_proto_name__";
    let method = "__method_proto_name__";
    std::boxed::Box::new(trapeze::__codegen_prelude::MethodNotFound { service, method }).__into_method_output__()
}
