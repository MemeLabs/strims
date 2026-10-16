// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Emits RPC service interfaces into each package's prost file: per
//! service, a `{snake}_methods` module of method markers, a `{Name}Service`
//! trait, `register_{snake}_service` and a `{Name}Client`. `src/service.rs`
//! documents the items and their contracts.
//!
//! Generated names share the package module with prost's, so a collision
//! with a message, enum or nested-type module fails the build, as does a
//! client-streaming method (the wire format has none).

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::rc::Rc;

use heck::ToSnakeCase;
use prost_build::{Comments, Service, ServiceGenerator};
use prost_types::FileDescriptorSet;

use crate::stable::{sanitize_identifier, to_snake, to_upper_camel};

/// Generic parameter names of the generated items, which must not shadow a
/// package type.
const GENERIC_NAMES: [&str; 3] = ["C", "R", "S"];

/// Inherent methods of the generated client.
const CLIENT_METHODS: [&str; 2] = ["new", "caller"];

pub struct RpcGenerator {
    /// Per package, the item and module names prost generates.
    taken: BTreeMap<String, BTreeSet<String>>,
    /// `ServiceGenerator` can't fail, so errors are collected here and
    /// reported once prost is done.
    errors: Rc<RefCell<Vec<String>>>,
}

impl RpcGenerator {
    pub fn new(fds: &FileDescriptorSet, errors: Rc<RefCell<Vec<String>>>) -> RpcGenerator {
        let mut taken: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for file in &fds.file {
            let names = taken.entry(file.package().to_owned()).or_default();
            for m in &file.message_type {
                names.insert(to_upper_camel(m.name()));
                names.insert(to_snake(m.name()));
            }
            for e in &file.enum_type {
                names.insert(to_upper_camel(e.name()));
            }
        }
        RpcGenerator { taken, errors }
    }

    fn write(&self, service: &Service, buf: &mut String) -> Result<(), String> {
        let full = format!("{}.{}", service.package, service.proto_name);
        let names = Names::new(service);
        self.check(service, &full, &names)?;
        write_service(service, &full, &names, buf).map_err(|e| format!("{full}: {e}"))
    }

    fn check(&self, service: &Service, full: &str, names: &Names) -> Result<(), String> {
        if let Some(m) = service.methods.iter().find(|m| m.client_streaming) {
            return Err(format!(
                "{full}.{}: client streaming is not supported",
                m.proto_name
            ));
        }
        let taken = self.taken.get(&service.package);
        let generated = [
            names.methods_mod.as_str(),
            &names.service_trait,
            &names.client,
            &names.register_fn,
        ];
        for name in generated.into_iter().chain(GENERIC_NAMES) {
            if taken.is_some_and(|t| t.contains(name)) {
                return Err(format!(
                    "{full}: generated name {name} collides with a type in package {}",
                    service.package
                ));
            }
        }
        let mut fns = BTreeSet::new();
        let mut markers = BTreeSet::new();
        for m in &service.methods {
            if CLIENT_METHODS.contains(&m.name.as_str()) || !fns.insert(m.name.as_str()) {
                return Err(format!(
                    "{full}.{}: method name {} collides",
                    m.proto_name, m.name
                ));
            }
            if !markers.insert(to_upper_camel(&m.proto_name)) {
                return Err(format!("{full}.{}: marker name collides", m.proto_name));
            }
        }
        Ok(())
    }
}

impl ServiceGenerator for RpcGenerator {
    fn generate(&mut self, service: Service, buf: &mut String) {
        if let Err(e) = self.write(&service, buf) {
            self.errors.borrow_mut().push(e);
        }
    }
}

/// The generated item names of one service.
struct Names {
    methods_mod: String,
    service_trait: String,
    client: String,
    register_fn: String,
}

impl Names {
    fn new(service: &Service) -> Names {
        let snake = service.proto_name.to_snake_case();
        Names {
            methods_mod: sanitize_identifier(&format!("{snake}_methods")),
            service_trait: format!("{}Service", service.name),
            client: format!("{}Client", service.name),
            register_fn: sanitize_identifier(&format!("register_{snake}_service")),
        }
    }
}

/// `path` (relative to the package module) as seen from a module nested one
/// level inside it.
fn from_child_module(path: &str) -> String {
    if path.starts_with("::") || path.starts_with("crate::") {
        path.to_owned()
    } else {
        format!("super::{path}")
    }
}

fn write_comments(comments: &Comments, indent: u8, buf: &mut String) {
    if !comments.leading.is_empty() || !comments.trailing.is_empty() {
        for _ in 0..indent {
            buf.push_str("    ");
        }
        buf.push_str("///\n");
        comments.append_with_indent(indent, buf);
    }
}

fn write_service(
    service: &Service,
    full: &str,
    names: &Names,
    buf: &mut String,
) -> std::fmt::Result {
    let Names {
        methods_mod,
        service_trait,
        client,
        register_fn,
    } = names;
    let fut = "::core::future::Future";
    let result = "::core::result::Result";
    let svc = "crate::service";

    // markers
    writeln!(buf, "/// Method markers of service `{full}`.")?;
    writeln!(buf, "pub mod {methods_mod} {{")?;
    for m in &service.methods {
        let marker = to_upper_camel(&m.proto_name);
        let kind = if m.server_streaming {
            "StreamingMethod"
        } else {
            "UnaryMethod"
        };
        writeln!(buf, "    /// `{full}.{}`.", m.proto_name)?;
        writeln!(
            buf,
            "    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]"
        )?;
        writeln!(buf, "    pub struct {marker};")?;
        writeln!(buf, "    impl {svc}::Method for {marker} {{")?;
        writeln!(
            buf,
            "        const NAME: &'static str = \"{full}.{}\";",
            m.proto_name
        )?;
        writeln!(
            buf,
            "        type Request = {};",
            from_child_module(&m.input_type)
        )?;
        writeln!(
            buf,
            "        type Response = {};",
            from_child_module(&m.output_type)
        )?;
        writeln!(buf, "    }}")?;
        writeln!(buf, "    impl {svc}::{kind} for {marker} {{}}")?;
    }
    writeln!(buf, "}}")?;

    // service trait
    writeln!(buf, "/// Service `{full}`.")?;
    writeln!(buf, "///")?;
    writeln!(
        buf,
        "/// Methods without an implementation answer `Error::unimplemented()`."
    )?;
    write_comments(&service.comments, 0, buf);
    writeln!(buf, "pub trait {service_trait} {{")?;
    for m in &service.methods {
        let output = if m.server_streaming {
            format!("{svc}::ServerStream<{}>", m.output_type)
        } else {
            m.output_type.clone()
        };
        writeln!(buf, "    /// `{full}.{}`.", m.proto_name)?;
        write_comments(&m.comments, 1, buf);
        writeln!(buf, "    fn {}(", m.name)?;
        writeln!(buf, "        &self,")?;
        writeln!(buf, "        ctx: &{svc}::Context,")?;
        writeln!(buf, "        req: {},", m.input_type)?;
        writeln!(
            buf,
            "    ) -> impl {fut}<Output = {result}<{output}, {svc}::Error>> {{"
        )?;
        writeln!(buf, "        let _ = (ctx, req);")?;
        writeln!(
            buf,
            "        ::core::future::ready({result}::Err({svc}::Error::unimplemented()))"
        )?;
        writeln!(buf, "    }}")?;
    }
    writeln!(buf, "}}")?;

    // registration
    writeln!(
        buf,
        "/// Registers every method of `{full}`, served by `service`."
    )?;
    writeln!(
        buf,
        "pub fn {register_fn}<R, S>(registry: &mut R, service: ::std::rc::Rc<S>)"
    )?;
    writeln!(buf, "where")?;
    writeln!(buf, "    R: {svc}::Registry,")?;
    writeln!(buf, "    S: {service_trait} + 'static,")?;
    writeln!(buf, "{{")?;
    for m in &service.methods {
        let marker = to_upper_camel(&m.proto_name);
        let register = if m.server_streaming {
            "register_streaming"
        } else {
            "register_unary"
        };
        writeln!(buf, "    {{")?;
        writeln!(buf, "        let service = ::std::rc::Rc::clone(&service);")?;
        writeln!(
            buf,
            "        registry.{register}::<{methods_mod}::{marker}, _, _>(move |ctx, req| {{"
        )?;
        writeln!(
            buf,
            "            let service = ::std::rc::Rc::clone(&service);"
        )?;
        writeln!(
            buf,
            "            async move {{ service.{}(&ctx, req).await }}",
            m.name
        )?;
        writeln!(buf, "        }});")?;
        writeln!(buf, "    }}")?;
    }
    writeln!(buf, "}}")?;

    // client
    writeln!(buf, "/// Client of service `{full}`.")?;
    writeln!(buf, "#[derive(Clone, Debug)]")?;
    writeln!(buf, "pub struct {client}<C> {{")?;
    writeln!(buf, "    caller: C,")?;
    writeln!(buf, "}}")?;
    writeln!(buf, "impl<C> {client}<C> {{")?;
    writeln!(buf, "    /// A client that calls through `caller`.")?;
    writeln!(buf, "    pub fn new(caller: C) -> Self {{")?;
    writeln!(buf, "        {client} {{ caller }}")?;
    writeln!(buf, "    }}")?;
    writeln!(buf, "    /// The caller this client calls through.")?;
    writeln!(buf, "    pub fn caller(&self) -> &C {{")?;
    writeln!(buf, "        &self.caller")?;
    writeln!(buf, "    }}")?;
    writeln!(buf, "}}")?;
    writeln!(buf, "impl<C: {svc}::Caller> {client}<C> {{")?;
    for m in &service.methods {
        let marker = to_upper_camel(&m.proto_name);
        writeln!(buf, "    /// Calls `{full}.{}`.", m.proto_name)?;
        if m.server_streaming {
            writeln!(buf, "    ///")?;
            writeln!(buf, "    /// Dropping the stream cancels the call.")?;
        }
        write_comments(&m.comments, 1, buf);
        let (output, call) = if m.server_streaming {
            (format!("C::Stream<{}>", m.output_type), "call_streaming")
        } else {
            (m.output_type.clone(), "call_unary")
        };
        writeln!(
            buf,
            "    pub fn {}(&self, req: &{}) -> impl {fut}<Output = {result}<{output}, {svc}::CallError>> + use<C> {{",
            m.name, m.input_type
        )?;
        writeln!(
            buf,
            "        self.caller.{call}::<{methods_mod}::{marker}>(req)"
        )?;
        writeln!(buf, "    }}")?;
    }
    writeln!(buf, "}}")?;
    Ok(())
}
