async fn __method_name__(&self, __method_input_name__: __method_input_type__) -> trapeze::Result<__method_output_type__> {
    trapeze::client::ClientImpl::request(self, "__service_package__.__service_proto_name__", "__method_proto_name__", &__method_input_name__).await
}
