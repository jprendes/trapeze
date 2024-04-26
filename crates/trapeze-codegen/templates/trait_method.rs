__method_comments__
#[allow(unused_variables)]
fn __method_name__(
    &self,
    __method_input_name__: __method_input_type__,
) -> __method_output_type__ {
    let not_found = trapeze::Status {
        code: trapeze::Code::NotFound as i32,
        message: "/__service_package__.__service_proto_name__/__method_proto_name__ is not supported".into(),
        details: vec![],
    };
    __method_not_found__
}
