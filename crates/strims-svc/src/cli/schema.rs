// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The frontend API's schema, from the descriptors strims-proto embeds:
//! the methods the command line calls, their messages as JSON, and
//! descriptions of them with the schema's comments.
//!
//! JSON is proto3's mapping: field names as in the schema (either the
//! schema's or the camelCase name on input), `bytes` in base64, 64-bit
//! integers as strings (numbers too on input), enums by name.

use std::fmt::Write as _;

use prost::Message as _;
use prost_reflect::{
    DescriptorPool, DeserializeOptions, DynamicMessage, EnumDescriptor, FieldDescriptor, Kind,
    MessageDescriptor, MethodDescriptor, SerializeOptions, ServiceDescriptor,
};
use prost_types::FileDescriptorProto;
use strims_node::app::FRONTEND_SERVICES;

/// The auth service, which a frontend connection serves before it signs
/// in.
pub(crate) const AUTH_SERVICE: &str = "strims.auth.v1.AuthFrontend";

/// The frontend API's schema.
pub(crate) struct Schema {
    pool: DescriptorPool,
}

impl Schema {
    pub(crate) fn load() -> Schema {
        #[allow(clippy::expect_used)]
        // invariant: strims-proto's build script encoded a set protox parsed
        let pool = DescriptorPool::decode(strims_proto::FILE_DESCRIPTOR_SET)
            .expect("strims-proto's descriptors decode");
        Schema { pool }
    }

    /// The services a frontend connection serves: auth, then the signed-in
    /// profile's.
    pub(crate) fn services(&self) -> Vec<ServiceDescriptor> {
        std::iter::once(AUTH_SERVICE)
            .chain(FRONTEND_SERVICES.iter().copied())
            .filter_map(|name| self.pool.get_service_by_name(name))
            .collect()
    }

    /// Every method of [`Schema::services`].
    pub(crate) fn methods(&self) -> Vec<MethodDescriptor> {
        self.services().iter().flat_map(|s| s.methods()).collect()
    }

    /// The frontend method `name` names: `Service.Method`, its package
    /// optional, in any case.
    pub(crate) fn method(&self, name: &str) -> Result<MethodDescriptor, String> {
        let want = name.to_ascii_lowercase();
        let methods = self.methods();
        if let Some(m) = methods.iter().find(|m| {
            m.full_name().eq_ignore_ascii_case(&want) || short_name(m).eq_ignore_ascii_case(&want)
        }) {
            return Ok(m.clone());
        }
        let tail = want.rsplit('.').next().unwrap_or(&want);
        let near: Vec<_> = methods
            .iter()
            .filter(|m| m.name().eq_ignore_ascii_case(tail))
            .map(short_name)
            .collect();
        Err(match near.as_slice() {
            [] => format!("no frontend method {name}; `strims-svc methods` lists them"),
            _ => format!(
                "no frontend method {name}; did you mean {}?",
                near.join(" or ")
            ),
        })
    }

    /// The frontend methods whose `Service.Method` contains `filter`, in
    /// any case, one a line: their request and response types.
    pub(crate) fn list(&self, filter: Option<&str>) -> String {
        let filter = filter.map(str::to_ascii_lowercase);
        let mut out = String::new();
        for m in self.methods() {
            let name = short_name(&m);
            if filter
                .as_ref()
                .is_some_and(|f| !name.to_ascii_lowercase().contains(f.as_str()))
            {
                continue;
            }
            let stream = if m.is_server_streaming() {
                "stream "
            } else {
                ""
            };
            let _ = writeln!(
                out,
                "{name}({}) -> {stream}{}",
                m.input().name(),
                m.output().name()
            );
        }
        out
    }

    /// A description of what `name` names: a frontend method (its request
    /// and response messages too), a message or an enum, by full name or,
    /// where it's unique, its own name.
    pub(crate) fn describe(&self, name: &str) -> Result<String, String> {
        if let Ok(m) = self.method(name) {
            let mut out = String::new();
            comment(
                &mut out,
                "",
                m.parent_file().file_descriptor_proto(),
                m.path(),
            );
            let stream = if m.is_server_streaming() {
                "stream "
            } else {
                ""
            };
            let _ = writeln!(
                out,
                "rpc {}({}) returns ({stream}{})\n",
                m.full_name(),
                m.input().full_name(),
                m.output().full_name()
            );
            describe_message(&mut out, &m.input());
            out.push('\n');
            describe_message(&mut out, &m.output());
            return Ok(out);
        }
        if let Some(m) = self.message(name) {
            let mut out = String::new();
            describe_message(&mut out, &m);
            return Ok(out);
        }
        if let Some(e) = self.enumeration(name) {
            let mut out = String::new();
            describe_enum(&mut out, &e);
            return Ok(out);
        }
        let named = self.named(name);
        Err(match named.as_slice() {
            [] => format!("no frontend method, message or enum {name}"),
            _ => format!("{name} names several: {}", named.join(", ")),
        })
    }

    /// The message `name` names: its full name, or its own where that's
    /// unique, in any case.
    pub(crate) fn find_message(&self, name: &str) -> Result<MessageDescriptor, String> {
        self.message(name).ok_or_else(|| {
            let named = self.named(name);
            match named.as_slice() {
                [] => format!("no message {name}"),
                _ => format!("{name} names several: {}", named.join(", ")),
            }
        })
    }

    /// The full names of the messages and enums whose own name is `name`,
    /// in any case.
    fn named(&self, name: &str) -> Vec<String> {
        let messages = self
            .pool
            .all_messages()
            .filter(|m| m.name().eq_ignore_ascii_case(name))
            .map(|m| m.full_name().to_owned());
        let enums = self
            .pool
            .all_enums()
            .filter(|e| e.name().eq_ignore_ascii_case(name))
            .map(|e| e.full_name().to_owned());
        messages.chain(enums).collect()
    }

    fn message(&self, name: &str) -> Option<MessageDescriptor> {
        self.pool.get_message_by_name(name).or_else(|| {
            unique(
                self.pool
                    .all_messages()
                    .filter(|m| m.name().eq_ignore_ascii_case(name)),
            )
        })
    }

    fn enumeration(&self, name: &str) -> Option<EnumDescriptor> {
        self.pool.get_enum_by_name(name).or_else(|| {
            unique(
                self.pool
                    .all_enums()
                    .filter(|e| e.name().eq_ignore_ascii_case(name)),
            )
        })
    }
}

/// The one item of `items`, if there's exactly one.
fn unique<T>(mut items: impl Iterator<Item = T>) -> Option<T> {
    let first = items.next()?;
    items.next().is_none().then_some(first)
}

/// `Service.Method`.
pub(crate) fn short_name(m: &MethodDescriptor) -> String {
    format!("{}.{}", m.parent_service().name(), m.name())
}

/// `json` as an encoded request of `method`; blank is the empty request.
pub(crate) fn request(method: &MethodDescriptor, json: &str) -> Result<Vec<u8>, String> {
    encode(&method.input(), json).map_err(|e| format!("the request {e}"))
}

/// `json` as an encoded message of type `desc`; blank is the empty
/// message.
pub(crate) fn encode(desc: &MessageDescriptor, json: &str) -> Result<Vec<u8>, String> {
    let json = if json.trim().is_empty() { "{}" } else { json };
    let mut de = serde_json::Deserializer::from_str(json);
    let message = DynamicMessage::deserialize_with_options(
        desc.clone(),
        &mut de,
        &DeserializeOptions::new().deny_unknown_fields(true),
    )
    .and_then(|m| de.end().map(|()| m))
    .map_err(|e| format!("isn't a {}: {e}", desc.full_name()))?;
    Ok(message.encode_to_vec())
}

/// How [`response`] writes JSON.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Style {
    /// Indented over lines, else on one line.
    pub(crate) pretty: bool,
    /// With fields at their default values, which proto3 JSON leaves out.
    pub(crate) defaults: bool,
}

/// The encoded message `b` of type `desc` as JSON.
pub(crate) fn response(desc: &MessageDescriptor, b: &[u8], style: Style) -> Result<String, String> {
    let message = DynamicMessage::decode(desc.clone(), b)
        .map_err(|e| format!("decoding a {}: {e}", desc.full_name()))?;
    let options = SerializeOptions::new()
        .use_proto_field_name(true)
        .skip_default_fields(!style.defaults);
    let mut out = Vec::new();
    let written = if style.pretty {
        message.serialize_with_options(&mut serde_json::Serializer::pretty(&mut out), &options)
    } else {
        message.serialize_with_options(&mut serde_json::Serializer::new(&mut out), &options)
    };
    written.map_err(|e| format!("writing a {} as JSON: {e}", desc.full_name()))?;
    String::from_utf8(out).map_err(|e| e.to_string())
}

/// Appends `m`'s fields, with their comments, as the schema declares them.
fn describe_message(out: &mut String, m: &MessageDescriptor) {
    let file = m.parent_file();
    let file = file.file_descriptor_proto();
    comment(out, "", file, m.path());
    let _ = writeln!(out, "message {} {{", m.full_name());
    let mut oneofs_done = Vec::new();
    for f in m.fields() {
        match f.containing_oneof().filter(|o| !o.is_synthetic()) {
            Some(o) => {
                if oneofs_done.contains(&o.name().to_owned()) {
                    continue;
                }
                oneofs_done.push(o.name().to_owned());
                comment(out, "  ", file, o.path());
                let _ = writeln!(out, "  oneof {} {{", o.name());
                for f in o.fields() {
                    describe_field(out, "    ", file, &f);
                }
                let _ = writeln!(out, "  }}");
            }
            None => describe_field(out, "  ", file, &f),
        }
    }
    let _ = writeln!(out, "}}");
}

fn describe_field(out: &mut String, indent: &str, file: &FileDescriptorProto, f: &FieldDescriptor) {
    comment(out, indent, file, f.path());
    let ty = if f.is_map() {
        match f.kind() {
            Kind::Message(entry) => {
                let key = entry.map_entry_key_field();
                let value = entry.map_entry_value_field();
                format!(
                    "map<{}, {}>",
                    kind_name(&key.kind()),
                    kind_name(&value.kind())
                )
            }
            k => kind_name(&k),
        }
    } else if f.is_list() {
        format!("repeated {}", kind_name(&f.kind()))
    } else if f.containing_oneof().is_some_and(|o| o.is_synthetic()) {
        format!("optional {}", kind_name(&f.kind()))
    } else {
        kind_name(&f.kind())
    };
    let _ = writeln!(out, "{indent}{ty} {} = {};", f.name(), f.number());
}

fn describe_enum(out: &mut String, e: &EnumDescriptor) {
    let file = e.parent_file();
    let file = file.file_descriptor_proto();
    comment(out, "", file, e.path());
    let _ = writeln!(out, "enum {} {{", e.full_name());
    for v in e.values() {
        comment(out, "  ", file, v.path());
        let _ = writeln!(out, "  {} = {};", v.name(), v.number());
    }
    let _ = writeln!(out, "}}");
}

/// A field type as the schema writes it.
fn kind_name(k: &Kind) -> String {
    match k {
        Kind::Double => "double".into(),
        Kind::Float => "float".into(),
        Kind::Int32 => "int32".into(),
        Kind::Int64 => "int64".into(),
        Kind::Uint32 => "uint32".into(),
        Kind::Uint64 => "uint64".into(),
        Kind::Sint32 => "sint32".into(),
        Kind::Sint64 => "sint64".into(),
        Kind::Fixed32 => "fixed32".into(),
        Kind::Fixed64 => "fixed64".into(),
        Kind::Sfixed32 => "sfixed32".into(),
        Kind::Sfixed64 => "sfixed64".into(),
        Kind::Bool => "bool".into(),
        Kind::String => "string".into(),
        Kind::Bytes => "bytes".into(),
        Kind::Message(m) => m.full_name().into(),
        Kind::Enum(e) => e.full_name().into(),
    }
}

/// Appends the schema's comment on the item at `path` in `file`, each line
/// under `// `.
fn comment(out: &mut String, indent: &str, file: &FileDescriptorProto, path: &[i32]) {
    let Some(info) = &file.source_code_info else {
        return;
    };
    let Some(location) = info.location.iter().find(|l| l.path == path) else {
        return;
    };
    let text = location
        .leading_comments
        .as_deref()
        .or(location.trailing_comments.as_deref())
        .unwrap_or("");
    for line in text.trim_end().lines() {
        let line = line.strip_prefix(' ').unwrap_or(line);
        let _ = writeln!(out, "{indent}// {line}");
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]

    use std::collections::BTreeMap;

    use prost::Message;
    use strims_proto::network::v1::directory::listing::{Content, Embed, embed};
    use strims_proto::network::v1::directory::{FrontendPublishRequest, Listing};
    use strims_proto::network::v1::{CreateInvitationRequest, CreateServerRequest};

    use super::*;

    fn round_trip<M: Message + Default + PartialEq + std::fmt::Debug>(method: &str, want: M) {
        let schema = Schema::load();
        let m = schema.method(method).unwrap();
        let json = response(&m.input(), &want.encode_to_vec(), Style::default()).unwrap();
        let back = M::decode(request(&m, &json).unwrap().as_slice()).unwrap();
        assert_eq!(back, want, "{json}");
    }

    #[test]
    fn requests_round_trip_through_json() {
        round_trip(
            "NetworkFrontend.CreateServer",
            CreateServerRequest {
                name: "streams".into(),
                ..Default::default()
            },
        );
        // 64-bit integers
        round_trip(
            "NetworkFrontend.CreateInvitation",
            CreateInvitationRequest {
                network_id: u64::MAX - 1,
                bootstrap_client_id: 3,
            },
        );
        // bytes, a oneof, an enum and a map
        round_trip(
            "DirectoryFrontend.Publish",
            FrontendPublishRequest {
                network_key: vec![0, 1, 254, 255],
                listing: Some(Listing {
                    content: Some(Content::Embed(Embed {
                        service: embed::Service::DirectoryListingEmbedServiceYoutube as i32,
                        id: "abc".into(),
                        query_params: BTreeMap::from([("t".into(), "10".into())]),
                    })),
                }),
            },
        );
    }

    #[test]
    fn json_takes_the_schemas_names_and_proto3s_encodings() {
        let schema = Schema::load();
        let m = schema.method("networkfrontend.createinvitation").unwrap();
        let by_number = request(&m, r#"{"network_id": 7, "bootstrapClientId": "8"}"#).unwrap();
        assert_eq!(
            CreateInvitationRequest::decode(by_number.as_slice()).unwrap(),
            CreateInvitationRequest {
                network_id: 7,
                bootstrap_client_id: 8,
            }
        );
        let json = response(&m.input(), &by_number, Style::default()).unwrap();
        assert_eq!(json, r#"{"network_id":"7","bootstrap_client_id":"8"}"#);
        assert!(request(&m, "").unwrap().is_empty());

        let bad = request(&m, r#"{"network": 7}"#).unwrap_err();
        assert!(bad.contains("CreateInvitationRequest"), "{bad}");
        assert!(request(&m, "{} trailing").is_err());
    }

    #[test]
    fn methods_resolve_by_service_and_name_in_any_case() {
        let schema = Schema::load();
        for name in [
            "NetworkFrontend.CreateServer",
            "networkfrontend.createserver",
            "strims.network.v1.NetworkFrontend.CreateServer",
        ] {
            assert_eq!(
                schema.method(name).unwrap().full_name(),
                "strims.network.v1.NetworkFrontend.CreateServer"
            );
        }
        let near = schema.method("CreateServer").unwrap_err();
        assert!(
            near.contains("NetworkFrontend.CreateServer")
                && near.contains("ChatServerFrontend.CreateServer"),
            "{near}"
        );
        // a service the frontend doesn't serve
        assert!(schema.method("CA.Renew").is_err());
        assert!(schema.method("AuthFrontend.SignUp").is_ok());
    }

    #[test]
    fn every_frontend_service_is_in_the_schema() {
        assert_eq!(Schema::load().services().len(), FRONTEND_SERVICES.len() + 1);
    }

    #[test]
    fn descriptions_show_fields_oneofs_and_comments() {
        let schema = Schema::load();
        let d = schema
            .describe("NetworkFrontend.CreateNetworkFromInvitation")
            .unwrap();
        assert!(
            d.contains("rpc strims.network.v1.NetworkFrontend.CreateNetworkFromInvitation("),
            "{d}"
        );
        assert!(d.contains("  oneof invitation {"), "{d}");
        assert!(d.contains("    bytes invitation_bytes = 1002;"), "{d}");

        let listings = schema.describe("DirectoryFrontend.WatchListings").unwrap();
        assert!(listings.contains("returns (stream "), "{listings}");

        let e = schema
            .describe("strims.network.v1.directory.ListingContentType")
            .unwrap();
        assert!(e.starts_with("enum ") || e.contains("\nenum "), "{e}");
        assert!(schema.describe("NoSuchThing").is_err());

        // a name two packages share
        let both = schema.find_message("CreateServerRequest").unwrap_err();
        assert!(
            both.contains("strims.network.v1.CreateServerRequest")
                && both.contains("strims.chat.v1.CreateServerRequest"),
            "{both}"
        );
        assert!(schema.find_message("CertificateBody").is_ok());
    }
}
