use std::collections::HashMap;

use prost_build::{Comments, Method, Service, ServiceGenerator};

/// A service generator that takes a service descriptor and generates Rust code for a `ttrpc` service.
///
/// It generates a trait describing methods of the service and implements the trait for a `trapeze::Client`.
/// To implement a server, users should implement the trait on their own objects.
pub struct TtrpcServiceGenerator;

impl ServiceGenerator for TtrpcServiceGenerator {
    fn generate(&mut self, service: Service, buf: &mut String) {
        let mut substitutions = service_substitutions(&service);

        let make_client_method = |m| make_client_method(substitutions.clone(), m);
        let make_trait_method = |m| make_trait_method(substitutions.clone(), m);
        let make_dispatch_branch = |m| make_dispatch_branch(substitutions.clone(), m);

        let methods = service.methods;

        let client_methods: String = methods.iter().map(make_client_method).collect();
        let trait_methods: String = methods.iter().map(make_trait_method).collect();
        let dispatch_branches: String = methods.iter().map(make_dispatch_branch).collect();

        substitutions.insert("client_methods", client_methods);
        substitutions.insert("trait_methods", trait_methods);
        substitutions.insert("dispatch_branches", dispatch_branches);

        let service = replace(include_str!("../templates/service.rs"), substitutions);

        buf.push_str(&service);
    }
}

fn make_trait_method(mut substitutions: HashMap<&'static str, String>, method: &Method) -> String {
    substitutions.extend(method_substitutions(method));

    replace(include_str!("../templates/trait_method.rs"), substitutions)
}

fn make_dispatch_branch(
    mut substitutions: HashMap<&'static str, String>,
    method: &Method,
) -> String {
    substitutions.extend(method_substitutions(method));

    replace(
        include_str!("../templates/dispatch_branch.rs"),
        substitutions,
    )
}

fn make_client_method(mut substitutions: HashMap<&'static str, String>, method: &Method) -> String {
    substitutions.extend(method_substitutions(method));

    replace(include_str!("../templates/client_method.rs"), substitutions)
}

fn service_substitutions(service: &Service) -> HashMap<&'static str, String> {
    let mut substitutions = HashMap::default();
    substitutions.insert("service_comments", format_comments(&service.comments, 0));
    substitutions.insert("service_name", service.name.clone());
    substitutions.insert("service_package", service.package.clone());
    substitutions.insert("service_proto_name", service.proto_name.clone());
    substitutions.insert("service_module_name", camel2snake(&service.name));
    substitutions
}

fn method_substitutions(method: &Method) -> HashMap<&'static str, String> {
    let mut substitutions = HashMap::default();
    let Method {
        name,
        proto_name,
        input_type,
        output_type,
        client_streaming,
        server_streaming,
        comments,
        ..
    } = method;

    let input_name = camel2snake(input_type);

    let wrapper = match (*client_streaming, *server_streaming) {
        (false, false) => "UnaryMethod",
        (false, true) => "ServerStreamingMethod",
        (true, false) => "ClientStreamingMethod",
        (true, true) => "DuplexStreamingMethod",
    };

    let request_handler = match (*client_streaming, *server_streaming) {
        (false, false) => "handle_unary_request",
        (false, true) => "handle_server_streaming_request",
        (true, false) => "handle_client_streaming_request",
        (true, true) => "handle_duplex_streaming_request",
    };

    let input_type = if *client_streaming {
        stream_for(input_type)
    } else {
        input_type.clone()
    };

    let output_type = if *server_streaming {
        fallible_stream_for(output_type)
    } else {
        fallible_future_for(output_type)
    };

    let output_handler = if *server_streaming {
        stream_handler(name)
    } else {
        future_handler(name)
    };

    let not_found = if *server_streaming {
        not_found_stream()
    } else {
        not_found_future()
    };

    substitutions.insert("method_comments", format_comments(comments, 1));
    substitutions.insert("method_name", name.clone());
    substitutions.insert("method_proto_name", proto_name.clone());
    substitutions.insert("method_input_name", input_name);
    substitutions.insert("method_input_type", input_type);
    substitutions.insert("method_output_type", output_type);
    substitutions.insert("method_wrapper", wrapper.to_string());
    substitutions.insert("method_request_handler", request_handler.to_string());
    substitutions.insert("method_output_handler", output_handler);
    substitutions.insert("method_not_found", not_found);
    substitutions
}

fn format_comments(comments: &Comments, indent_level: u8) -> String {
    let mut formatted = String::new();
    comments.append_with_indent(indent_level, &mut formatted);
    formatted
}

fn future_for(ty: &str) -> String {
    format!("impl trapeze::prelude::Future<Output = {ty}> + Send")
}

fn fallible_future_for(ty: &str) -> String {
    future_for(&format!("trapeze::Result<{ty}>"))
}

fn stream_for(ty: &str) -> String {
    format!("impl trapeze::prelude::Stream<Item = {ty}> + Send")
}

fn fallible_stream_for(ty: &str) -> String {
    stream_for(&format!("trapeze::Result<{ty}>"))
}

fn replace(src: impl Into<String>, substitutions: HashMap<&'static str, String>) -> String {
    let mut src = src.into();
    for (from, to) in substitutions {
        src = src.replace(&format!("__{from}__"), &to);
    }
    src
}

fn future_handler(method_name: &str) -> String {
    format!("async move {{ target.{method_name}(input).await }}")
}

fn stream_handler(method_name: &str) -> String {
    format!("trapeze::stream::stream! {{ for await value in target.{method_name}(input) {{ yield value; }} }}")
}

fn not_found_future() -> String {
    "async move { Err(not_found) }".into()
}

fn not_found_stream() -> String {
    "trapeze::stream::stream! { yield Err(not_found); }".into()
}

fn camel2snake(name: impl AsRef<str>) -> String {
    name.as_ref()
        .split("::")
        .last()
        .unwrap()
        .chars()
        .enumerate()
        .flat_map(|(i, c)| {
            if i > 0 && c.is_uppercase() {
                vec!['_'].into_iter().chain(c.to_lowercase())
            } else {
                vec![].into_iter().chain(c.to_lowercase())
            }
        })
        .collect()
}
