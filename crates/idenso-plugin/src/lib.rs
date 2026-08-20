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
    IndexDisplay, IndexPalette, LibraryRep, RepName, initialize as initialize_representations,
};
use symbolica::atom::{
    Atom, AtomCore, AtomView, DefaultNamespace, NamespacedSymbol, Symbol, SymbolAttribute,
    SymbolBuilder,
};
use symbolica::printer::PrintOptions;
use tymbolica_atom_payload::{
    decode_atom as decode_shared_atom, encode_atom as encode_shared_atom,
};
use wasm_minimal_protocol::*;

initiate_protocol!();

const DISPLAY_INDEX_VERSION: i64 = 1;
const MAX_DISPLAY_INDEX_AST_BYTES: usize = 64 * 1024;
const MAX_DISPLAY_INDEX_DEPTH: usize = 16;
const MAX_DISPLAY_INDEX_NODES: usize = 64;

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

fn decode_atom(input: &[u8], label: &str) -> Result<Atom, String> {
    tymbolica_symbol_registry::initialize();
    decode_shared_atom(input)
        .map_err(|error| format!("{label} must be Atom payload bytes: {error}"))
}

fn encode_atom(atom: &Atom) -> Result<Vec<u8>, String> {
    encode_shared_atom(atom).map_err(|error| format!("could not encode Tydenso result: {error}"))
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
fn exact_atom_from_index_ast(value: &Value) -> Result<Option<Atom>, String> {
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
        Some(Value::Bytes(bytes)) => decode_atom(bytes, "tymbolica index metadata").map(Some),
        Some(other) => Err(format!(
            "tymbolica index metadata atom must be bytes, got {other:?}"
        )),
        None => Err("tymbolica index metadata missing atom".to_owned()),
    }
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
    initialize_representations();
    validate_canonical_dual_name(map, name)?;
    let qualified_name = if name.contains("::") {
        name.to_owned()
    } else {
        format!("{namespace}::{name}")
    };
    let namespaced = NamespacedSymbol::parse(&qualified_name);
    let self_dual = map_bool(map, "self-dual", false)?;
    let index_palette = representation_index_palette(map)?;

    if let Some(symbol) = Symbol::get_symbol(namespaced)
        && let Ok(representation) = LibraryRep::try_from_symbol_coerced(symbol)
    {
        if representation.is_self_dual() != self_dual {
            return Err(format!(
                "symbol {} already exists with a different representation type",
                symbol.get_name()
            ));
        }
        if representation
            .metadata()
            .is_none_or(|metadata| metadata.index_palette != index_palette)
        {
            return Err(format!(
                "symbol {} already exists with a different fixed index palette",
                symbol.get_name()
            ));
        }
        return Ok(representation);
    }

    if self_dual {
        LibraryRep::new_self_dual_with_index_palette(&qualified_name, index_palette)
    } else {
        LibraryRep::new_dual_with_index_palette(&qualified_name, index_palette)
    }
    .map_err(|error| error.to_string())
}

fn representation_atom(map: &[(Value, Value)], index: Option<&Value>) -> Result<Atom, String> {
    let name = map_text(map, "name")?;
    let namespace = map_text_or(map, "namespace", "spenso")?;
    let dimension = map_get(map, "dimension").ok_or_else(|| "missing dimension".to_owned())?;
    let representation = parse_representation(name, namespace, map)?;
    let symbol = representation.symbol();
    let mut arguments = vec![atom_from_value(dimension, namespace)?];
    if let Some(index) = index {
        arguments.push(atom_from_value(index, namespace)?);
    }
    Ok(symbol.call_args(arguments))
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

fn atom_from_value(value: &Value, namespace: &str) -> Result<Atom, String> {
    tymbolica_symbol_registry::initialize();
    match value {
        Value::Bytes(bytes) => decode_atom(bytes, "expression"),
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
                    .map(|argument| atom_from_value(argument, symbol_namespace))
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
                    .map(|argument| atom_from_value(argument, symbol_namespace))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(symbol.call_args(arguments))
            }
            "representation" => representation_atom(map, None),
            "slot" => slot_atom(map),
            "sum" => Ok(Atom::add_many(
                map_array(map, "terms")?
                    .iter()
                    .map(|term| atom_from_value(term, namespace))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            "product" => Ok(Atom::mul_many(
                map_array(map, "factors")?
                    .iter()
                    .map(|factor| atom_from_value(factor, namespace))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            "negative" => Ok(-atom_from_value(
                map_get(map, "expression").ok_or_else(|| "missing expression".to_owned())?,
                namespace,
            )?),
            "power" => Ok(atom_from_value(
                map_get(map, "base").ok_or_else(|| "missing base".to_owned())?,
                namespace,
            )?
            .pow(atom_from_value(
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

fn decode_symbol(input: &[u8], label: &str) -> Result<Symbol, String> {
    match decode_atom(input, label)?.as_view() {
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

fn encode_atom_array(atoms: Vec<Atom>) -> Result<Vec<u8>, String> {
    let values = atoms
        .iter()
        .map(|atom| encode_atom(atom).map(Value::Bytes))
        .collect::<Result<Vec<_>, _>>()?;
    let mut output = Vec::new();
    ciborium::into_writer(&Value::Array(values), &mut output)
        .map_err(|error| format!("could not encode Tydenso result array: {error}"))?;
    Ok(output)
}

#[wasm_func]
pub fn construct(value: &[u8]) -> Result<Vec<u8>, String> {
    let value = decode_cbor(value, "value")?;
    encode_atom(&atom_from_value(&value, "spenso")?)
}

#[wasm_func]
pub fn from_ast(ast: &[u8], namespace: &[u8]) -> Result<Vec<u8>, String> {
    let namespace = match decode_cbor(namespace, "namespace")? {
        Value::Text(namespace) => namespace,
        other => return Err(format!("namespace must be text, got {other:?}")),
    };
    encode_atom(&tymbolica_typst_ast::atom_from_ast(ast, &namespace, "ast")?)
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
    encode_atom(
        &decode_atom(expr, "expr")?
            .cook_function()
            .map_err(|error| format!("{error:?}"))?,
    )
}

#[wasm_func]
pub fn cook_indices(expr: &[u8]) -> Result<Vec<u8>, String> {
    encode_atom(&decode_atom(expr, "expr")?.cook_indices())
}

#[wasm_func]
pub fn dirac_adjoint(expr: &[u8]) -> Result<Vec<u8>, String> {
    encode_atom(
        &decode_atom(expr, "expr")?
            .dirac_adjoint::<AbstractIndex>()
            .map_err(|error| error.to_string())?,
    )
}

#[wasm_func]
pub fn expand_bis(expr: &[u8]) -> Result<Vec<u8>, String> {
    encode_atom(&expanded(decode_atom(expr, "expr")?.expand_bis()))
}

#[wasm_func]
pub fn expand_color(expr: &[u8]) -> Result<Vec<u8>, String> {
    encode_atom(&expanded(decode_atom(expr, "expr")?.expand_color()))
}

#[wasm_func]
pub fn expand_metrics(expr: &[u8]) -> Result<Vec<u8>, String> {
    encode_atom(&expanded(decode_atom(expr, "expr")?.expand_metrics()))
}

#[wasm_func]
pub fn expand_mink(expr: &[u8]) -> Result<Vec<u8>, String> {
    encode_atom(&expanded(decode_atom(expr, "expr")?.expand_mink()))
}

#[wasm_func]
pub fn expand_mink_bis(expr: &[u8]) -> Result<Vec<u8>, String> {
    encode_atom(&expanded(decode_atom(expr, "expr")?.expand_mink_bis()))
}

#[wasm_func]
pub fn list_dangling(expr: &[u8]) -> Result<Vec<u8>, String> {
    encode_atom_array(decode_atom(expr, "expr")?.list_dangling::<AbstractIndex>())
}

#[wasm_func]
pub fn simplify_color(expr: &[u8]) -> Result<Vec<u8>, String> {
    encode_atom(&decode_atom(expr, "expr")?.simplify_color())
}

#[wasm_func]
pub fn simplify_gamma(expr: &[u8]) -> Result<Vec<u8>, String> {
    encode_atom(&decode_atom(expr, "expr")?.simplify_gamma())
}

#[wasm_func]
pub fn simplify_metrics(expr: &[u8]) -> Result<Vec<u8>, String> {
    encode_atom(&decode_atom(expr, "expr")?.simplify_metrics())
}

#[wasm_func]
pub fn to_dots(expr: &[u8]) -> Result<Vec<u8>, String> {
    encode_atom(&decode_atom(expr, "expr")?.to_dots())
}

#[wasm_func]
pub fn wrap_dummies(expr: &[u8], header: &[u8]) -> Result<Vec<u8>, String> {
    let header = decode_symbol(header, "header")?;
    encode_atom(&decode_atom(expr, "expr")?.wrap_dummies::<AbstractIndex>(header))
}

#[wasm_func]
pub fn wrap_indices(expr: &[u8], header: &[u8]) -> Result<Vec<u8>, String> {
    let header = decode_symbol(header, "header")?;
    encode_atom(&decode_atom(expr, "expr")?.wrap_indices(header))
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn representation_value(name: &str, indices: Vec<Value>) -> Value {
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
        ])
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

    fn semantic_atom_ast(atom: &Atom) -> Value {
        ast_node(
            "semantic-metadata",
            vec![Value::Text("visible".to_owned())],
            vec![(
                "value",
                cbor_map([
                    ("protocol", Value::Text("tymbolica".to_owned())),
                    ("version", Value::Integer(1.into())),
                    ("kind", Value::Text("atom".to_owned())),
                    ("atom", Value::Bytes(encode_atom(atom).unwrap())),
                ]),
            )],
        )
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
}
