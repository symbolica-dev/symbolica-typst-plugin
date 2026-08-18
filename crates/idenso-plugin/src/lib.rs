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
use symbolica::atom::{
    Atom, AtomCore, AtomView, DefaultNamespace, Symbol, SymbolAttribute, SymbolBuilder,
};
use symbolica::printer::PrintOptions;
use tymbolica_atom_payload::{
    decode_atom as decode_shared_atom, encode_atom as encode_shared_atom,
};
use wasm_minimal_protocol::*;

initiate_protocol!();

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

fn parse_representation_symbol(
    name: &str,
    namespace: &str,
    map: &[(Value, Value)],
) -> Result<Symbol, String> {
    let namespace = DefaultNamespace {
        namespace: namespace.to_owned().into(),
        data: "",
        file: "".into(),
        line: 0,
    };
    let namespaced = namespace.attach_namespace(name);
    if let Some(symbol) = Symbol::get_symbol(namespaced.clone()) {
        if symbol.has_tag(&SPENSO_TAG.representation) {
            return Ok(symbol);
        }
        return Err(format!(
            "symbol {} already exists and is not a Spenso representation",
            symbol.get_name()
        ));
    }

    let mut tags = vec![SPENSO_TAG.representation.clone()];
    if map_bool(map, "self-dual", false)? {
        tags.push(SPENSO_TAG.self_dual.clone());
    } else {
        tags.push(SPENSO_TAG.dualizable.clone());
    }
    SymbolBuilder::new(namespaced)
        .with_tags(tags)
        .build()
        .map_err(|error| error.to_string())
}

fn representation_atom(map: &[(Value, Value)], index: Option<&Value>) -> Result<Atom, String> {
    let name = map_text(map, "name")?;
    let namespace = map_text_or(map, "namespace", "spenso")?;
    let dimension = map_get(map, "dimension").ok_or_else(|| "missing dimension".to_owned())?;
    let symbol = parse_representation_symbol(name, namespace, map)?;
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
