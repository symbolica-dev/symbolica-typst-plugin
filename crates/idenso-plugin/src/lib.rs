//! Tydenso tensor construction, printing, inspection, and Idenso transforms.

use std::io::Cursor;

use ciborium::value::Value;
use idenso::color::ColorSimplifier;
use idenso::dirac::GammaSimplifier;
use idenso::selective_expand::SelectiveExpand;
use idenso::shorthands::{metric::MetricSimplifier, schoonschip::Schoonschip};
use idenso::{Cookable, IndexTooling};
use spenso::network::tags::{SPENSO_TAG, prepare_tensor_print, register_tensor_symbol};
use spenso::shadowing::symbolica_utils::SpensoPrintSettings;
use spenso::structure::abstract_index::AbstractIndex;
use spenso::structure::representation::{
    IndexDisplay, IndexPalette, IndexRow, LibraryRep, RepName,
};
use symbolica::atom::{
    Atom, AtomCore, AtomView, DefaultNamespace, NamespacedSymbol, Symbol, SymbolAttribute,
    SymbolBuilder,
};
use symbolica::printer::PrintOptions;
use tymbolica_atom_payload::{
    AttachmentSet, ParsedPayload, PayloadFormat, encode_atom_from_set, parse_payload,
};
use tymbolica_symbol_registry::{
    PortableRepresentationClass, REPRESENTATION_ATTACHMENT_SCHEMA, RepresentationDeclaration,
    RepresentationDeclarations, canonical_representation_name,
};
use wasm_minimal_protocol::*;

#[cfg(test)]
use tymbolica_atom_payload::{Attachment, AttachmentKey};
#[cfg(test)]
use tymbolica_symbol_registry::REPRESENTATION_ATTACHMENT_VERSION;

initiate_protocol!();

const DISPLAY_INDEX_VERSION: i64 = 1;
const MAX_DISPLAY_INDEX_AST_BYTES: usize = 64 * 1024;
const MAX_DISPLAY_INDEX_DEPTH: usize = 16;
const MAX_DISPLAY_INDEX_NODES: usize = 64;

fn legacy_payload_error(label: &str) -> String {
    format!(
        "{label} uses legacy raw Atom bytes, which cannot carry portable representation declarations; recreate the payload with aligned Tymbolica/Tydenso package versions"
    )
}

getrandom_02::register_custom_getrandom!(tymbolica_getrandom_v02);

fn tymbolica_getrandom_v02(destination: &mut [u8]) -> Result<(), getrandom_02::Error> {
    let mut state = 0x517c_c1b7_2722_0a95u64 ^ destination.len() as u64;
    for byte in destination {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        *byte = (state >> 56) as u8;
    }
    Ok(())
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
#[unsafe(no_mangle)]
unsafe extern "Rust" fn __getrandom_v03_custom(
    dest: *mut u8,
    len: usize,
) -> Result<(), getrandom::Error> {
    if dest.is_null() && len != 0 {
        return Err(getrandom::Error::new_custom(1));
    }
    let mut state = 0x9e37_79b9_7f4a_7c15u64 ^ len as u64;
    for index in 0..len {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        unsafe { dest.add(index).write((state >> 56) as u8) };
    }
    Ok(())
}

#[derive(Clone, Debug, Default)]
struct InputContext {
    /// Raw input attachments enforce the common fail-closed key invariant even
    /// when a known schema has two semantically equivalent CBOR encodings.
    all_input: AttachmentSet,
    /// Attachments not interpreted by this plugin. Known representation
    /// attachments are regenerated canonically and only for output references.
    passthrough: AttachmentSet,
    representations: RepresentationDeclarations,
}

impl InputContext {
    fn merge_representation(
        &mut self,
        name: String,
        declaration: RepresentationDeclaration,
    ) -> Result<(), String> {
        self.representations
            .insert(name, declaration)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    fn inspect_payload<'a>(
        &mut self,
        input: &'a [u8],
        label: &str,
    ) -> Result<ParsedPayload<'a>, String> {
        let parsed = parse_payload(input)
            .map_err(|error| format!("{label} must be Atom payload bytes: {error}"))?;
        if parsed.format() == PayloadFormat::LegacyRawAtom {
            return Err(legacy_payload_error(label));
        }
        parsed
            .ensure_import_compatible()
            .map_err(|error| format!("{label} is incompatible: {error}"))?;

        let incoming = parsed.attachment_set();
        self.absorb_attachment_set(&incoming)?;
        Ok(parsed)
    }

    fn absorb_attachment_set(&mut self, incoming: &AttachmentSet) -> Result<(), String> {
        // Mutate a candidate so a malformed known declaration cannot leave a
        // partially absorbed multi-input context behind.
        let mut candidate = self.clone();
        candidate
            .all_input
            .merge(incoming)
            .map_err(|error| format!("could not merge Atom attachments: {error}"))?;
        candidate
            .representations
            .absorb_attachments(incoming)
            .map_err(|error| error.to_string())?;

        for attachment in incoming.iter() {
            if attachment.schema() != REPRESENTATION_ATTACHMENT_SCHEMA {
                candidate
                    .passthrough
                    .insert(attachment.to_owned_attachment())
                    .map_err(|error| format!("could not merge Atom attachments: {error}"))?;
            }
        }
        *self = candidate;
        Ok(())
    }

    /// Register every declaration only after every input has been inspected and
    /// merged. This must precede `Atom::import`, otherwise Symbolica can intern a
    /// representation head without its local callback.
    fn register_representations(&self) -> Result<(), String> {
        self.representations
            .register_before_atom_import()
            .map_err(|error| error.to_string())
    }
}

fn import_payload(parsed: &ParsedPayload<'_>, label: &str) -> Result<Atom, String> {
    parsed
        .import_atom()
        .map_err(|error| format!("{label} must be Atom payload bytes: {error}"))
}

fn decode_atom_with_context(input: &[u8], label: &str) -> Result<(Atom, InputContext), String> {
    let mut context = InputContext::default();
    let parsed = context.inspect_payload(input, label)?;
    context.register_representations()?;
    let atom = import_payload(&parsed, label)?;
    Ok((atom, context))
}

fn decode_atom(input: &[u8], label: &str) -> Result<Atom, String> {
    decode_atom_with_context(input, label).map(|(atom, _)| atom)
}

fn decode_cbor(input: &[u8], label: &str) -> Result<Value, String> {
    ciborium::from_reader::<Value, _>(Cursor::new(input))
        .map_err(|error| format!("{label} must be CBOR-encoded: {error}"))
}

fn encode_cbor(value: Value) -> Result<Vec<u8>, String> {
    let mut output = Vec::new();
    ciborium::into_writer(&value, &mut output)
        .map_err(|error| format!("could not encode Tydenso data: {error}"))?;
    Ok(output)
}

#[cfg(test)]
fn decode_cbor_exact(input: &[u8], label: &str) -> Result<Value, String> {
    let mut cursor = Cursor::new(input);
    let value = ciborium::from_reader::<Value, _>(&mut cursor)
        .map_err(|error| format!("{label} must be CBOR-encoded: {error}"))?;
    if cursor.position() != input.len() as u64 {
        return Err(format!("{label} has trailing bytes"));
    }
    Ok(value)
}

fn encode_atom_with_context(atom: &Atom, context: &InputContext) -> Result<Vec<u8>, String> {
    let mut attachments = context.passthrough.clone();
    RepresentationDeclarations::referenced_by_atom(atom)
        .and_then(|declarations| declarations.append_attachments_to(&mut attachments))
        .map_err(|error| error.to_string())?;
    encode_atom_from_set(atom, &attachments)
        .map_err(|error| format!("could not encode Tydenso result: {error}"))
}

#[cfg(test)]
fn encode_atom(atom: &Atom) -> Result<Vec<u8>, String> {
    encode_atom_with_context(atom, &InputContext::default())
}

fn map_get<'a>(map: &'a [(Value, Value)], key: &str) -> Option<&'a Value> {
    map.iter().find_map(|(candidate, value)| match candidate {
        Value::Text(candidate) if candidate == key => Some(value),
        _ => None,
    })
}

fn map_text<'a>(map: &'a [(Value, Value)], key: &str) -> Result<&'a str, String> {
    match map_get(map, key) {
        Some(Value::Text(value)) => Ok(value),
        Some(other) => Err(format!("{key} must be text, got {other:?}")),
        None => Err(format!("missing {key}")),
    }
}

fn map_text_or<'a>(
    map: &'a [(Value, Value)],
    key: &str,
    default: &'a str,
) -> Result<&'a str, String> {
    match map_get(map, key) {
        Some(Value::Text(value)) => Ok(value),
        Some(other) => Err(format!("{key} must be text, got {other:?}")),
        None => Ok(default),
    }
}

fn map_array<'a>(map: &'a [(Value, Value)], key: &str) -> Result<&'a [Value], String> {
    match map_get(map, key) {
        Some(Value::Array(value)) => Ok(value),
        Some(other) => Err(format!("{key} must be an array, got {other:?}")),
        None => Err(format!("missing {key}")),
    }
}

fn map_bool(map: &[(Value, Value)], key: &str, default: bool) -> Result<bool, String> {
    match map_get(map, key) {
        Some(Value::Bool(value)) => Ok(*value),
        Some(other) => Err(format!("{key} must be bool, got {other:?}")),
        None => Ok(default),
    }
}

fn value_map<'a>(value: &'a Value, label: &str) -> Result<&'a [(Value, Value)], String> {
    match value {
        Value::Map(map) => Ok(map),
        other => Err(format!("{label} must be a dictionary, got {other:?}")),
    }
}

fn text_key(key: &str) -> Value {
    Value::Text(key.to_owned())
}

fn cbor_map(entries: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(
        entries
            .into_iter()
            .map(|(key, value)| (text_key(key), value))
            .collect(),
    )
}

fn value_i64(value: &Value, label: &str) -> Result<i64, String> {
    match value {
        Value::Integer(value) => (*value)
            .try_into()
            .map_err(|_| format!("{label} is out of range")),
        other => Err(format!("{label} must be an integer, got {other:?}")),
    }
}

fn display_ast_child<'a>(
    slots: &'a [(Value, Value)],
    key: &str,
    args: &'a [Value],
    index: usize,
    head: &str,
) -> Result<&'a Value, String> {
    map_get(slots, key)
        .or_else(|| args.get(index))
        .ok_or_else(|| format!("manual index {head} missing {key}"))
}

fn index_display_sequence<'a>(
    values: impl IntoIterator<Item = &'a Value>,
    depth: usize,
    nodes: &mut usize,
) -> Result<IndexDisplay, String> {
    let displays = values
        .into_iter()
        .filter(|value| !matches!(value, Value::Text(text) if text.trim().is_empty()))
        .map(|value| index_display_from_ast(value, depth + 1, nodes))
        .collect::<Result<Vec<_>, _>>()?;

    match displays.len() {
        0 => Err("manual index display cannot be empty".to_owned()),
        1 => Ok(displays.into_iter().next().unwrap()),
        _ => Ok(IndexDisplay::Sequence(displays)),
    }
}

fn index_display_from_ast(
    value: &Value,
    depth: usize,
    nodes: &mut usize,
) -> Result<IndexDisplay, String> {
    if depth > MAX_DISPLAY_INDEX_DEPTH {
        return Err(format!(
            "manual index display exceeds the maximum depth of {MAX_DISPLAY_INDEX_DEPTH}"
        ));
    }
    *nodes += 1;
    if *nodes > MAX_DISPLAY_INDEX_NODES {
        return Err(format!(
            "manual index display exceeds the maximum size of {MAX_DISPLAY_INDEX_NODES} nodes"
        ));
    }

    match value {
        Value::Integer(value) => {
            let value: i64 = (*value)
                .try_into()
                .map_err(|_| "manual index integer is out of range".to_owned())?;
            Ok(IndexDisplay::Number(value))
        }
        Value::Text(text) => {
            let text = text.trim();
            if let Ok(number) = text.parse::<i64>() {
                return Ok(IndexDisplay::Number(number));
            }
            IndexDisplay::symbol(text).map_err(|error| error.to_string())
        }
        Value::Array(values) => index_display_sequence(values, depth, nodes),
        Value::Map(map) => {
            let head = map_text(map, "head")?;
            let args = map_array(map, "args")?;
            let slots = value_map(
                map_get(map, "slots")
                    .ok_or_else(|| format!("manual index {head} missing slots"))?,
                "manual index slots",
            )?;

            match head {
                "attach" => {
                    for unsupported in ["tl", "tr", "bl", "br"] {
                        if map_get(slots, unsupported).is_some() {
                            return Err(format!(
                                "manual index attachment {unsupported} is not supported"
                            ));
                        }
                    }

                    let base = index_display_from_ast(
                        display_ast_child(slots, "base", args, 0, head)?,
                        depth + 1,
                        nodes,
                    )?;
                    let top = map_get(slots, "t")
                        .filter(|value| !matches!(value, Value::Null))
                        .map(|value| index_display_from_ast(value, depth + 1, nodes))
                        .transpose()?;
                    let bottom = map_get(slots, "b")
                        .filter(|value| !matches!(value, Value::Null))
                        .map(|value| index_display_from_ast(value, depth + 1, nodes))
                        .transpose()?;
                    if top.is_none() && bottom.is_none() {
                        Ok(base)
                    } else {
                        Ok(IndexDisplay::Attach {
                            base: Box::new(base),
                            top: top.map(Box::new),
                            bottom: bottom.map(Box::new),
                        })
                    }
                }
                "()" | "group" | "plus" => index_display_from_ast(
                    display_ast_child(slots, "expr", args, 0, head)?,
                    depth + 1,
                    nodes,
                ),
                "lr" => index_display_from_ast(
                    display_ast_child(slots, "body", args, 0, head)?,
                    depth + 1,
                    nodes,
                ),
                "sequence" | "mul" => index_display_sequence(args, depth, nodes),
                "semantic-metadata" => index_display_from_ast(
                    args.first().ok_or_else(|| {
                        "manual index metadata missing visible content".to_owned()
                    })?,
                    depth + 1,
                    nodes,
                ),
                other => Err(format!(
                    "unsupported manual index display node {other:?}; use symbols, groups, sequences, or attachments"
                )),
            }
        }
        other => Err(format!(
            "unsupported manual index display value {other:?}; use Typst math content"
        )),
    }
}

fn index_ast_from_bytes(input: &[u8]) -> Result<Value, String> {
    if input.len() > MAX_DISPLAY_INDEX_AST_BYTES {
        return Err(format!(
            "manual index AST exceeds the {MAX_DISPLAY_INDEX_AST_BYTES}-byte limit"
        ));
    }
    decode_cbor(input, "manual index AST")
}

fn index_display_from_bytes(input: &[u8]) -> Result<IndexDisplay, String> {
    let ast = index_ast_from_bytes(input)?;
    let mut nodes = 0;
    index_display_from_ast(&ast, 0, &mut nodes)
}

/// Recover an exact Atom carried by Tymbolica semantic metadata at the root of
/// an index expression. This preserves symbol namespaces and identity without
/// evaluating any Typst source. Ordinary handwritten math has no such envelope
/// and continues through the restricted `IndexDisplay` parser.
fn semantic_atom_payload_bytes(value: &Value) -> Result<Option<&[u8]>, String> {
    let Value::Map(node) = value else {
        return Ok(None);
    };
    if map_text(node, "head").ok() != Some("semantic-metadata") {
        return Ok(None);
    }
    let Some(Value::Map(slots)) = map_get(node, "slots") else {
        return Ok(None);
    };
    let Some(Value::Map(payload)) = map_get(slots, "value") else {
        return Ok(None);
    };
    if map_get(payload, "protocol") != Some(&Value::Text("tymbolica".to_owned())) {
        return Ok(None);
    }
    if map_text(payload, "kind")? != "atom" {
        return Ok(None);
    }
    let version = value_i64(
        map_get(payload, "version")
            .ok_or_else(|| "tymbolica index metadata missing version".to_owned())?,
        "tymbolica index metadata version",
    )?;
    if version != 1 {
        return Err(format!(
            "unsupported tymbolica index metadata version {version}"
        ));
    }
    match map_get(payload, "atom") {
        Some(Value::Bytes(bytes)) => Ok(Some(bytes)),
        Some(other) => Err(format!(
            "tymbolica index metadata atom must be bytes, got {other:?}"
        )),
        None => Err("tymbolica index metadata missing atom".to_owned()),
    }
}

fn exact_atom_from_index_ast(value: &Value) -> Result<Option<Atom>, String> {
    semantic_atom_payload_bytes(value)?
        .map(|bytes| decode_atom(bytes, "tymbolica index metadata"))
        .transpose()
}

fn hash_index_display(display: &IndexDisplay, hasher: &mut blake3::Hasher) {
    match display {
        IndexDisplay::Symbol(name) => {
            hasher.update(&[0]);
            hasher.update(&(name.len() as u64).to_le_bytes());
            hasher.update(name.as_bytes());
        }
        IndexDisplay::Number(number) => {
            hasher.update(&[1]);
            hasher.update(&number.to_le_bytes());
        }
        IndexDisplay::Sequence(items) => {
            hasher.update(&[2]);
            hasher.update(&(items.len() as u64).to_le_bytes());
            for item in items {
                hash_index_display(item, hasher);
            }
        }
        IndexDisplay::Attach { base, top, bottom } => {
            hasher.update(&[3]);
            hash_index_display(base, hasher);
            match top {
                Some(top) => {
                    hasher.update(&[1]);
                    hash_index_display(top, hasher);
                }
                None => {
                    hasher.update(&[0]);
                }
            }
            match bottom {
                Some(bottom) => {
                    hasher.update(&[1]);
                    hash_index_display(bottom, hasher);
                }
                None => {
                    hasher.update(&[0]);
                }
            }
        }
    }
}

fn display_index_symbol_name(display: &IndexDisplay) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"tydenso-display-index-v1\0");
    hash_index_display(display, &mut hasher);
    format!("tydenso_index_{}", hasher.finalize().to_hex())
}

fn register_display_index(display: &IndexDisplay, namespace: &str) -> Result<Symbol, String> {
    let user_data = display.symbol_user_data();
    let namespace = DefaultNamespace {
        namespace: namespace.to_owned().into(),
        data: "",
        file: "".into(),
        line: 0,
    };
    let namespaced = namespace.attach_namespace(&display_index_symbol_name(display));

    if let Some(existing) = Symbol::get_symbol(namespaced.clone()) {
        if existing.has_tag(&SPENSO_TAG.index) && existing.get_data() == &user_data {
            return Ok(existing);
        }
        return Err(format!(
            "symbol {} already exists with different manual-index metadata",
            existing.get_name()
        ));
    }

    SymbolBuilder::new(namespaced)
        .with_tags([SPENSO_TAG.index.clone()])
        .with_user_data(user_data)
        .build()
        .map_err(|error| error.to_string())
}

fn display_index_atom(map: &[(Value, Value)], namespace: &str) -> Result<Atom, String> {
    let version = value_i64(
        map_get(map, "version").ok_or_else(|| "display index missing version".to_owned())?,
        "display index version",
    )?;
    if version != DISPLAY_INDEX_VERSION {
        return Err(format!("unsupported display index version {version}"));
    }
    let ast = match map_get(map, "ast") {
        Some(Value::Bytes(ast)) => ast,
        Some(other) => return Err(format!("display index AST must be bytes, got {other:?}")),
        None => return Err("display index missing AST".to_owned()),
    };
    let ast_value = index_ast_from_bytes(ast)?;
    if let Some(atom) = exact_atom_from_index_ast(&ast_value)? {
        return Ok(atom);
    }
    let mut nodes = 0;
    let display = index_display_from_ast(&ast_value, 0, &mut nodes)?;
    Ok(Atom::var(register_display_index(&display, namespace)?))
}

fn palette_index_display(value: &Value) -> Result<IndexDisplay, String> {
    match value {
        Value::Integer(value) => {
            let number: i64 = (*value)
                .try_into()
                .map_err(|_| "index palette integer is out of range".to_owned())?;
            Ok(IndexDisplay::Number(number))
        }
        Value::Text(text) => IndexDisplay::symbol(text.trim()).map_err(|error| error.to_string()),
        Value::Map(map) if map_text(map, "kind")? == "display-index" => {
            let version = value_i64(
                map_get(map, "version")
                    .ok_or_else(|| "display index missing version".to_owned())?,
                "display index version",
            )?;
            if version != DISPLAY_INDEX_VERSION {
                return Err(format!("unsupported display index version {version}"));
            }
            let ast = match map_get(map, "ast") {
                Some(Value::Bytes(ast)) => ast,
                Some(other) => {
                    return Err(format!("display index AST must be bytes, got {other:?}"));
                }
                None => return Err("display index missing AST".to_owned()),
            };
            index_display_from_bytes(ast)
        }
        other => Err(format!(
            "index palette entries must be Typst math, text, or integers, got {other:?}"
        )),
    }
}

fn representation_index_palette(map: &[(Value, Value)]) -> Result<IndexPalette, String> {
    let Some(indices) = map_get(map, "indices") else {
        return Ok(IndexPalette::Numeric);
    };
    let indices = match indices {
        Value::Null => return Ok(IndexPalette::Numeric),
        Value::Array(indices) => indices,
        other => {
            return Err(format!(
                "representation indices must be an array, got {other:?}"
            ));
        }
    };
    let start = match map_get(map, "index-start") {
        None => 1,
        Some(value) => {
            let start = value_i64(value, "representation index-start")?;
            usize::try_from(start)
                .map_err(|_| "representation index-start must be non-negative".to_owned())?
        }
    };
    IndexPalette::cyclic(
        start,
        indices
            .iter()
            .map(palette_index_display)
            .collect::<Result<Vec<_>, _>>()?,
    )
    .map_err(|error| error.to_string())
}

fn validate_canonical_dual_name(map: &[(Value, Value)], name: &str) -> Result<(), String> {
    match map_get(map, "dual-name") {
        None | Some(Value::Null) => Ok(()),
        Some(Value::Text(dual_name)) if dual_name == name => Ok(()),
        Some(Value::Text(dual_name)) => Err(format!(
            "dual-name {dual_name:?} cannot name a different representation symbol; duality is stored as slot variance"
        )),
        Some(other) => Err(format!("dual-name must be text or null, got {other:?}")),
    }
}

fn default_index_row(name: &str) -> IndexRow {
    if name == "spenso::bis" {
        IndexRow::Bottom
    } else {
        IndexRow::Top
    }
}

fn representation_index_row(map: &[(Value, Value)], name: &str) -> Result<IndexRow, String> {
    match map_get(map, "index-row") {
        None => Ok(default_index_row(name)),
        Some(Value::Text(value)) if value == "top" => Ok(IndexRow::Top),
        Some(Value::Text(value)) if value == "bottom" => Ok(IndexRow::Bottom),
        Some(Value::Text(value)) => Err(format!(
            "representation index-row must be \"top\" or \"bottom\", got {value:?}"
        )),
        Some(other) => Err(format!(
            "representation index-row must be text, got {other:?}"
        )),
    }
}

fn representation_declaration_from_map(
    map: &[(Value, Value)],
) -> Result<(String, RepresentationDeclaration), String> {
    let name = map_text(map, "name")?;
    let namespace = map_text_or(map, "namespace", "spenso")?;
    validate_canonical_dual_name(map, name)?;
    let qualified_name =
        canonical_representation_name(name, namespace).map_err(|error| error.to_string())?;
    let self_dual = map_bool(map, "self-dual", false)?;
    let declaration = RepresentationDeclaration {
        class: if self_dual && qualified_name == "spenso::mink" {
            // Typst's public constructor models variance, so the locally known
            // Minkowski inline metric is presented as self-dual there.
            PortableRepresentationClass::InlineMetric
        } else if self_dual {
            PortableRepresentationClass::SelfDual
        } else {
            PortableRepresentationClass::Dualizable
        },
        index_palette: representation_index_palette(map)?,
        index_row: representation_index_row(map, &qualified_name)?,
    };
    Ok((qualified_name, declaration))
}

fn parse_symbol(
    name: &str,
    namespace: &str,
    map: Option<&[(Value, Value)]>,
) -> Result<Symbol, String> {
    let Some(map) = map else {
        return Symbol::parse(name, namespace.to_owned());
    };

    let symmetric = map_bool(map, "symmetric", false)?;
    let antisymmetric = map_bool(map, "antisymmetric", false)?;
    let cyclesymmetric = map_bool(map, "cycle-symmetric", false)?;
    let linear = map_bool(map, "linear", false)?;
    if [symmetric, antisymmetric, cyclesymmetric]
        .into_iter()
        .filter(|value| *value)
        .count()
        > 1
    {
        return Err(
            "a tensor cannot be symmetric, antisymmetric, and cycle-symmetric at the same time"
                .to_owned(),
        );
    }

    let mut attributes = Vec::new();
    if symmetric {
        attributes.push(SymbolAttribute::Symmetric);
    }
    if antisymmetric {
        attributes.push(SymbolAttribute::Antisymmetric);
    }
    if cyclesymmetric {
        attributes.push(SymbolAttribute::Cyclesymmetric);
    }
    if linear {
        attributes.push(SymbolAttribute::Linear);
    }
    if attributes.is_empty() {
        return Symbol::parse(name, namespace.to_owned());
    }

    let namespace = DefaultNamespace {
        namespace: namespace.to_owned().into(),
        data: "",
        file: "".into(),
        line: 0,
    };
    SymbolBuilder::new(namespace.attach_namespace(name))
        .with_attributes(attributes)
        .build()
        .map_err(|error| error.to_string())
}

fn parse_tensor_symbol(
    name: &str,
    namespace: &str,
    map: &[(Value, Value)],
    rank_one: bool,
) -> Result<Symbol, String> {
    let symmetric = map_bool(map, "symmetric", false)?;
    let antisymmetric = map_bool(map, "antisymmetric", false)?;
    let cyclesymmetric = map_bool(map, "cycle-symmetric", false)?;
    let linear = map_bool(map, "linear", false)?;
    if [symmetric, antisymmetric, cyclesymmetric]
        .into_iter()
        .filter(|value| *value)
        .count()
        > 1
    {
        return Err(
            "a tensor cannot be symmetric, antisymmetric, and cycle-symmetric at the same time"
                .to_owned(),
        );
    }

    let mut attributes = Vec::new();
    if symmetric {
        attributes.push(SymbolAttribute::Symmetric);
    }
    if antisymmetric {
        attributes.push(SymbolAttribute::Antisymmetric);
    }
    if cyclesymmetric {
        attributes.push(SymbolAttribute::Cyclesymmetric);
    }
    if linear {
        attributes.push(SymbolAttribute::Linear);
    }

    let namespace = DefaultNamespace {
        namespace: namespace.to_owned().into(),
        data: "",
        file: "".into(),
        line: 0,
    };
    register_tensor_symbol(namespace.attach_namespace(name), attributes, rank_one)
}

fn parse_representation(
    name: &str,
    namespace: &str,
    map: &[(Value, Value)],
) -> Result<LibraryRep, String> {
    let (qualified_name, declaration) = representation_declaration_from_map(map)?;
    let expected_name =
        canonical_representation_name(name, namespace).map_err(|error| error.to_string())?;
    debug_assert_eq!(qualified_name, expected_name);
    let symbol = Symbol::get_symbol(NamespacedSymbol::parse(&qualified_name)).ok_or_else(|| {
        format!("representation {qualified_name} was not registered during input preparation")
    })?;
    let representation = LibraryRep::try_from_symbol_coerced(symbol)
        .map_err(|error| format!("representation {qualified_name} is not registered: {error}"))?;
    if representation.metadata().is_none_or(|metadata| {
        PortableRepresentationClass::from(metadata.class) != declaration.class
            || metadata.index_palette != declaration.index_palette
            || metadata.index_row != declaration.index_row
    }) {
        return Err(format!(
            "symbol {qualified_name} already exists with a different representation declaration"
        ));
    }
    Ok(representation)
}

fn representation_atom(map: &[(Value, Value)], index: Option<&Value>) -> Result<Atom, String> {
    let name = map_text(map, "name")?;
    let namespace = map_text_or(map, "namespace", "spenso")?;
    let dimension = map_get(map, "dimension").ok_or_else(|| "missing dimension".to_owned())?;
    let representation = parse_representation(name, namespace, map)?;
    let symbol = representation.symbol();
    let mut arguments = vec![atom_from_value_prepared(dimension, namespace)?];
    if let Some(index) = index {
        arguments.push(atom_from_value_prepared(index, namespace)?);
    }
    let representation = symbol.call_args(arguments);
    if index.is_none() && map_bool(map, "is-dual", false)? {
        Ok(parse_symbol("dind", "spenso", None)?.call(representation))
    } else {
        Ok(representation)
    }
}

fn slot_atom(map: &[(Value, Value)]) -> Result<Atom, String> {
    let representation = value_map(
        map_get(map, "representation").ok_or_else(|| "missing representation".to_owned())?,
        "representation",
    )?;
    let index = map_get(map, "index").ok_or_else(|| "missing index".to_owned())?;
    let slot = representation_atom(representation, Some(index))?;
    if map_bool(map, "dual", false)? {
        Ok(parse_symbol("dind", "spenso", None)?.call(slot))
    } else {
        Ok(slot)
    }
}

fn collect_construct_value(
    value: &Value,
    namespace: &str,
    context: &mut InputContext,
) -> Result<(), String> {
    match value {
        Value::Bytes(bytes) => {
            context.inspect_payload(bytes, "embedded expression")?;
            Ok(())
        }
        Value::Integer(_) | Value::Float(_) | Value::Text(_) => Ok(()),
        Value::Map(map) => match map_text(map, "kind")? {
            "display-index" => {
                let version = value_i64(
                    map_get(map, "version")
                        .ok_or_else(|| "display index missing version".to_owned())?,
                    "display index version",
                )?;
                if version != DISPLAY_INDEX_VERSION {
                    return Err(format!("unsupported display index version {version}"));
                }
                let ast = match map_get(map, "ast") {
                    Some(Value::Bytes(ast)) => ast,
                    Some(other) => {
                        return Err(format!("display index AST must be bytes, got {other:?}"));
                    }
                    None => return Err("display index missing AST".to_owned()),
                };
                let ast = index_ast_from_bytes(ast)?;
                if let Some(bytes) = semantic_atom_payload_bytes(&ast)? {
                    context.inspect_payload(bytes, "tymbolica index metadata")?;
                } else {
                    let mut nodes = 0;
                    index_display_from_ast(&ast, 0, &mut nodes)?;
                }
                Ok(())
            }
            "symbol" => Ok(()),
            "call" | "tensor" | "vector" => {
                let child_namespace = map_text_or(map, "namespace", namespace)?;
                for argument in map_array(map, "arguments")? {
                    collect_construct_value(argument, child_namespace, context)?;
                }
                Ok(())
            }
            "representation" => {
                let (name, declaration) = representation_declaration_from_map(map)?;
                context.merge_representation(name, declaration)?;
                let child_namespace = map_text_or(map, "namespace", namespace)?;
                collect_construct_value(
                    map_get(map, "dimension").ok_or_else(|| "missing dimension".to_owned())?,
                    child_namespace,
                    context,
                )
            }
            "slot" => {
                let representation = map_get(map, "representation")
                    .ok_or_else(|| "missing representation".to_owned())?;
                collect_construct_value(representation, namespace, context)?;
                collect_construct_value(
                    map_get(map, "index").ok_or_else(|| "missing index".to_owned())?,
                    namespace,
                    context,
                )
            }
            "sum" => {
                for term in map_array(map, "terms")? {
                    collect_construct_value(term, namespace, context)?;
                }
                Ok(())
            }
            "product" => {
                for factor in map_array(map, "factors")? {
                    collect_construct_value(factor, namespace, context)?;
                }
                Ok(())
            }
            "negative" => collect_construct_value(
                map_get(map, "expression").ok_or_else(|| "missing expression".to_owned())?,
                namespace,
                context,
            ),
            "power" => {
                collect_construct_value(
                    map_get(map, "base").ok_or_else(|| "missing base".to_owned())?,
                    namespace,
                    context,
                )?;
                collect_construct_value(
                    map_get(map, "exponent").ok_or_else(|| "missing exponent".to_owned())?,
                    namespace,
                    context,
                )
            }
            kind => Err(format!("unsupported Tydenso value kind {kind:?}")),
        },
        other => Err(format!("unsupported Tydenso value: {other:?}")),
    }
}

fn atom_from_value_with_context(
    value: &Value,
    namespace: &str,
) -> Result<(Atom, InputContext), String> {
    let mut context = InputContext::default();
    collect_construct_value(value, namespace, &mut context)?;
    context.register_representations()?;
    let atom = atom_from_value_prepared(value, namespace)?;
    Ok((atom, context))
}

fn atom_from_value(value: &Value, namespace: &str) -> Result<Atom, String> {
    atom_from_value_with_context(value, namespace).map(|(atom, _)| atom)
}

fn atom_from_value_prepared(value: &Value, namespace: &str) -> Result<Atom, String> {
    match value {
        Value::Bytes(bytes) => {
            let parsed = parse_payload(bytes)
                .map_err(|error| format!("expression must be Atom payload bytes: {error}"))?;
            import_payload(&parsed, "expression")
        }
        Value::Integer(value) => {
            let value: i64 = (*value)
                .try_into()
                .map_err(|_| "integer is out of range".to_owned())?;
            Ok(Atom::num(value))
        }
        Value::Float(value) => Ok(Atom::num(*value)),
        Value::Text(value) => Ok(Atom::var(parse_symbol(value, namespace, None)?)),
        Value::Map(map) => match map_text(map, "kind")? {
            "display-index" => display_index_atom(map, namespace),
            "symbol" => {
                let symbol_namespace = map_text_or(map, "namespace", namespace)?;
                Ok(Atom::var(parse_symbol(
                    map_text(map, "name")?,
                    symbol_namespace,
                    Some(map),
                )?))
            }
            "call" => {
                let symbol_namespace = map_text_or(map, "namespace", namespace)?;
                let symbol = parse_symbol(map_text(map, "name")?, symbol_namespace, Some(map))?;
                let arguments = map_array(map, "arguments")?
                    .iter()
                    .map(|argument| atom_from_value_prepared(argument, symbol_namespace))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(symbol.call_args(arguments))
            }
            "tensor" | "vector" => {
                let symbol_namespace = map_text_or(map, "namespace", namespace)?;
                let symbol = parse_tensor_symbol(
                    map_text(map, "name")?,
                    symbol_namespace,
                    map,
                    map_text(map, "kind")? == "vector",
                )?;
                let arguments = map_array(map, "arguments")?
                    .iter()
                    .map(|argument| atom_from_value_prepared(argument, symbol_namespace))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(symbol.call_args(arguments))
            }
            "representation" => representation_atom(map, None),
            "slot" => slot_atom(map),
            "sum" => Ok(Atom::add_many(
                map_array(map, "terms")?
                    .iter()
                    .map(|term| atom_from_value_prepared(term, namespace))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            "product" => Ok(Atom::mul_many(
                map_array(map, "factors")?
                    .iter()
                    .map(|factor| atom_from_value_prepared(factor, namespace))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            "negative" => Ok(-atom_from_value_prepared(
                map_get(map, "expression").ok_or_else(|| "missing expression".to_owned())?,
                namespace,
            )?),
            "power" => Ok(atom_from_value_prepared(
                map_get(map, "base").ok_or_else(|| "missing base".to_owned())?,
                namespace,
            )?
            .pow(atom_from_value_prepared(
                map_get(map, "exponent").ok_or_else(|| "missing exponent".to_owned())?,
                namespace,
            )?)),
            kind => Err(format!("unsupported Tydenso value kind {kind:?}")),
        },
        other => Err(format!("unsupported Tydenso value: {other:?}")),
    }
}

fn atom_tree(view: AtomView<'_>) -> Value {
    match view {
        AtomView::Num(_) => cbor_map([
            ("kind", Value::Text("number".to_owned())),
            ("value", Value::Text(view.to_plain_string())),
        ]),
        AtomView::Var(variable) => {
            let symbol = variable.get_symbol();
            cbor_map([
                ("kind", Value::Text("symbol".to_owned())),
                ("name", Value::Text(symbol.get_name().to_owned())),
                (
                    "short-name",
                    Value::Text(symbol.get_stripped_name().to_owned()),
                ),
            ])
        }
        AtomView::Fun(function) => {
            let symbol = function.get_symbol();
            cbor_map([
                ("kind", Value::Text("function".to_owned())),
                ("name", Value::Text(symbol.get_name().to_owned())),
                (
                    "short-name",
                    Value::Text(symbol.get_stripped_name().to_owned()),
                ),
                (
                    "arguments",
                    Value::Array(function.iter().map(atom_tree).collect()),
                ),
                ("symmetric", Value::Bool(function.is_symmetric())),
                ("antisymmetric", Value::Bool(function.is_antisymmetric())),
                ("cycle-symmetric", Value::Bool(function.is_cyclesymmetric())),
                ("linear", Value::Bool(function.is_linear())),
            ])
        }
        AtomView::Pow(power) => {
            let (base, exponent) = power.get_base_exp();
            cbor_map([
                ("kind", Value::Text("power".to_owned())),
                ("base", atom_tree(base)),
                ("exponent", atom_tree(exponent)),
            ])
        }
        AtomView::Mul(product) => cbor_map([
            ("kind", Value::Text("product".to_owned())),
            (
                "factors",
                Value::Array(product.iter().map(atom_tree).collect()),
            ),
        ]),
        AtomView::Add(sum) => cbor_map([
            ("kind", Value::Text("sum".to_owned())),
            ("terms", Value::Array(sum.iter().map(atom_tree).collect())),
        ]),
    }
}

fn print_settings(
    value: Option<&Value>,
    default_preset: &str,
) -> Result<SpensoPrintSettings, String> {
    let Some(value) = value else {
        return Ok(match default_preset {
            "typst" => SpensoPrintSettings::typst(),
            "compact" => SpensoPrintSettings::compact(),
            _ => unreachable!(),
        });
    };
    let map = value_map(value, "settings")?;
    let preset = map_text_or(map, "preset", default_preset)?;
    let mut settings = match preset {
        "typst" => SpensoPrintSettings::typst(),
        "compact" => SpensoPrintSettings::compact(),
        other => return Err(format!("unknown print preset {other:?}")),
    };
    settings.with_dim = map_bool(map, "with-dim", settings.with_dim)?;
    settings.parens = map_bool(map, "parens", settings.parens)?;
    settings.commas = map_bool(map, "commas", settings.commas)?;
    settings.index_subscripts = map_bool(map, "index-subscripts", settings.index_subscripts)?;
    settings.symbol_scripts = map_bool(map, "symbol-scripts", settings.symbol_scripts)?;
    Ok(settings)
}

fn render_request(input: &[u8], typst: bool) -> Result<Vec<u8>, String> {
    let value = decode_cbor(input, "request")?;
    let map = value_map(&value, "request")?;
    let expr = match map_get(map, "expr") {
        Some(Value::Bytes(bytes)) => decode_atom(bytes, "expr")?,
        Some(other) => atom_from_value(other, "spenso")?,
        None => return Err("missing expr".to_owned()),
    };
    let settings = print_settings(
        map_get(map, "settings"),
        if typst { "typst" } else { "compact" },
    )?;
    let options = if typst {
        PrintOptions {
            custom_print_mode: (&settings).into(),
            ..PrintOptions::typst()
        }
    } else {
        let mut options = settings.nice_symbolica();
        options.color_builtin_symbols = false;
        options.color_top_level_sum = false;
        options.terms_on_new_line = false;
        options
    };
    let printable = if typst {
        prepare_tensor_print(&expr)
    } else {
        expr
    };
    Ok(printable.printer(options).to_string().into_bytes())
}

fn symbol_from_atom(atom: &Atom, label: &str) -> Result<Symbol, String> {
    match atom.as_view() {
        AtomView::Var(variable) => Ok(variable.get_symbol()),
        _ => Err(format!("{label} must be a symbol")),
    }
}

fn expanded(terms: Vec<(Atom, Atom)>) -> Atom {
    terms
        .into_iter()
        .fold(Atom::Zero, |sum, (coefficient, tensor)| {
            sum + coefficient * tensor
        })
}

fn encode_atom_array(atoms: Vec<Atom>, context: &InputContext) -> Result<Vec<u8>, String> {
    let values = atoms
        .iter()
        .map(|atom| encode_atom_with_context(atom, context).map(Value::Bytes))
        .collect::<Result<Vec<_>, _>>()?;
    let mut output = Vec::new();
    ciborium::into_writer(&Value::Array(values), &mut output)
        .map_err(|error| format!("could not encode Tydenso result array: {error}"))?;
    Ok(output)
}

#[wasm_func]
pub fn construct(value: &[u8]) -> Result<Vec<u8>, String> {
    let value = decode_cbor(value, "value")?;
    let (atom, context) = atom_from_value_with_context(&value, "spenso")?;
    encode_atom_with_context(&atom, &context)
}

#[wasm_func]
pub fn from_ast(ast: &[u8], namespace: &[u8]) -> Result<Vec<u8>, String> {
    let namespace = match decode_cbor(namespace, "namespace")? {
        Value::Text(namespace) => namespace,
        other => return Err(format!("namespace must be text, got {other:?}")),
    };
    let mut context = InputContext::default();
    let preflight = tymbolica_typst_ast::preflight_payloads_from_ast(ast, "ast")?;
    if preflight.has_legacy_payload {
        return Err(legacy_payload_error("ast"));
    }
    context.absorb_attachment_set(&preflight.attachments)?;
    context.register_representations()?;
    let attached = tymbolica_typst_ast::attached_atom_from_ast(ast, &namespace, "ast")?;
    debug_assert_eq!(attached.attachments, preflight.attachments);
    encode_atom_with_context(&attached.atom, &context)
}

#[wasm_func]
pub fn inspect(expr: &[u8]) -> Result<Vec<u8>, String> {
    encode_cbor(atom_tree(decode_atom(expr, "expr")?.as_view()))
}

#[wasm_func]
pub fn to_typst(request: &[u8]) -> Result<Vec<u8>, String> {
    render_request(request, true)
}

#[wasm_func]
pub fn to_string(request: &[u8]) -> Result<Vec<u8>, String> {
    render_request(request, false)
}

#[wasm_func]
pub fn cook_function(expr: &[u8]) -> Result<Vec<u8>, String> {
    let (atom, context) = decode_atom_with_context(expr, "expr")?;
    let result = atom.cook_function().map_err(|error| format!("{error:?}"))?;
    encode_atom_with_context(&result, &context)
}

#[wasm_func]
pub fn cook_indices(expr: &[u8]) -> Result<Vec<u8>, String> {
    let (atom, context) = decode_atom_with_context(expr, "expr")?;
    encode_atom_with_context(&atom.cook_indices(), &context)
}

#[wasm_func]
pub fn dirac_adjoint(expr: &[u8]) -> Result<Vec<u8>, String> {
    let (atom, context) = decode_atom_with_context(expr, "expr")?;
    let result = atom
        .dirac_adjoint::<AbstractIndex>()
        .map_err(|error| error.to_string())?;
    encode_atom_with_context(&result, &context)
}

#[wasm_func]
pub fn expand_bis(expr: &[u8]) -> Result<Vec<u8>, String> {
    let (atom, context) = decode_atom_with_context(expr, "expr")?;
    encode_atom_with_context(&expanded(atom.expand_bis()), &context)
}

#[wasm_func]
pub fn expand_color(expr: &[u8]) -> Result<Vec<u8>, String> {
    let (atom, context) = decode_atom_with_context(expr, "expr")?;
    encode_atom_with_context(&expanded(atom.expand_color()), &context)
}

#[wasm_func]
pub fn expand_metrics(expr: &[u8]) -> Result<Vec<u8>, String> {
    let (atom, context) = decode_atom_with_context(expr, "expr")?;
    encode_atom_with_context(&expanded(atom.expand_metrics()), &context)
}

#[wasm_func]
pub fn expand_mink(expr: &[u8]) -> Result<Vec<u8>, String> {
    let (atom, context) = decode_atom_with_context(expr, "expr")?;
    encode_atom_with_context(&expanded(atom.expand_mink()), &context)
}

#[wasm_func]
pub fn expand_mink_bis(expr: &[u8]) -> Result<Vec<u8>, String> {
    let (atom, context) = decode_atom_with_context(expr, "expr")?;
    encode_atom_with_context(&expanded(atom.expand_mink_bis()), &context)
}

#[wasm_func]
pub fn list_dangling(expr: &[u8]) -> Result<Vec<u8>, String> {
    let (atom, context) = decode_atom_with_context(expr, "expr")?;
    encode_atom_array(atom.list_dangling::<AbstractIndex>(), &context)
}

#[wasm_func]
pub fn simplify_color(expr: &[u8]) -> Result<Vec<u8>, String> {
    let (atom, context) = decode_atom_with_context(expr, "expr")?;
    encode_atom_with_context(&atom.simplify_color(), &context)
}

#[wasm_func]
pub fn simplify_gamma(expr: &[u8]) -> Result<Vec<u8>, String> {
    let (atom, context) = decode_atom_with_context(expr, "expr")?;
    encode_atom_with_context(&atom.simplify_gamma(), &context)
}

#[wasm_func]
pub fn simplify_metrics(expr: &[u8]) -> Result<Vec<u8>, String> {
    let (atom, context) = decode_atom_with_context(expr, "expr")?;
    encode_atom_with_context(&atom.simplify_metrics(), &context)
}

#[wasm_func]
pub fn to_dots(expr: &[u8]) -> Result<Vec<u8>, String> {
    let (atom, context) = decode_atom_with_context(expr, "expr")?;
    encode_atom_with_context(&atom.to_dots(), &context)
}

#[wasm_func]
pub fn wrap_dummies(expr: &[u8], header: &[u8]) -> Result<Vec<u8>, String> {
    let mut context = InputContext::default();
    let expr_payload = context.inspect_payload(expr, "expr")?;
    let header_payload = context.inspect_payload(header, "header")?;
    context.register_representations()?;
    let expr = import_payload(&expr_payload, "expr")?;
    let header_atom = import_payload(&header_payload, "header")?;
    let header = symbol_from_atom(&header_atom, "header")?;
    encode_atom_with_context(&expr.wrap_dummies::<AbstractIndex>(header), &context)
}

#[wasm_func]
pub fn wrap_indices(expr: &[u8], header: &[u8]) -> Result<Vec<u8>, String> {
    let mut context = InputContext::default();
    let expr_payload = context.inspect_payload(expr, "expr")?;
    let header_payload = context.inspect_payload(header, "header")?;
    context.register_representations()?;
    let expr = import_payload(&expr_payload, "expr")?;
    let header_atom = import_payload(&header_payload, "header")?;
    let header = symbol_from_atom(&header_atom, "header")?;
    encode_atom_with_context(&expr.wrap_indices(header), &context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tymbolica_symbol_registry::{
        decode_representation_declaration, encode_representation_declaration,
    };

    fn ast_node(head: &str, args: Vec<Value>, slots: Vec<(&str, Value)>) -> Value {
        Value::Map(vec![
            (Value::Text("head".to_owned()), Value::Text(head.to_owned())),
            (Value::Text("args".to_owned()), Value::Array(args)),
            (
                Value::Text("slots".to_owned()),
                Value::Map(
                    slots
                        .into_iter()
                        .map(|(key, value)| (Value::Text(key.to_owned()), value))
                        .collect(),
                ),
            ),
        ])
    }

    fn value_bytes(value: &Value) -> Vec<u8> {
        let mut output = Vec::new();
        ciborium::into_writer(value, &mut output).unwrap();
        output
    }

    fn display_index_value(version: i64, ast: &Value) -> Value {
        Value::Map(vec![
            (
                Value::Text("kind".to_owned()),
                Value::Text("display-index".to_owned()),
            ),
            (
                Value::Text("version".to_owned()),
                Value::Integer(version.into()),
            ),
            (
                Value::Text("ast".to_owned()),
                Value::Bytes(value_bytes(ast)),
            ),
        ])
    }

    fn mu_one_ast() -> Value {
        ast_node(
            "attach",
            vec![Value::Text("mu".to_owned())],
            vec![("b", Value::Text("1".to_owned()))],
        )
    }

    fn variable_symbol(atom: &Atom) -> Symbol {
        let AtomView::Var(variable) = atom.as_view() else {
            panic!("expected a variable");
        };
        variable.get_symbol()
    }

    fn representation_value_with_row(name: &str, indices: Vec<Value>, index_row: &str) -> Value {
        cbor_map([
            ("kind", Value::Text("representation".to_owned())),
            ("name", Value::Text(name.to_owned())),
            (
                "namespace",
                Value::Text("tydenso_palette_registration_test".to_owned()),
            ),
            ("dimension", Value::Integer(4.into())),
            ("self-dual", Value::Bool(true)),
            ("indices", Value::Array(indices)),
            ("index-start", Value::Integer(1.into())),
            ("index-row", Value::Text(index_row.to_owned())),
        ])
    }

    fn representation_value(name: &str, indices: Vec<Value>) -> Value {
        representation_value_with_row(name, indices, "top")
    }

    fn dualizable_representation_value(name: &str, namespace: &str) -> Value {
        cbor_map([
            ("kind", Value::Text("representation".to_owned())),
            ("name", Value::Text(name.to_owned())),
            ("namespace", Value::Text(namespace.to_owned())),
            ("dimension", Value::Integer(4.into())),
            ("self-dual", Value::Bool(false)),
            ("is-dual", Value::Bool(false)),
            ("dual-name", Value::Text(name.to_owned())),
            ("indices", Value::Null),
            ("index-start", Value::Integer(1.into())),
            ("index-row", Value::Text("top".to_owned())),
        ])
    }

    fn slot_value(representation: Value, index: &str, dual: bool) -> Value {
        cbor_map([
            ("kind", Value::Text("slot".to_owned())),
            ("representation", representation),
            ("index", Value::Text(index.to_owned())),
            ("dual", Value::Bool(dual)),
        ])
    }

    fn tensor_value(name: &str, namespace: &str, arguments: Vec<Value>) -> Value {
        cbor_map([
            ("kind", Value::Text("tensor".to_owned())),
            ("name", Value::Text(name.to_owned())),
            ("namespace", Value::Text(namespace.to_owned())),
            ("arguments", Value::Array(arguments)),
            ("symmetric", Value::Bool(false)),
            ("antisymmetric", Value::Bool(false)),
            ("cycle-symmetric", Value::Bool(false)),
            ("linear", Value::Bool(false)),
        ])
    }

    fn representation_attachment(
        name: &str,
        declaration: &RepresentationDeclaration,
    ) -> Attachment {
        tymbolica_symbol_registry::representation_attachment(name, declaration).unwrap()
    }

    fn unknown_attachment(identity: &[u8], data: &[u8]) -> (AttachmentKey, Attachment) {
        let key = AttachmentKey::new("tydenso.test.unknown", 7, identity.to_vec()).unwrap();
        let attachment = Attachment::new(key.clone(), data.to_vec()).unwrap();
        (key, attachment)
    }

    fn add_attachment(payload: &[u8], attachment: Attachment) -> Vec<u8> {
        let parsed = parse_payload(payload).unwrap();
        let mut attachments = parsed.attachment_set();
        attachments.insert(attachment).unwrap();
        encode_atom_from_set(&parsed.import_atom().unwrap(), &attachments).unwrap()
    }

    fn semantic_atom_payload_ast(payload: Vec<u8>) -> Value {
        ast_node(
            "semantic-metadata",
            vec![Value::Text("visible".to_owned())],
            vec![(
                "value",
                cbor_map([
                    ("protocol", Value::Text("tymbolica".to_owned())),
                    ("version", Value::Integer(1.into())),
                    ("kind", Value::Text("atom".to_owned())),
                    ("atom", Value::Bytes(payload)),
                ]),
            )],
        )
    }

    fn semantic_atom_ast(atom: &Atom) -> Value {
        semantic_atom_payload_ast(encode_atom(atom).unwrap())
    }

    #[test]
    fn parses_manual_bottom_attachment_without_losing_it() {
        let display = index_display_from_bytes(&value_bytes(&mu_one_ast())).unwrap();

        assert_eq!(
            display,
            IndexDisplay::Attach {
                base: Box::new(IndexDisplay::symbol("mu").unwrap()),
                top: None,
                bottom: Some(Box::new(IndexDisplay::Number(1))),
            }
        );
        assert_eq!(display.to_typst_source(), "attach(mu,b:1)");
    }

    #[test]
    fn parses_groups_and_sequences() {
        let grouped = ast_node(
            "()",
            vec![],
            vec![(
                "expr",
                Value::Array(vec![
                    Value::Text("a".to_owned()),
                    ast_node(
                        "attach",
                        vec![Value::Text("b".to_owned())],
                        vec![("t", Value::Text("2".to_owned()))],
                    ),
                ]),
            )],
        );

        assert_eq!(
            index_display_from_bytes(&value_bytes(&grouped)).unwrap(),
            IndexDisplay::Sequence(vec![
                IndexDisplay::symbol("a").unwrap(),
                IndexDisplay::Attach {
                    base: Box::new(IndexDisplay::symbol("b").unwrap()),
                    top: Some(Box::new(IndexDisplay::Number(2))),
                    bottom: None,
                },
            ])
        );
    }

    #[test]
    fn rejects_unsupported_manual_index_nodes_and_versions() {
        let call = ast_node("call", vec![Value::Text("f".to_owned())], vec![]);
        assert!(
            index_display_from_bytes(&value_bytes(&call))
                .unwrap_err()
                .contains("unsupported manual index display node")
        );

        assert!(
            atom_from_value(
                &display_index_value(DISPLAY_INDEX_VERSION + 1, &mu_one_ast()),
                "tydenso_manual_index_version_test",
            )
            .unwrap_err()
            .contains("unsupported display index version")
        );
    }

    #[test]
    fn display_indices_are_deterministic_and_survive_atom_export() {
        let value = display_index_value(DISPLAY_INDEX_VERSION, &mu_one_ast());
        let first = atom_from_value(&value, "tydenso_manual_index_roundtrip_test").unwrap();
        let second = atom_from_value(&value, "tydenso_manual_index_roundtrip_test").unwrap();
        let first_symbol = variable_symbol(&first);

        assert_eq!(first_symbol, variable_symbol(&second));
        assert!(first_symbol.has_tag(&SPENSO_TAG.index));
        assert_eq!(
            IndexDisplay::from_symbol(first_symbol),
            Some(IndexDisplay::Attach {
                base: Box::new(IndexDisplay::symbol("mu").unwrap()),
                top: None,
                bottom: Some(Box::new(IndexDisplay::Number(1))),
            })
        );

        let imported = decode_atom(&encode_atom(&first).unwrap(), "round-tripped index").unwrap();
        assert_eq!(
            IndexDisplay::from_symbol(variable_symbol(&imported)),
            IndexDisplay::from_symbol(first_symbol)
        );
    }

    #[test]
    fn annotated_index_content_preserves_the_exact_atom_identity() {
        let exact =
            Atom::var(parse_symbol("i", "tydenso_exact_index_metadata_test", None).unwrap());
        let value = display_index_value(DISPLAY_INDEX_VERSION, &semantic_atom_ast(&exact));

        assert_eq!(
            atom_from_value(&value, "unrelated_fallback_namespace").unwrap(),
            exact
        );
    }

    #[test]
    fn generated_symbol_collisions_fail_closed() {
        let expected = IndexDisplay::symbol("collision-expected").unwrap();
        let conflicting = IndexDisplay::symbol("collision-conflicting").unwrap();
        let namespace = "tydenso_manual_index_collision_test";
        let default_namespace = DefaultNamespace {
            namespace: namespace.to_owned().into(),
            data: "",
            file: "".into(),
            line: 0,
        };
        SymbolBuilder::new(
            default_namespace.attach_namespace(&display_index_symbol_name(&expected)),
        )
        .with_tags([SPENSO_TAG.index.clone()])
        .with_user_data(conflicting.symbol_user_data())
        .build()
        .unwrap();

        assert!(
            register_display_index(&expected, namespace)
                .unwrap_err()
                .contains("different manual-index metadata")
        );
    }

    #[test]
    fn representation_palette_is_canonical_symbol_metadata() {
        let value = representation_value(
            "M",
            vec![
                display_index_value(DISPLAY_INDEX_VERSION, &Value::Text("mu".to_owned())),
                display_index_value(DISPLAY_INDEX_VERSION, &Value::Text("nu".to_owned())),
            ],
        );
        let atom = atom_from_value(&value, "spenso").unwrap();
        let AtomView::Fun(function) = atom.as_view() else {
            panic!("representation should be a function atom");
        };
        let metadata = function.get_symbol().get_data();
        let palette = spenso::structure::representation::RepresentationMetadata::from_symbol(
            function.get_symbol(),
        )
        .unwrap()
        .index_palette;

        assert_eq!(
            palette.resolve(1),
            Some(IndexDisplay::symbol("mu").unwrap())
        );
        assert_eq!(
            palette.resolve(2),
            Some(IndexDisplay::symbol("nu").unwrap())
        );
        assert_eq!(
            palette.resolve(3),
            Some(
                IndexDisplay::symbol("mu")
                    .unwrap()
                    .with_bottom(IndexDisplay::Number(1))
            )
        );

        let imported =
            decode_atom(&encode_atom(&atom).unwrap(), "representation round trip").unwrap();
        let AtomView::Fun(imported) = imported.as_view() else {
            panic!("imported representation should be a function atom");
        };
        assert_eq!(imported.get_symbol().get_data(), metadata);
    }

    #[test]
    fn representation_rejects_a_second_palette_for_the_same_symbol() {
        let first = representation_value("PaletteConflict", vec![Value::Text("mu".to_owned())]);
        let _ = atom_from_value(&first, "spenso").unwrap();
        let second = representation_value("PaletteConflict", vec![Value::Text("rho".to_owned())]);

        assert!(
            atom_from_value(&second, "spenso")
                .unwrap_err()
                .contains("different fixed index palette")
        );
    }

    #[test]
    fn custom_dual_slots_share_one_head_and_contract() {
        use spenso::network::library::symbolic::ETS;

        let namespace = "tydenso_custom_dual_contraction_test";
        let representation = dualizable_representation_value("R", namespace);
        let i = atom_from_value(&slot_value(representation.clone(), "i", false), "spenso").unwrap();
        let j = atom_from_value(&slot_value(representation.clone(), "j", false), "spenso").unwrap();
        let dual_j = atom_from_value(&slot_value(representation, "j", true), "spenso").unwrap();

        let AtomView::Fun(i_representation) = i.as_view() else {
            panic!("base slot should be a representation call");
        };
        let AtomView::Fun(dual_wrapper) = dual_j.as_view() else {
            panic!("dual slot should be wrapped");
        };
        let AtomView::Fun(j_representation) = dual_wrapper.iter().next().unwrap() else {
            panic!("dual wrapper should contain a representation call");
        };
        assert_eq!(i_representation.get_symbol(), j_representation.get_symbol());

        let vector = parse_symbol("q", namespace, None).unwrap();
        let expression = ETS.metric(i.clone(), dual_j) * vector.call(j);
        assert_eq!(expression.simplify_metrics(), vector.call(i));
    }

    #[test]
    fn stripped_dual_representations_keep_the_dind_wrapper() {
        let base =
            dualizable_representation_value("R", "tydenso_stripped_dual_representation_test");
        let base_atom = atom_from_value(&base, "spenso").unwrap();
        let AtomView::Fun(base_representation) = base_atom.as_view() else {
            panic!("base representation should be an unwrapped function call");
        };
        assert_eq!(base_representation.get_symbol().get_stripped_name(), "R");
        assert_eq!(base_representation.get_nargs(), 1);

        let Value::Map(mut representation) = base else {
            unreachable!();
        };
        representation
            .iter_mut()
            .find(|(key, _)| key == &Value::Text("is-dual".to_owned()))
            .unwrap()
            .1 = Value::Bool(true);

        let atom = atom_from_value(&Value::Map(representation), "spenso").unwrap();
        let AtomView::Fun(dual) = atom.as_view() else {
            panic!("dual representation should be a dind call");
        };
        assert_eq!(dual.get_symbol().get_stripped_name(), "dind");
        assert_eq!(dual.get_nargs(), 1);
        let AtomView::Fun(representation) = dual.iter().next().unwrap() else {
            panic!("dind should wrap the stripped representation call");
        };
        assert_eq!(representation.get_symbol().get_stripped_name(), "R");
        assert_eq!(representation.get_nargs(), 1);
    }

    #[test]
    fn representation_index_rows_are_validated_and_registered() {
        let bottom =
            representation_value_with_row("Bottom", vec![Value::Text("i".to_owned())], "bottom");
        let atom = atom_from_value(&bottom, "spenso").unwrap();
        let AtomView::Fun(representation) = atom.as_view() else {
            panic!("representation should be a function atom");
        };
        assert_eq!(
            spenso::structure::representation::RepresentationMetadata::from_symbol(
                representation.get_symbol(),
            )
            .unwrap()
            .index_row,
            IndexRow::Bottom
        );

        let invalid =
            representation_value_with_row("Invalid", vec![Value::Text("i".to_owned())], "middle");
        assert!(
            atom_from_value(&invalid, "spenso")
                .unwrap_err()
                .contains("index-row must be \"top\" or \"bottom\"")
        );
    }

    #[test]
    fn missing_index_row_defaults_only_the_canonical_bispinor_to_bottom() {
        let descriptor = Vec::<(Value, Value)>::new();
        assert_eq!(
            representation_index_row(&descriptor, "spenso::bis").unwrap(),
            IndexRow::Bottom
        );
        assert_eq!(
            representation_index_row(&descriptor, "example::bis").unwrap(),
            IndexRow::Top
        );
    }

    #[test]
    fn a_distinct_dual_name_is_rejected_instead_of_creating_a_second_representation() {
        let Value::Map(mut representation) =
            dualizable_representation_value("R", "tydenso_distinct_dual_name_test")
        else {
            unreachable!();
        };
        let dual_name = representation
            .iter_mut()
            .find(|(key, _)| key == &Value::Text("dual-name".to_owned()))
            .unwrap();
        dual_name.1 = Value::Text("Rbar".to_owned());

        assert!(
            atom_from_value(&Value::Map(representation), "spenso")
                .unwrap_err()
                .contains("cannot name a different representation symbol")
        );
    }

    #[test]
    fn custom_palette_sidecar_round_trips_without_visual_or_identity_data() {
        let name = "SidecarPalette";
        let qualified = format!("tydenso_palette_registration_test::{name}");
        let descriptor = representation_value_with_row(
            name,
            vec![
                display_index_value(DISPLAY_INDEX_VERSION, &Value::Text("mu".to_owned())),
                display_index_value(DISPLAY_INDEX_VERSION, &mu_one_ast()),
            ],
            "bottom",
        );
        let payload = construct(&value_bytes(&descriptor)).unwrap();
        let key = AttachmentKey::new(
            REPRESENTATION_ATTACHMENT_SCHEMA,
            REPRESENTATION_ATTACHMENT_VERSION,
            qualified.as_bytes().to_vec(),
        )
        .unwrap();
        let parsed = parse_payload(&payload).unwrap();
        let data = parsed.attachment(&key).expect("custom sidecar");
        let Value::Array(fields) = decode_cbor_exact(data, "sidecar").unwrap() else {
            panic!("sidecar DATA should be an array");
        };
        assert_eq!(
            fields.len(),
            3,
            "v2 DATA is class, palette, and preferred index row"
        );
        let declaration = decode_representation_declaration(data).unwrap();
        assert_eq!(declaration.class, PortableRepresentationClass::SelfDual);
        assert_eq!(declaration.index_row, IndexRow::Bottom);
        assert_eq!(
            declaration
                .index_palette
                .resolve(1)
                .unwrap()
                .to_native_string(),
            "mu"
        );
        assert_eq!(
            declaration
                .index_palette
                .resolve(2)
                .unwrap()
                .to_native_string(),
            "mu_(1)"
        );

        let round_tripped = cook_indices(&payload).unwrap();
        let round_tripped = parse_payload(&round_tripped).unwrap();
        assert_eq!(round_tripped.attachment(&key), Some(data));
    }

    #[test]
    fn minkowski_descriptor_uses_its_local_inline_metric_class() {
        let descriptor = cbor_map([
            ("kind", Value::Text("representation".to_owned())),
            ("name", Value::Text("mink".to_owned())),
            ("namespace", Value::Text("spenso".to_owned())),
            ("dimension", Value::Integer(4.into())),
            ("self-dual", Value::Bool(true)),
            ("indices", Value::Null),
        ]);
        let payload = construct(&value_bytes(&descriptor)).unwrap();
        let key = AttachmentKey::new(
            REPRESENTATION_ATTACHMENT_SCHEMA,
            REPRESENTATION_ATTACHMENT_VERSION,
            b"spenso::mink".to_vec(),
        )
        .unwrap();
        let parsed = parse_payload(&payload).unwrap();
        let declaration =
            decode_representation_declaration(parsed.attachment(&key).unwrap()).unwrap();
        assert_eq!(declaration.class, PortableRepresentationClass::InlineMetric);
        assert_eq!(declaration.index_row, IndexRow::Top);
    }

    #[test]
    fn raw_representation_conflict_is_rejected_before_registration_or_import() {
        let name = "tydenso_sidecar_raw_conflict_test::R";
        assert!(Symbol::get_symbol(NamespacedSymbol::parse(name)).is_none());
        let numeric = RepresentationDeclaration {
            class: PortableRepresentationClass::SelfDual,
            index_palette: IndexPalette::Numeric,
            index_row: IndexRow::Top,
        };
        let dual = RepresentationDeclaration {
            class: PortableRepresentationClass::Dualizable,
            index_palette: IndexPalette::Numeric,
            index_row: IndexRow::Top,
        };
        let x = Atom::var(parse_symbol("x", "tydenso_sidecar_raw_conflict_test", None).unwrap());
        let h = Atom::var(parse_symbol("h", "tydenso_sidecar_raw_conflict_test", None).unwrap());
        let first = tymbolica_atom_payload::encode_atom_with_attachments(
            &x,
            [representation_attachment(name, &numeric)],
        )
        .unwrap();
        let second = tymbolica_atom_payload::encode_atom_with_attachments(
            &h,
            [representation_attachment(name, &dual)],
        )
        .unwrap();

        assert!(
            wrap_indices(&first, &second)
                .unwrap_err()
                .contains("conflicting data for attachment")
        );
        assert!(Symbol::get_symbol(NamespacedSymbol::parse(name)).is_none());
    }

    #[test]
    fn unsupported_representation_attachment_versions_fail_preflight() {
        let name = "tydenso_sidecar_future_version_test::R";
        let declaration = RepresentationDeclaration {
            class: PortableRepresentationClass::SelfDual,
            index_palette: IndexPalette::Numeric,
            index_row: IndexRow::Top,
        };
        let attachment = Attachment::new(
            AttachmentKey::new(
                REPRESENTATION_ATTACHMENT_SCHEMA,
                REPRESENTATION_ATTACHMENT_VERSION + 1,
                name.as_bytes().to_vec(),
            )
            .unwrap(),
            encode_representation_declaration(&declaration).unwrap(),
        )
        .unwrap();
        let atom =
            Atom::var(parse_symbol("x", "tydenso_sidecar_future_version_test", None).unwrap());
        let payload =
            tymbolica_atom_payload::encode_atom_with_attachments(&atom, [attachment]).unwrap();

        let error = decode_atom(&payload, "future representation payload").unwrap_err();
        assert!(error.contains("unsupported spenso.representation attachment version 3"));
        assert!(Symbol::get_symbol(NamespacedSymbol::parse(name)).is_none());
    }

    #[test]
    fn legacy_raw_atoms_are_rejected_but_current_generic_envelopes_are_accepted() {
        let atom = Atom::var(parse_symbol("x", "tydenso_legacy_payload_test", None).unwrap());
        let current = tymbolica_atom_payload::encode_atom(&atom).unwrap();
        assert_eq!(decode_atom(&current, "current payload").unwrap(), atom);

        let legacy = parse_payload(&current).unwrap().atom_bytes().to_vec();
        let error = decode_atom(&legacy, "legacy payload").unwrap_err();
        assert!(error.contains("legacy raw Atom bytes"));
        assert!(error.contains("aligned Tymbolica/Tydenso package versions"));
    }

    #[test]
    fn from_ast_rejects_legacy_semantic_atoms_before_import() {
        let namespace = "tydenso_legacy_ast_test";
        let atom = Atom::var(parse_symbol("x", namespace, None).unwrap());
        let current = tymbolica_atom_payload::encode_atom(&atom).unwrap();
        let legacy = parse_payload(&current).unwrap().atom_bytes().to_vec();
        let namespace = value_bytes(&Value::Text(namespace.to_owned()));

        let current_ast = value_bytes(&semantic_atom_payload_ast(current));
        let output = from_ast(&current_ast, &namespace).unwrap();
        assert_eq!(decode_atom(&output, "current AST output").unwrap(), atom);

        let legacy_ast = value_bytes(&semantic_atom_payload_ast(legacy));
        let error = from_ast(&legacy_ast, &namespace).unwrap_err();
        assert!(error.contains("ast uses legacy raw Atom bytes"));
        assert!(error.contains("aligned Tymbolica/Tydenso package versions"));
    }

    #[test]
    fn unary_and_list_outputs_preserve_unknown_and_referenced_representation_sidecars() {
        let namespace = "tydenso_sidecar_fanout_test";
        let representation = dualizable_representation_value("R", namespace);
        let expression = tensor_value(
            "T",
            namespace,
            vec![
                slot_value(representation.clone(), "i", false),
                slot_value(representation, "j", true),
            ],
        );
        let payload = construct(&value_bytes(&expression)).unwrap();
        let (unknown_key, unknown) = unknown_attachment(b"fanout", b"preserve me");
        let payload = add_attachment(&payload, unknown);
        let rep_key = AttachmentKey::new(
            REPRESENTATION_ATTACHMENT_SCHEMA,
            REPRESENTATION_ATTACHMENT_VERSION,
            format!("{namespace}::R").into_bytes(),
        )
        .unwrap();

        let unary = cook_indices(&payload).unwrap();
        let unary = parse_payload(&unary).unwrap();
        assert_eq!(
            unary.attachment(&unknown_key),
            Some(b"preserve me".as_slice())
        );
        assert!(unary.attachment(&rep_key).is_some());

        let fanout = list_dangling(&payload).unwrap();
        let Value::Array(outputs) = decode_cbor(&fanout, "fanout").unwrap() else {
            panic!("list_dangling should return an array");
        };
        assert_eq!(outputs.len(), 2);
        for output in outputs {
            let Value::Bytes(output) = output else {
                panic!("fanout entry should be an Atom payload");
            };
            let parsed = parse_payload(&output).unwrap();
            assert_eq!(
                parsed.attachment(&unknown_key),
                Some(b"preserve me".as_slice())
            );
            assert!(parsed.attachment(&rep_key).is_some());
        }
    }

    #[test]
    fn multi_input_operation_merges_attachment_contexts_before_import() {
        let namespace = "tydenso_sidecar_multi_input_test";
        let representation = representation_value("R", vec![Value::Text("mu".to_owned())]);
        let expression = tensor_value("T", namespace, vec![slot_value(representation, "i", false)]);
        let expr = construct(&value_bytes(&expression)).unwrap();
        let (expr_key, expr_unknown) = unknown_attachment(b"multi-expr", b"expr-data");
        let expr = add_attachment(&expr, expr_unknown);

        let header_atom =
            Atom::var(parse_symbol("wrap", "tydenso_sidecar_multi_input_test", None).unwrap());
        // Duplicate the expression's representation declaration in the second
        // input: identical raw entries must merge, not count twice or conflict.
        let expr_attachments = parse_payload(&expr).unwrap().attachment_set();
        let header = encode_atom_from_set(&header_atom, &expr_attachments).unwrap();
        let (header_key, header_unknown) = unknown_attachment(b"multi-header", b"header-data");
        let header = add_attachment(&header, header_unknown);

        let output = wrap_indices(&expr, &header).unwrap();
        let parsed = parse_payload(&output).unwrap();
        assert_eq!(parsed.attachment(&expr_key), Some(b"expr-data".as_slice()));
        assert_eq!(
            parsed.attachment(&header_key),
            Some(b"header-data".as_slice())
        );
        assert!(parsed.attachments().iter().any(|attachment| {
            attachment.schema() == REPRESENTATION_ATTACHMENT_SCHEMA
                && attachment.version() == REPRESENTATION_ATTACHMENT_VERSION
        }));
    }

    #[test]
    fn from_ast_preflights_embedded_representation_attachments() {
        let namespace = "tydenso_sidecar_ast_test";
        let representation = dualizable_representation_value("R", namespace);
        let payload = construct(&value_bytes(&representation)).unwrap();
        let (unknown_key, unknown) = unknown_attachment(b"ast", b"ast-data");
        let payload = add_attachment(&payload, unknown);
        let ast = ast_node(
            "semantic-metadata",
            vec![Value::Text("visible".to_owned())],
            vec![(
                "value",
                cbor_map([
                    ("protocol", Value::Text("tymbolica".to_owned())),
                    ("version", Value::Integer(1.into())),
                    ("kind", Value::Text("atom".to_owned())),
                    ("atom", Value::Bytes(payload)),
                ]),
            )],
        );
        let output = from_ast(
            &value_bytes(&ast),
            &value_bytes(&Value::Text(namespace.to_owned())),
        )
        .unwrap();
        let parsed = parse_payload(&output).unwrap();
        assert_eq!(
            parsed.attachment(&unknown_key),
            Some(b"ast-data".as_slice())
        );
        let rep_key = AttachmentKey::new(
            REPRESENTATION_ATTACHMENT_SCHEMA,
            REPRESENTATION_ATTACHMENT_VERSION,
            format!("{namespace}::R").into_bytes(),
        )
        .unwrap();
        assert!(parsed.attachment(&rep_key).is_some());
    }
}
