//! Shared conversion from Parsely's CBOR tree to a Symbolica Atom.

use std::io::Cursor;

use ciborium::value::Value;
use symbolica::prelude::{Atom, AtomCore, Symbol};
use tymbolica_atom_payload::{AttachmentSet, PayloadFormat, parse_payload};

/// A parsed Symbolica Atom together with every portable attachment carried by
/// embedded Atom payloads in the source tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttachedAtom {
    pub atom: Atom,
    pub attachments: AttachmentSet,
}

/// The complete, import-free payload preflight for one supported Parsely tree.
///
/// Consumers with stricter trust requirements can reject legacy native Atom
/// exports before calling [`attached_atom_from_ast`]. The legacy marker follows
/// exactly the same schema-aware walk as the merged attachment set, so bytes in
/// unconsumed metadata do not affect it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AstPayloadPreflight {
    pub attachments: AttachmentSet,
    pub has_legacy_payload: bool,
}

/// Decode a CBOR-encoded Parsely tree and convert it to a Symbolica Atom.
///
/// This compatibility convenience discards portable attachments. New callers
/// that return another payload should use [`attached_atom_from_ast`].
pub fn atom_from_ast(input: &[u8], namespace: &str, label: &str) -> Result<Atom, String> {
    attached_atom_from_ast(input, namespace, label).map(|parsed| parsed.atom)
}

/// Decode a CBOR-encoded Parsely tree without discarding attachments carried
/// by Atom payloads nested anywhere in the tree.
pub fn attached_atom_from_ast(
    input: &[u8],
    namespace: &str,
    label: &str,
) -> Result<AttachedAtom, String> {
    tymbolica_symbol_registry::initialize();
    let value = ciborium::from_reader::<Value, _>(Cursor::new(input))
        .map_err(|error| format!("{label} must be CBOR-encoded: {error}"))?;
    attached_atom_from_value(&value, namespace)
}

/// Inspect every Atom payload the supported AST grammar will consume, merge
/// its attachments, and validate envelope revisions without importing an Atom.
///
/// Consumers that must initialize attachment-defined registries before
/// Symbolica import can call this first, perform registration, and only then
/// call [`attached_atom_from_ast`].
pub fn attachments_from_ast(input: &[u8], label: &str) -> Result<AttachmentSet, String> {
    preflight_payloads_from_ast(input, label).map(|preflight| preflight.attachments)
}

/// Inspect every grammar-consumed Atom payload without importing it.
pub fn preflight_payloads_from_ast(
    input: &[u8],
    label: &str,
) -> Result<AstPayloadPreflight, String> {
    let value = ciborium::from_reader::<Value, _>(Cursor::new(input))
        .map_err(|error| format!("{label} must be CBOR-encoded: {error}"))?;
    preflight_payloads_from_value(&value)
}

/// Preflight the Atom payloads recursively consumed from one walked value.
pub fn attachments_from_value(value: &Value) -> Result<AttachmentSet, String> {
    preflight_payloads_from_value(value).map(|preflight| preflight.attachments)
}

/// Inspect every grammar-consumed Atom payload in an already decoded value.
pub fn preflight_payloads_from_value(value: &Value) -> Result<AstPayloadPreflight, String> {
    let mut preflight = AstPayloadPreflight::default();
    inspect_value_attachments(value, &mut preflight)?;
    Ok(preflight)
}

/// Convert one value in a walked Parsely tree to a Symbolica Atom.
///
/// This compatibility convenience discards portable attachments. New callers
/// that return another payload should use [`attached_atom_from_value`].
pub fn atom_from_value(value: &Value, namespace: &str) -> Result<Atom, String> {
    attached_atom_from_value(value, namespace).map(|parsed| parsed.atom)
}

/// Convert one walked Parsely value while merging attachments from every
/// recursively embedded Atom payload.
pub fn attached_atom_from_value(value: &Value, namespace: &str) -> Result<AttachedAtom, String> {
    tymbolica_symbol_registry::initialize();
    // Validate and merge the complete attachment environment before importing
    // any Atom, so callers never observe a partially initialized traversal.
    let attachments = attachments_from_value(value)?;
    let mut imported_attachments = AttachmentSet::new();
    let atom = atom_from_value_collect(value, namespace, &mut imported_attachments)?;
    debug_assert_eq!(attachments, imported_attachments);
    Ok(AttachedAtom { atom, attachments })
}

fn atom_from_value_collect(
    value: &Value,
    namespace: &str,
    attachments: &mut AttachmentSet,
) -> Result<Atom, String> {
    match value {
        Value::Bytes(bytes) => decode_embedded_atom(bytes, "embedded Atom payload", attachments),
        Value::Integer(number) => {
            let number: i64 = (*number)
                .try_into()
                .map_err(|_| "integer literal is out of range".to_owned())?;
            Ok(Atom::num(number))
        }
        Value::Float(number) => Ok(Atom::num(*number)),
        Value::Text(text) => atom_from_leaf(text, namespace),
        Value::Map(map) => atom_from_node(map, namespace, attachments),
        other => Err(format!("unsupported Parsely AST value: {other:?}")),
    }
}

fn decode_embedded_atom(
    bytes: &[u8],
    label: &str,
    attachments: &mut AttachmentSet,
) -> Result<Atom, String> {
    let payload = parse_payload(bytes).map_err(|error| format!("{label} is invalid: {error}"))?;
    let atom = payload
        .import_atom()
        .map_err(|error| format!("{label} is invalid: {error}"))?;
    attachments
        .merge(&payload.attachment_set())
        .map_err(|error| format!("could not merge {label} attachments: {error}"))?;
    Ok(atom)
}

fn inspect_payload_attachments(
    bytes: &[u8],
    label: &str,
    preflight: &mut AstPayloadPreflight,
) -> Result<(), String> {
    let payload = parse_payload(bytes).map_err(|error| format!("{label} is invalid: {error}"))?;
    payload
        .ensure_import_compatible()
        .map_err(|error| format!("{label} is incompatible: {error}"))?;
    preflight.has_legacy_payload |= payload.format() == PayloadFormat::LegacyRawAtom;
    preflight
        .attachments
        .merge(&payload.attachment_set())
        .map_err(|error| format!("could not merge {label} attachments: {error}"))
}

fn inspect_value_attachments(
    value: &Value,
    preflight: &mut AstPayloadPreflight,
) -> Result<(), String> {
    match value {
        Value::Bytes(bytes) => {
            inspect_payload_attachments(bytes, "embedded Atom payload", preflight)
        }
        Value::Map(map) => inspect_node_attachments(map, preflight),
        _ => Ok(()),
    }
}

fn inspect_values_attachments(
    values: &[Value],
    preflight: &mut AstPayloadPreflight,
) -> Result<(), String> {
    for value in values {
        inspect_value_attachments(value, preflight)?;
    }
    Ok(())
}

fn inspect_arg_attachment(
    args: &[Value],
    index: usize,
    head: &str,
    preflight: &mut AstPayloadPreflight,
) -> Result<(), String> {
    let value = args
        .get(index)
        .ok_or_else(|| format!("{head} missing argument {index}"))?;
    inspect_value_attachments(value, preflight)
}

fn inspect_slot_or_arg_attachment(
    slots: &[(Value, Value)],
    key: &str,
    args: &[Value],
    index: usize,
    head: &str,
    preflight: &mut AstPayloadPreflight,
) -> Result<(), String> {
    if let Some(value) = map_get(slots, key) {
        inspect_value_attachments(value, preflight)
    } else {
        inspect_arg_attachment(args, index, head, preflight)
    }
}

fn inspect_semantic_metadata_attachments(
    args: &[Value],
    slots: &[(Value, Value)],
    preflight: &mut AstPayloadPreflight,
) -> Result<(), String> {
    let Some(Value::Map(payload)) = map_get(slots, "value") else {
        return inspect_arg_attachment(args, 0, "semantic-metadata", preflight);
    };
    if map_get(payload, "protocol") != Some(&Value::Text("tymbolica".to_owned())) {
        return inspect_arg_attachment(args, 0, "semantic-metadata", preflight);
    }
    if map_text(payload, "kind")? != "atom" {
        return inspect_arg_attachment(args, 0, "semantic-metadata", preflight);
    }
    if value_i64(
        map_get(payload, "version")
            .ok_or_else(|| "tymbolica metadata missing version".to_owned())?,
        "tymbolica metadata version",
    )? != 1
    {
        return Err("unsupported tymbolica metadata version".to_owned());
    }
    match map_get(payload, "atom") {
        Some(Value::Bytes(bytes)) => {
            inspect_payload_attachments(bytes, "tymbolica metadata Atom", preflight)
        }
        Some(other) => Err(format!(
            "tymbolica metadata atom must be bytes, got {other:?}"
        )),
        None => Err("tymbolica metadata missing atom".to_owned()),
    }
}

fn inspect_node_attachments(
    map: &[(Value, Value)],
    preflight: &mut AstPayloadPreflight,
) -> Result<(), String> {
    let head = map_text(map, "head")?;
    let args = map_array(map, "args")?;
    let slots = map_map(map, "slots")?;
    match head {
        "add" | "arg" | "mul" | "times" | "dot" => inspect_values_attachments(args, preflight),
        "sub" => {
            inspect_arg_attachment(args, 0, head, preflight)?;
            inspect_arg_attachment(args, 1, head, preflight)
        }
        "neg" | "factorial" => inspect_arg_attachment(args, 0, head, preflight),
        "plus" | "group" | "()" => {
            inspect_slot_or_arg_attachment(slots, "expr", args, 0, head, preflight)
        }
        "lr" => inspect_slot_or_arg_attachment(slots, "body", args, 0, head, preflight),
        "attach" => {
            inspect_slot_or_arg_attachment(slots, "base", args, 0, head, preflight)?;
            if let Some(value) = map_get(slots, "t") {
                inspect_value_attachments(value, preflight)?;
            }
            Ok(())
        }
        "semantic-metadata" => inspect_semantic_metadata_attachments(args, slots, preflight),
        "frac" => {
            inspect_slot_or_arg_attachment(slots, "num", args, 0, head, preflight)?;
            inspect_slot_or_arg_attachment(slots, "denom", args, 1, head, preflight)
        }
        "pow" => {
            inspect_slot_or_arg_attachment(slots, "base", args, 0, head, preflight)?;
            inspect_slot_or_arg_attachment(slots, "exp", args, 1, head, preflight)
        }
        "sqrt" => {
            if let Some(value) = map_get(slots, "radicand").or_else(|| map_get(slots, "body")) {
                inspect_value_attachments(value, preflight)
            } else {
                inspect_arg_attachment(args, 0, head, preflight)
            }
        }
        "root" => {
            if let Some(value) = map_get(slots, "radicand").or_else(|| map_get(slots, "body")) {
                inspect_value_attachments(value, preflight)?;
            } else if args.len() > 1 {
                inspect_arg_attachment(args, 1, head, preflight)?;
            } else {
                inspect_arg_attachment(args, 0, head, preflight)?;
            }
            if let Some(value) = map_get(slots, "index").or_else(|| args.first()) {
                inspect_value_attachments(value, preflight)?;
            }
            Ok(())
        }
        "abs" | "norm" => {
            if let Some(value) = map_get(slots, "body") {
                inspect_value_attachments(value, preflight)
            } else {
                inspect_arg_attachment(args, 0, head, preflight)
            }
        }
        "call" => {
            let function = map_get(slots, "fn").ok_or_else(|| "missing fn".to_owned())?;
            if !matches!(function, Value::Text(_))
                && !matches!(function, Value::Map(map) if map_text(map, "head").ok() == Some("op"))
            {
                inspect_value_attachments(function, preflight)?;
            }
            if let Some(body) = map_get(slots, "body") {
                if let Value::Map(map) = body
                    && map_text(map, "head").ok() == Some("arg")
                {
                    inspect_values_attachments(map_array(map, "args")?, preflight)?;
                } else {
                    inspect_value_attachments(body, preflight)?;
                }
            }
            Ok(())
        }
        "op-call" => {
            let op = map_get(slots, "op").ok_or_else(|| "missing op".to_owned())?;
            if !matches!(op, Value::Text(_))
                && !matches!(op, Value::Map(map) if map_text(map, "head").ok() == Some("op"))
            {
                inspect_value_attachments(op, preflight)?;
            }
            if let Some(values) = map_get(slots, "args") {
                if let Value::Map(map) = values
                    && map_text(map, "head").ok() == Some("arg")
                {
                    inspect_values_attachments(map_array(map, "args")?, preflight)?;
                } else {
                    inspect_value_attachments(values, preflight)?;
                }
            }
            Ok(())
        }
        "mat" | "vec" => Err(format!(
            "{head} is matrix-valued; use matrix(...) or vec(...)"
        )),
        _ => inspect_values_attachments(args, preflight),
    }
}

/// Construct a namespaced Symbolica symbol using the shared registry.
pub fn symbol_atom(name: &str, namespace: &str) -> Result<Atom, String> {
    tymbolica_symbol_registry::initialize();
    Symbol::parse(name.trim(), namespace.to_owned()).map(Atom::var)
}

fn atom_from_leaf(text: &str, namespace: &str) -> Result<Atom, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("empty math leaf".to_owned());
    }

    if let Ok(number) = text.parse::<i64>() {
        return Ok(Atom::num(number));
    }

    if (text.contains('.') || text.contains('e') || text.contains('E'))
        && let Ok(number) = text.parse::<f64>()
        && number.is_finite()
    {
        return Ok(Atom::num(number));
    }

    symbol_atom(text, namespace)
}

fn atom_from_node(
    map: &[(Value, Value)],
    namespace: &str,
    attachments: &mut AttachmentSet,
) -> Result<Atom, String> {
    let head = map_text(map, "head")?;
    let args = map_array(map, "args")?;
    let slots = map_map(map, "slots")?;

    match head {
        "add" => Ok(Atom::add_many(atoms_from_values(
            args,
            namespace,
            attachments,
        )?)),
        "arg" => Ok(Symbol::ARG.call_args(atoms_from_values(args, namespace, attachments)?)),
        "mul" | "times" | "dot" => Ok(Atom::mul_many(atoms_from_values(
            args,
            namespace,
            attachments,
        )?)),
        "sub" => Ok(atom_arg(args, 0, head, namespace, attachments)?
            - atom_arg(args, 1, head, namespace, attachments)?),
        "neg" => Ok(-atom_arg(args, 0, head, namespace, attachments)?),
        "plus" | "group" | "()" => {
            slot_or_arg_atom(slots, "expr", args, 0, head, namespace, attachments)
        }
        "lr" => slot_or_arg_atom(slots, "body", args, 0, head, namespace, attachments),
        "attach" => {
            let base = slot_or_arg_atom(slots, "base", args, 0, head, namespace, attachments)?;
            if let Some(exponent) = slot_atom(slots, "t", namespace, attachments) {
                Ok(base.pow(exponent?))
            } else {
                Ok(base)
            }
        }
        "factorial" => {
            let argument = atom_arg(args, 0, head, namespace, attachments)?;
            Ok(Symbol::parse("gamma", "symbolica")?.call(argument + Atom::num(1)))
        }
        "semantic-metadata" => atom_from_semantic_metadata(args, slots, namespace, attachments),
        "frac" => {
            let numerator = slot_or_arg_atom(slots, "num", args, 0, head, namespace, attachments)?;
            let denominator =
                slot_or_arg_atom(slots, "denom", args, 1, head, namespace, attachments)?;
            Ok(numerator / denominator)
        }
        "pow" => {
            let base = slot_or_arg_atom(slots, "base", args, 0, head, namespace, attachments)?;
            let exponent = slot_or_arg_atom(slots, "exp", args, 1, head, namespace, attachments)?;
            Ok(base.pow(exponent))
        }
        "sqrt" => {
            let radicand = slot_atom(slots, "radicand", namespace, attachments)
                .or_else(|| slot_atom(slots, "body", namespace, attachments))
                .unwrap_or_else(|| atom_arg(args, 0, head, namespace, attachments))?;
            Ok(Symbol::SQRT.call(radicand))
        }
        "root" => {
            let radicand = slot_atom(slots, "radicand", namespace, attachments)
                .or_else(|| slot_atom(slots, "body", namespace, attachments))
                .unwrap_or_else(|| {
                    atom_arg(args, 1, head, namespace, attachments)
                        .or_else(|_| atom_arg(args, 0, head, namespace, attachments))
                })?;
            if let Some(index) = slot_atom(slots, "index", namespace, attachments).or_else(|| {
                args.first()
                    .map(|value| atom_from_value_collect(value, namespace, attachments))
            }) {
                Ok(radicand.pow(Atom::num(1) / index?))
            } else {
                Ok(Symbol::SQRT.call(radicand))
            }
        }
        "abs" | "norm" => {
            let body = slot_atom(slots, "body", namespace, attachments)
                .unwrap_or_else(|| atom_arg(args, 0, head, namespace, attachments))?;
            Ok(Symbol::ABS.call(body))
        }
        "call" => {
            let symbol = slot_symbol(slots, "fn", namespace, attachments)?;
            let arguments = slot_atoms(slots, "body", namespace, attachments)?.unwrap_or_default();
            Ok(symbol.call_args(arguments))
        }
        "op-call" => {
            let symbol = slot_symbol(slots, "op", namespace, attachments)?;
            let arguments = slot_atoms(slots, "args", namespace, attachments)?.unwrap_or_default();
            Ok(symbol.call_args(arguments))
        }
        "mat" | "vec" => Err(format!(
            "{head} is matrix-valued; use matrix(...) or vec(...)"
        )),
        _ => {
            let symbol = Symbol::parse(head, namespace.to_owned())?;
            Ok(symbol.call_args(atoms_from_values(args, namespace, attachments)?))
        }
    }
}

fn atom_from_semantic_metadata(
    args: &[Value],
    slots: &[(Value, Value)],
    namespace: &str,
    attachments: &mut AttachmentSet,
) -> Result<Atom, String> {
    let visible = |attachments: &mut AttachmentSet| {
        atom_arg(args, 0, "semantic-metadata", namespace, attachments)
    };
    let Some(Value::Map(payload)) = map_get(slots, "value") else {
        return visible(attachments);
    };

    if map_get(payload, "protocol") != Some(&Value::Text("tymbolica".to_owned())) {
        return visible(attachments);
    }

    match map_text(payload, "kind")? {
        "atom" => {
            if value_i64(
                map_get(payload, "version")
                    .ok_or_else(|| "tymbolica metadata missing version".to_owned())?,
                "tymbolica metadata version",
            )? != 1
            {
                return Err("unsupported tymbolica metadata version".to_owned());
            }
            match map_get(payload, "atom") {
                Some(Value::Bytes(bytes)) => {
                    decode_embedded_atom(bytes, "tymbolica metadata Atom", attachments)
                }
                Some(other) => Err(format!(
                    "tymbolica metadata atom must be bytes, got {other:?}"
                )),
                None => Err("tymbolica metadata missing atom".to_owned()),
            }
        }
        _ => visible(attachments),
    }
}

fn atoms_from_values(
    values: &[Value],
    namespace: &str,
    attachments: &mut AttachmentSet,
) -> Result<Vec<Atom>, String> {
    values
        .iter()
        .map(|value| atom_from_value_collect(value, namespace, attachments))
        .collect()
}

fn atoms_from_arg_value(
    value: &Value,
    namespace: &str,
    attachments: &mut AttachmentSet,
) -> Result<Vec<Atom>, String> {
    if let Value::Map(map) = value
        && map_text(map, "head").ok() == Some("arg")
    {
        return atoms_from_values(map_array(map, "args")?, namespace, attachments);
    }

    Ok(vec![atom_from_value_collect(
        value,
        namespace,
        attachments,
    )?])
}

fn slot_atoms(
    slots: &[(Value, Value)],
    key: &str,
    namespace: &str,
    attachments: &mut AttachmentSet,
) -> Result<Option<Vec<Atom>>, String> {
    map_get(slots, key)
        .map(|value| atoms_from_arg_value(value, namespace, attachments))
        .transpose()
}

fn symbol_from_value(
    value: &Value,
    namespace: &str,
    attachments: &mut AttachmentSet,
) -> Result<Symbol, String> {
    match value {
        Value::Text(text) => Symbol::parse(text.trim(), namespace.to_owned()),
        Value::Map(map) if map_text(map, "head").ok() == Some("op") => {
            if let Some(Value::Text(text)) = map_get(map, "text") {
                Symbol::parse(text.trim(), namespace.to_owned())
            } else if let Some(Value::Text(text)) = map_get(map_map(map, "slots")?, "text") {
                Symbol::parse(text.trim(), namespace.to_owned())
            } else {
                Err("op missing text".to_owned())
            }
        }
        other => atom_from_value_collect(other, namespace, attachments).and_then(|atom| {
            atom.get_symbol()
                .ok_or_else(|| "function head must be a symbol".to_owned())
        }),
    }
}

fn slot_symbol(
    slots: &[(Value, Value)],
    key: &str,
    namespace: &str,
    attachments: &mut AttachmentSet,
) -> Result<Symbol, String> {
    map_get(slots, key)
        .ok_or_else(|| format!("missing {key}"))
        .and_then(|value| symbol_from_value(value, namespace, attachments))
}

fn atom_arg(
    args: &[Value],
    index: usize,
    head: &str,
    namespace: &str,
    attachments: &mut AttachmentSet,
) -> Result<Atom, String> {
    args.get(index)
        .ok_or_else(|| format!("{head} missing argument {index}"))
        .and_then(|value| atom_from_value_collect(value, namespace, attachments))
}

fn slot_or_arg_atom(
    slots: &[(Value, Value)],
    key: &str,
    args: &[Value],
    index: usize,
    head: &str,
    namespace: &str,
    attachments: &mut AttachmentSet,
) -> Result<Atom, String> {
    slot_atom(slots, key, namespace, attachments)
        .unwrap_or_else(|| atom_arg(args, index, head, namespace, attachments))
}

fn slot_atom(
    slots: &[(Value, Value)],
    key: &str,
    namespace: &str,
    attachments: &mut AttachmentSet,
) -> Option<Result<Atom, String>> {
    map_get(slots, key).map(|value| atom_from_value_collect(value, namespace, attachments))
}

fn map_get<'a>(map: &'a [(Value, Value)], key: &str) -> Option<&'a Value> {
    map.iter().find_map(|(candidate, value)| match candidate {
        Value::Text(candidate) if candidate == key => Some(value),
        _ => None,
    })
}

fn map_text<'a>(map: &'a [(Value, Value)], key: &str) -> Result<&'a str, String> {
    match map_get(map, key) {
        Some(Value::Text(text)) => Ok(text),
        Some(_) => Err(format!("{key} must be text")),
        None => Err(format!("missing {key}")),
    }
}

fn map_array<'a>(map: &'a [(Value, Value)], key: &str) -> Result<&'a [Value], String> {
    match map_get(map, key) {
        Some(Value::Array(values)) => Ok(values),
        Some(_) => Err(format!("{key} must be an array")),
        None => Err(format!("missing {key}")),
    }
}

fn map_map<'a>(map: &'a [(Value, Value)], key: &str) -> Result<&'a [(Value, Value)], String> {
    match map_get(map, key) {
        Some(Value::Map(values)) => Ok(values),
        Some(_) => Err(format!("{key} must be a dictionary")),
        None => Err(format!("missing {key}")),
    }
}

fn value_i64(value: &Value, label: &str) -> Result<i64, String> {
    match value {
        Value::Integer(number) => (*number)
            .try_into()
            .map_err(|_| format!("{label} is out of range")),
        other => Err(format!("{label} must be an integer, got {other:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tymbolica_atom_payload::{
        Attachment, AttachmentKey, SYMBOLICA_REVISION, encode_atom, encode_atom_from_set,
    };

    fn node(head: &str, args: Vec<Value>, slots: Vec<(Value, Value)>) -> Value {
        Value::Map(vec![
            (Value::Text("head".to_owned()), Value::Text(head.to_owned())),
            (Value::Text("args".to_owned()), Value::Array(args)),
            (Value::Text("slots".to_owned()), Value::Map(slots)),
        ])
    }

    fn attached_bytes(atom: &Atom, key: &AttachmentKey, data: &[u8]) -> Vec<u8> {
        let attachments =
            AttachmentSet::from_attachments([Attachment::new(key.clone(), data.to_vec()).unwrap()])
                .unwrap();
        encode_atom_from_set(atom, &attachments).unwrap()
    }

    #[test]
    fn exact_atom_metadata_overrides_its_visible_math() {
        let exact = symbol_atom("momentum", "kinematics").unwrap();
        let payload = Value::Map(vec![
            (
                Value::Text("protocol".to_owned()),
                Value::Text("tymbolica".to_owned()),
            ),
            (Value::Text("version".to_owned()), Value::Integer(1.into())),
            (
                Value::Text("kind".to_owned()),
                Value::Text("atom".to_owned()),
            ),
            (
                Value::Text("atom".to_owned()),
                Value::Bytes(encode_atom(&exact).unwrap()),
            ),
            (
                Value::Text("semantic".to_owned()),
                Value::Map(vec![(
                    Value::Text("kind".to_owned()),
                    Value::Text("symbol".to_owned()),
                )]),
            ),
        ]);
        let ast = node(
            "semantic-metadata",
            vec![Value::Text("visible".to_owned())],
            vec![(Value::Text("value".to_owned()), payload)],
        );

        assert_eq!(atom_from_value(&ast, "fallback").unwrap(), exact);
    }

    #[test]
    fn unrelated_metadata_is_transparent() {
        let ast = node(
            "semantic-metadata",
            vec![Value::Text("x".to_owned())],
            vec![(
                Value::Text("value".to_owned()),
                Value::Map(vec![(
                    Value::Text("source".to_owned()),
                    Value::Text("another-package".to_owned()),
                )]),
            )],
        );

        assert_eq!(
            atom_from_value(&ast, "fallback").unwrap(),
            symbol_atom("x", "fallback").unwrap(),
        );
    }

    #[test]
    fn other_tymbolica_metadata_kinds_are_transparent() {
        let ast = node(
            "semantic-metadata",
            vec![Value::Text("x".to_owned())],
            vec![(
                Value::Text("value".to_owned()),
                Value::Map(vec![
                    (
                        Value::Text("protocol".to_owned()),
                        Value::Text("tymbolica".to_owned()),
                    ),
                    (
                        Value::Text("kind".to_owned()),
                        Value::Text("layout-hint".to_owned()),
                    ),
                ]),
            )],
        );

        assert_eq!(
            atom_from_value(&ast, "fallback").unwrap(),
            symbol_atom("x", "fallback").unwrap(),
        );
    }

    #[test]
    fn recognized_atom_envelopes_fail_closed() {
        let envelope = |atom: Option<Value>, version: i64| {
            let mut fields = vec![
                (
                    Value::Text("protocol".to_owned()),
                    Value::Text("tymbolica".to_owned()),
                ),
                (
                    Value::Text("version".to_owned()),
                    Value::Integer(version.into()),
                ),
                (
                    Value::Text("kind".to_owned()),
                    Value::Text("atom".to_owned()),
                ),
            ];
            if let Some(atom) = atom {
                fields.push((Value::Text("atom".to_owned()), atom));
            }
            node(
                "semantic-metadata",
                vec![Value::Text("misleading".to_owned())],
                vec![(Value::Text("value".to_owned()), Value::Map(fields))],
            )
        };

        assert!(atom_from_value(&envelope(None, 1), "fallback").is_err());
        assert!(
            atom_from_value(
                &envelope(Some(Value::Text("not bytes".to_owned())), 1),
                "fallback",
            )
            .is_err()
        );
        assert!(atom_from_value(&envelope(Some(Value::Bytes(vec![0])), 2), "fallback").is_err());
    }

    #[test]
    fn raw_atom_bytes_can_be_function_arguments() {
        let x = symbol_atom("x", "embedded").unwrap();
        let ast = node(
            "call",
            vec![],
            vec![
                (Value::Text("fn".to_owned()), Value::Text("f".to_owned())),
                (
                    Value::Text("body".to_owned()),
                    node("arg", vec![Value::Bytes(encode_atom(&x).unwrap())], vec![]),
                ),
            ],
        );
        let f = Symbol::parse("f", "default").unwrap();
        assert_eq!(atom_from_value(&ast, "default").unwrap(), f.call(x));
    }

    #[test]
    fn recursive_atom_payloads_merge_attachments_before_import() {
        let first_key = AttachmentKey::new("org.tymbolica.test", 1, b"first".to_vec()).unwrap();
        let second_key = AttachmentKey::new("org.tymbolica.test", 1, b"second".to_vec()).unwrap();
        let x = symbol_atom("x", "embedded").unwrap();
        let y = symbol_atom("y", "embedded").unwrap();
        let ast = node(
            "add",
            vec![
                Value::Bytes(attached_bytes(&x, &first_key, b"one")),
                node(
                    "call",
                    vec![],
                    vec![
                        (Value::Text("fn".to_owned()), Value::Text("f".to_owned())),
                        (
                            Value::Text("body".to_owned()),
                            node(
                                "arg",
                                vec![Value::Bytes(attached_bytes(&y, &second_key, b"two"))],
                                vec![],
                            ),
                        ),
                    ],
                ),
            ],
            vec![],
        );

        let preflight = attachments_from_value(&ast).unwrap();
        assert_eq!(preflight.get(&first_key), Some(b"one".as_slice()));
        assert_eq!(preflight.get(&second_key), Some(b"two".as_slice()));

        let parsed = attached_atom_from_value(&ast, "default").unwrap();
        assert_eq!(parsed.attachments, preflight);
        assert_eq!(
            parsed.atom,
            x + Symbol::parse("f", "default").unwrap().call(y)
        );
    }

    #[test]
    fn recursive_attachment_conflicts_fail_closed() {
        let key = AttachmentKey::new("org.tymbolica.test", 1, b"same".to_vec()).unwrap();
        let x = symbol_atom("x", "embedded").unwrap();
        let ast = node(
            "add",
            vec![
                Value::Bytes(attached_bytes(&x, &key, b"one")),
                Value::Bytes(attached_bytes(&x, &key, b"two")),
            ],
            vec![],
        );

        assert!(
            attachments_from_value(&ast)
                .unwrap_err()
                .contains("conflicting data")
        );
        assert!(attached_atom_from_value(&ast, "default").is_err());
    }

    #[test]
    fn preflight_rejects_revision_mismatches_without_importing() {
        let x = symbol_atom("x", "embedded").unwrap();
        let mut payload = encode_atom(&x).unwrap();
        let revision = SYMBOLICA_REVISION.as_bytes();
        let offset = payload
            .windows(revision.len())
            .position(|window| window == revision)
            .unwrap();
        payload[offset] = if payload[offset] == b'0' { b'1' } else { b'0' };

        let error = attachments_from_value(&Value::Bytes(payload)).unwrap_err();
        assert!(error.contains("uses Symbolica revision"));
    }

    #[test]
    fn rich_preflight_reports_legacy_payloads_without_breaking_attachment_wrappers() {
        let x = symbol_atom("x", "embedded").unwrap();
        let current = encode_atom(&x).unwrap();
        let legacy = parse_payload(&current).unwrap().atom_bytes().to_vec();
        let ast = node(
            "add",
            vec![Value::Bytes(current), Value::Bytes(legacy)],
            vec![],
        );

        let preflight = preflight_payloads_from_value(&ast).unwrap();
        assert!(preflight.has_legacy_payload);
        assert!(preflight.attachments.is_empty());
        assert!(attachments_from_value(&ast).unwrap().is_empty());
    }
}
