use std::collections::HashMap;
use std::io::Result;
use std::ops::{Deref, DerefMut};
use std::path::Path;

pub use prost_build;

pub use prost_build::{protoc_from_env, protoc_include_from_env};
use prost_build::{Method, Service, ServiceGenerator};

pub fn compile_protos(protos: &[impl AsRef<Path>], includes: &[impl AsRef<Path>]) -> Result<()> {
    Config::new().compile_protos(protos, includes)
}

pub struct Config(prost_build::Config);

impl Config {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut cfg = prost_build::Config::new();
        cfg.service_generator(Box::new(TtrpcServiceGenerator));
        Self(cfg)
    }
}

impl Deref for Config {
    type Target = prost_build::Config;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Config {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct TtrpcServiceGenerator;

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

fn make_trait_method(mut substitutions: HashMap<&'static str, String>, method: &Method) -> String {
    substitutions.extend(method_substitutions(method));

    let Method {
        client_streaming,
        server_streaming,
        ..
    } = method;

    if *client_streaming || *server_streaming {
        panic!("Streaming server or client not supported");
    }

    replace(include_str!("../templates/trait_method.rs"), substitutions)
}

/*
fn make_trait_ctx_method(mut substitutions: HashMap<&'static str, String>, method: &Method) -> String {
    let mut method = method.clone();

    if method.input_type.starts_with("super::") {
        method.input_type.insert_str(0, "super::");
    }
    if method.output_type.starts_with("super::") {
        method.output_type.insert_str(0, "super::");
    }

    substitutions.extend(method_substitutions(&method));

    replace(include_str!("../templates/trait_ctx_method.rs"), substitutions)
}
*/

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

impl ServiceGenerator for TtrpcServiceGenerator {
    fn generate(&mut self, service: Service, buf: &mut String) {
        let mut substitutions = service_substitutions(&service);

        let make_client_method = |m| make_client_method(substitutions.clone(), m);
        let make_trait_method = |m| make_trait_method(substitutions.clone(), m);
        //let make_trait_ctx_method = |m| make_trait_ctx_method(substitutions.clone(), m);
        let make_dispatch_branch = |m| make_dispatch_branch(substitutions.clone(), m);

        let methods = service.methods;

        let client_methods: String = methods.iter().map(make_client_method).collect();
        let trait_methods: String = methods.iter().map(make_trait_method).collect();
        //let trait_ctx_methods: String = methods.iter().map(make_trait_ctx_method).collect();
        let dispatch_branches: String = methods.iter().map(make_dispatch_branch).collect();

        substitutions.insert("client_methods", client_methods);
        substitutions.insert("trait_methods", trait_methods);
        //substitutions.insert("trait_ctx_methods", trait_ctx_methods);
        substitutions.insert("dispatch_branches", dispatch_branches);

        let service = replace(include_str!("../templates/service.rs"), substitutions);

        buf.push_str(&service);
    }
}

fn service_substitutions(service: &Service) -> HashMap<&'static str, String> {
    let mut substitutions: HashMap<&'static str, String> = Default::default();
    substitutions.insert("service_name", service.name.clone());
    substitutions.insert("service_package", service.package.clone());
    substitutions.insert("service_proto_name", service.proto_name.clone());
    substitutions.insert("service_module_name", camel2snake(&service.name));
    substitutions
}

fn method_substitutions(method: &Method) -> HashMap<&'static str, String> {
    let mut substitutions: HashMap<&'static str, String> = Default::default();
    substitutions.insert("method_name", method.name.clone());
    substitutions.insert("method_proto_name", method.proto_name.clone());
    substitutions.insert("method_input_type", method.input_type.clone());
    substitutions.insert("method_input_name", camel2snake(&method.input_type));
    substitutions.insert("method_output_type", method.output_type.clone());
    substitutions
}

fn replace(src: impl ToString, substitutions: HashMap<&'static str, String>) -> String {
    let mut src = src.to_string();
    for (from, to) in substitutions {
        src = src.replace(&format!("__{from}__"), &to);
    }
    src
}
