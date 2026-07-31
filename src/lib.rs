use std::io::Cursor;
use std::sync::Arc;

use ahash::HashMap;

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

use ciborium::value::Value;
use symbolica::domains::SelfRing;
use symbolica::prelude::{
    Atom, AtomCore, AtomPrinter, AtomView, Coefficient, CoefficientView, Complex, DoubleFloat,
    ExpressionEvaluator, F64, Float, Indeterminate, IntegerRing, Matrix, PolyVariable,
    PrintOptions, PrintState, Q, RationalPolynomial, RationalPolynomialField, Real,
    ReplaceSettings, Replacement, Ring, SeriesDepth, SolveError, Symbol, Z,
};
use wasm_minimal_protocol::*;

initiate_protocol!();

type MatrixField = RationalPolynomialField<IntegerRing, u16>;
type MatrixEntry = RationalPolynomial<IntegerRing, u16>;
type PluginMatrix = Matrix<MatrixField>;

const MATRIX_PAYLOAD_MAGIC: &[u8; 4] = b"SMTP";
const MATRIX_PAYLOAD_VERSION: u8 = 1;

fn decode_cbor(input: &[u8], label: &str) -> Result<Value, String> {
    ciborium::from_reader::<Value, _>(Cursor::new(input))
        .map_err(|err| format!("{label} must be CBOR-encoded: {err}"))
}

fn encode_cbor(value: Value) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    ciborium::into_writer(&value, &mut bytes)
        .map_err(|err| format!("failed to encode CBOR: {err}"))?;
    Ok(bytes)
}

fn cbor_bool(input: &[u8], label: &str) -> Result<bool, String> {
    match decode_cbor(input, label)? {
        Value::Bool(value) => Ok(value),
        other => Err(format!("{label} must be bool, got {other:?}")),
    }
}

fn cbor_usize(input: &[u8], label: &str) -> Result<usize, String> {
    value_usize(&decode_cbor(input, label)?, label)
}

fn read_u8(input: &mut &[u8], label: &str) -> Result<u8, String> {
    if input.is_empty() {
        return Err(format!("{label} is truncated"));
    }
    let value = input[0];
    *input = &input[1..];
    Ok(value)
}

fn read_u32(input: &mut &[u8], label: &str) -> Result<u32, String> {
    if input.len() < 4 {
        return Err(format!("{label} is truncated"));
    }
    let (value, rest) = input.split_at(4);
    *input = rest;
    Ok(u32::from_le_bytes(
        value.try_into().expect("u32 slice length"),
    ))
}

fn read_bytes<'a>(input: &mut &'a [u8], len: usize, label: &str) -> Result<&'a [u8], String> {
    if input.len() < len {
        return Err(format!("{label} is truncated"));
    }
    let (value, rest) = input.split_at(len);
    *input = rest;
    Ok(value)
}

fn write_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn exported_atom(atom: &Atom) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    atom.export(&mut bytes)
        .map_err(|err| format!("failed to export Atom: {err}"))?;
    Ok(bytes)
}

fn decode_payload(input: &[u8], label: &str) -> Result<Atom, String> {
    Atom::import(&mut Cursor::new(input), None)
        .map_err(|err| format!("{label} must be exported Atom bytes: {err}"))
}

fn decode_atom(input: &[u8], label: &str) -> Result<Atom, String> {
    decode_payload(input, label)
}

fn encode_atom(atom: &Atom) -> Result<Vec<u8>, String> {
    exported_atom(atom)
}

fn is_matrix_payload(input: &[u8]) -> bool {
    input.starts_with(MATRIX_PAYLOAD_MAGIC)
}

fn atom_to_matrix_entry(atom: &Atom) -> Result<MatrixEntry, String> {
    atom.try_to_rational_polynomial::<_, _, u16>(&Q, &Z, None)
        .map_err(|err| format!("matrix entry must be rational-polynomial compatible: {err}"))
}

fn atoms_to_matrix(atoms: Vec<Atom>, nrows: u32, ncols: u32) -> Result<PluginMatrix, String> {
    if nrows == 0 || ncols == 0 {
        return Err("matrix must have at least one row and one column".to_owned());
    }
    if atoms.len() != nrows as usize * ncols as usize {
        return Err(format!(
            "matrix has {} entries but shape is {nrows}x{ncols}",
            atoms.len()
        ));
    }

    let mut entries = atoms
        .iter()
        .map(atom_to_matrix_entry)
        .collect::<Result<Vec<_>, _>>()?;
    if let Some((first, rest)) = entries.split_first_mut() {
        for _ in 0..2 {
            for entry in &mut *rest {
                first.unify_variables(entry);
            }
        }
    }

    Matrix::from_linear(entries, nrows, ncols, RationalPolynomialField::new(Z))
        .map_err(|err| format!("invalid matrix: {err}"))
}

fn matrix_entries_to_atoms(matrix: &PluginMatrix) -> Vec<Atom> {
    matrix
        .clone()
        .into_vec()
        .into_iter()
        .map(|entry| entry.to_expression())
        .collect()
}

fn encode_matrix(matrix: &PluginMatrix) -> Result<Vec<u8>, String> {
    let atoms = matrix_entries_to_atoms(matrix);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(MATRIX_PAYLOAD_MAGIC);
    bytes.push(MATRIX_PAYLOAD_VERSION);
    write_u32(&mut bytes, matrix.nrows() as u32);
    write_u32(&mut bytes, matrix.ncols() as u32);
    write_u32(&mut bytes, atoms.len() as u32);
    for atom in atoms {
        let atom_bytes = encode_atom(&atom)?;
        write_u32(&mut bytes, atom_bytes.len() as u32);
        bytes.extend_from_slice(&atom_bytes);
    }
    Ok(bytes)
}

fn decode_matrix(input: &[u8], label: &str) -> Result<PluginMatrix, String> {
    let mut rest = input;
    let magic = read_bytes(&mut rest, MATRIX_PAYLOAD_MAGIC.len(), label)?;
    if magic != MATRIX_PAYLOAD_MAGIC {
        return Err(format!("{label} must be matrix bytes"));
    }
    let version = read_u8(&mut rest, label)?;
    if version != MATRIX_PAYLOAD_VERSION {
        return Err(format!(
            "{label} has unsupported matrix payload version {version}"
        ));
    }
    let nrows = read_u32(&mut rest, label)?;
    let ncols = read_u32(&mut rest, label)?;
    let len = read_u32(&mut rest, label)? as usize;
    let mut atoms = Vec::with_capacity(len);
    for index in 0..len {
        let atom_len = read_u32(&mut rest, label)? as usize;
        let atom = read_bytes(&mut rest, atom_len, label)?;
        atoms.push(decode_atom(atom, &format!("{label}[{index}]"))?);
    }
    atoms_to_matrix(atoms, nrows, ncols)
}

fn decode_atom_array(input: &[u8], label: &str) -> Result<Vec<Atom>, String> {
    match decode_cbor(input, label)? {
        Value::Array(values) => values
            .iter()
            .enumerate()
            .map(|(index, value)| atom_from_cbor_value(value, &format!("{label}[{index}]")))
            .collect(),
        other => Err(format!(
            "{label} must be an array of Atom bytes, got {other:?}"
        )),
    }
}

fn atom_from_cbor_value(value: &Value, label: &str) -> Result<Atom, String> {
    match value {
        Value::Bytes(bytes) => decode_atom(bytes, label),
        Value::Integer(n) => {
            let n: i64 = (*n)
                .try_into()
                .map_err(|_| format!("{label} integer is out of range"))?;
            Ok(Atom::num(n))
        }
        Value::Float(n) => Ok(Atom::num(*n)),
        Value::Text(text) => symbol_atom(text, "typst"),
        other => Err(format!("{label} must be Atom bytes, got {other:?}")),
    }
}

fn atom_from_ast(input: &[u8], namespace: &str, label: &str) -> Result<Atom, String> {
    let value = decode_cbor(input, label)?;
    atom_from_value(&value, namespace)
}

fn atom_from_value(value: &Value, namespace: &str) -> Result<Atom, String> {
    match value {
        Value::Integer(n) => {
            let n: i64 = (*n)
                .try_into()
                .map_err(|_| "integer literal is out of range".to_owned())?;
            Ok(Atom::num(n))
        }
        Value::Float(n) => Ok(Atom::num(*n)),
        Value::Text(text) => atom_from_leaf(text, namespace),
        Value::Map(map) => atom_from_node(map, namespace),
        other => Err(format!("unsupported Parsely AST value: {other:?}")),
    }
}

fn atom_from_leaf(text: &str, namespace: &str) -> Result<Atom, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("empty math leaf".to_owned());
    }

    if let Ok(n) = text.parse::<i64>() {
        return Ok(Atom::num(n));
    }

    if (text.contains('.') || text.contains('e') || text.contains('E'))
        && let Ok(n) = text.parse::<f64>()
        && n.is_finite()
    {
        return Ok(Atom::num(n));
    }

    symbol_atom(text, namespace)
}

fn symbol_atom(name: &str, namespace: &str) -> Result<Atom, String> {
    Symbol::parse(name.trim(), namespace.to_owned()).map(Atom::var)
}

fn atom_from_node(map: &[(Value, Value)], namespace: &str) -> Result<Atom, String> {
    let head = map_text(map, "head")?;
    let args = map_array(map, "args")?;
    let slots = map_map(map, "slots")?;

    match head {
        "add" => Ok(Atom::add_many(atoms_from_values(args, namespace)?)),
        "arg" => Ok(Symbol::ARG.call_args(atoms_from_values(args, namespace)?)),
        "mul" | "times" | "dot" => Ok(Atom::mul_many(atoms_from_values(args, namespace)?)),
        "sub" => Ok(atom_arg(args, 0, head, namespace)? - atom_arg(args, 1, head, namespace)?),
        "neg" => Ok(-atom_arg(args, 0, head, namespace)?),
        "plus" | "group" | "()" => slot_or_arg_atom(slots, "expr", args, 0, head, namespace),
        "lr" => slot_or_arg_atom(slots, "body", args, 0, head, namespace),
        "attach" => {
            let base = slot_or_arg_atom(slots, "base", args, 0, head, namespace)?;
            if let Some(exp) = slot_atom(slots, "t", namespace) {
                Ok(base.pow(exp?))
            } else {
                Ok(base)
            }
        }
        "factorial" => {
            let arg = atom_arg(args, 0, head, namespace)?;
            Ok(Symbol::parse("gamma", "symbolica")?.call(arg + Atom::num(1)))
        }
        "frac" => {
            let num = slot_or_arg_atom(slots, "num", args, 0, head, namespace)?;
            let denom = slot_or_arg_atom(slots, "denom", args, 1, head, namespace)?;
            Ok(num / denom)
        }
        "pow" => {
            let base = slot_or_arg_atom(slots, "base", args, 0, head, namespace)?;
            let exp = slot_or_arg_atom(slots, "exp", args, 1, head, namespace)?;
            Ok(base.pow(exp))
        }
        "sqrt" => {
            let radicand = slot_atom(slots, "radicand", namespace)
                .or_else(|| slot_atom(slots, "body", namespace))
                .unwrap_or_else(|| atom_arg(args, 0, head, namespace))?;
            Ok(Symbol::SQRT.call(radicand))
        }
        "root" => {
            let radicand = slot_atom(slots, "radicand", namespace)
                .or_else(|| slot_atom(slots, "body", namespace))
                .unwrap_or_else(|| {
                    atom_arg(args, 1, head, namespace)
                        .or_else(|_| atom_arg(args, 0, head, namespace))
                })?;
            if let Some(index) = slot_atom(slots, "index", namespace)
                .or_else(|| args.first().map(|v| atom_from_value(v, namespace)))
            {
                Ok(radicand.pow(Atom::num(1) / index?))
            } else {
                Ok(Symbol::SQRT.call(radicand))
            }
        }
        "abs" | "norm" => {
            let body = slot_atom(slots, "body", namespace)
                .unwrap_or_else(|| atom_arg(args, 0, head, namespace))?;
            Ok(Symbol::ABS.call(body))
        }
        "call" => {
            let symbol = slot_symbol(slots, "fn", namespace)?;
            let args = slot_atoms(slots, "body", namespace)?.unwrap_or_default();
            Ok(symbol.call_args(args))
        }
        "op-call" => {
            let symbol = slot_symbol(slots, "op", namespace)?;
            let args = slot_atoms(slots, "args", namespace)?.unwrap_or_default();
            Ok(symbol.call_args(args))
        }
        "mat" | "vec" => Err(format!(
            "{head} is matrix-valued; use matrix(...) or vec(...)"
        )),
        _ => {
            let symbol = Symbol::parse(head, namespace.to_owned())?;
            Ok(symbol.call_args(atoms_from_values(args, namespace)?))
        }
    }
}

fn atoms_from_values(values: &[Value], namespace: &str) -> Result<Vec<Atom>, String> {
    values
        .iter()
        .map(|v| atom_from_value(v, namespace))
        .collect()
}

fn atoms_from_arg_value(value: &Value, namespace: &str) -> Result<Vec<Atom>, String> {
    if let Value::Map(map) = value
        && map_text(map, "head").ok() == Some("arg")
    {
        return atoms_from_values(map_array(map, "args")?, namespace);
    }

    Ok(vec![atom_from_value(value, namespace)?])
}

fn slot_atoms(
    slots: &[(Value, Value)],
    key: &str,
    namespace: &str,
) -> Result<Option<Vec<Atom>>, String> {
    map_get(slots, key)
        .map(|value| atoms_from_arg_value(value, namespace))
        .transpose()
}

fn symbol_from_value(value: &Value, namespace: &str) -> Result<Symbol, String> {
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
        other => atom_from_value(other, namespace).and_then(|atom| {
            atom.get_symbol()
                .ok_or_else(|| "function head must be a symbol".to_owned())
        }),
    }
}

fn slot_symbol(slots: &[(Value, Value)], key: &str, namespace: &str) -> Result<Symbol, String> {
    map_get(slots, key)
        .ok_or_else(|| format!("missing {key}"))
        .and_then(|v| symbol_from_value(v, namespace))
}

fn atom_arg(args: &[Value], index: usize, head: &str, namespace: &str) -> Result<Atom, String> {
    args.get(index)
        .ok_or_else(|| format!("{head} missing argument {index}"))
        .and_then(|v| atom_from_value(v, namespace))
}

fn slot_or_arg_atom(
    slots: &[(Value, Value)],
    key: &str,
    args: &[Value],
    index: usize,
    head: &str,
    namespace: &str,
) -> Result<Atom, String> {
    slot_atom(slots, key, namespace).unwrap_or_else(|| atom_arg(args, index, head, namespace))
}

fn slot_atom(slots: &[(Value, Value)], key: &str, namespace: &str) -> Option<Result<Atom, String>> {
    map_get(slots, key).map(|v| atom_from_value(v, namespace))
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

fn map_bool(map: &[(Value, Value)], key: &str, default: bool) -> Result<bool, String> {
    match map_get(map, key) {
        Some(Value::Bool(value)) => Ok(*value),
        Some(other) => Err(format!("{key} must be bool, got {other:?}")),
        None => Ok(default),
    }
}

fn map_usize(map: &[(Value, Value)], key: &str, default: usize) -> Result<usize, String> {
    match map_get(map, key) {
        Some(value) => value_usize(value, key),
        None => Ok(default),
    }
}

fn map_optional_usize(map: &[(Value, Value)], key: &str) -> Result<Option<usize>, String> {
    match map_get(map, key) {
        Some(Value::Null) | None => Ok(None),
        Some(value) => Ok(Some(value_usize(value, key)?)),
    }
}

fn map_i64(map: &[(Value, Value)], key: &str, default: i64) -> Result<i64, String> {
    match map_get(map, key) {
        Some(value) => value_i64(value, key),
        None => Ok(default),
    }
}

fn map_f64(map: &[(Value, Value)], key: &str, default: f64) -> Result<f64, String> {
    match map_get(map, key) {
        Some(value) => value_f64(value, key),
        None => Ok(default),
    }
}

fn map_bytes<'a>(map: &'a [(Value, Value)], key: &str) -> Result<&'a [u8], String> {
    match map_get(map, key) {
        Some(Value::Bytes(bytes)) => Ok(bytes),
        Some(other) => Err(format!("{key} must be bytes, got {other:?}")),
        None => Err(format!("missing {key}")),
    }
}

fn value_i64(value: &Value, label: &str) -> Result<i64, String> {
    match value {
        Value::Integer(n) => (*n)
            .try_into()
            .map_err(|_| format!("{label} integer is out of range")),
        other => Err(format!("{label} must be integer, got {other:?}")),
    }
}

fn value_usize(value: &Value, label: &str) -> Result<usize, String> {
    let n = value_i64(value, label)?;
    n.try_into()
        .map_err(|_| format!("{label} must be non-negative"))
}

fn value_f64(value: &Value, label: &str) -> Result<f64, String> {
    match value {
        Value::Float(n) => Ok(*n),
        Value::Integer(n) => {
            let n: i64 = (*n)
                .try_into()
                .map_err(|_| format!("{label} integer is out of range"))?;
            Ok(n as f64)
        }
        Value::Map(map) => {
            let re = map_f64(map, "re", 0.0)?;
            let im = map_f64(map, "im", 0.0)?;
            if im == 0.0 {
                Ok(re)
            } else {
                Err(format!("{label} must be real for this operation"))
            }
        }
        other => Err(format!("{label} must be number, got {other:?}")),
    }
}

fn value_complex(value: &Value, label: &str) -> Result<Complex<f64>, String> {
    match value {
        Value::Map(map) => Ok(Complex::new(
            map_f64(map, "re", 0.0)?,
            map_f64(map, "im", 0.0)?,
        )),
        _ => Ok(Complex::new(value_f64(value, label)?, 0.0)),
    }
}

fn value_atom_bytes<'a>(value: &'a Value, label: &str) -> Result<&'a [u8], String> {
    match value {
        Value::Bytes(bytes) => Ok(bytes),
        other => Err(format!("{label} must be bytes, got {other:?}")),
    }
}

fn symbol_from_atom_bytes(bytes: &[u8], label: &str) -> Result<Symbol, String> {
    match decode_atom(bytes, label)?.as_view() {
        AtomView::Var(var) => Ok(var.get_symbol()),
        _ => Err(format!("{label} must be a symbol")),
    }
}

fn indeterminate_from_bytes(bytes: &[u8], label: &str) -> Result<Indeterminate, String> {
    Indeterminate::try_from(decode_atom(bytes, label)?).map_err(|err| err.to_string())
}

fn replacement_options(
    map: &[(Value, Value)],
) -> Result<((usize, Option<usize>), bool, bool, bool, usize, Vec<Symbol>), String> {
    let min_level = map_usize(map, "min-level", 0)?;
    let max_level = map_optional_usize(map, "max-level")?;
    let level_range = if let Some(Value::Array(values)) = map_get(map, "level-range") {
        if values.len() != 2 {
            return Err("level-range must have two entries".to_owned());
        }
        let max = match &values[1] {
            Value::Null => None,
            value => Some(value_usize(value, "level-range[1]")?),
        };
        (value_usize(&values[0], "level-range[0]")?, max)
    } else {
        (min_level, max_level)
    };

    let non_greedy = if let Some(Value::Array(values)) = map_get(map, "non-greedy-wildcards") {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                let bytes = value_atom_bytes(value, &format!("non-greedy-wildcards[{index}]"))?;
                symbol_from_atom_bytes(bytes, &format!("non-greedy-wildcards[{index}]"))
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        Vec::new()
    };

    Ok((
        level_range,
        map_bool(map, "level-is-tree-depth", false)?,
        map_bool(map, "partial", true)?,
        map_bool(map, "allow-new-wildcards-on-rhs", false)?,
        map_usize(map, "rhs-cache-size", 100)?,
        non_greedy,
    ))
}

fn build_replace_settings(map: &[(Value, Value)]) -> Result<ReplaceSettings, String> {
    Ok(ReplaceSettings::new()
        .once(map_bool(map, "once", false)?)
        .bottom_up(map_bool(map, "bottom-up", false)?)
        .nested(map_bool(map, "nested", false)?))
}

fn build_replacement(map: &[(Value, Value)]) -> Result<Replacement, String> {
    let pattern_atom = decode_atom(map_bytes(map, "pattern")?, "pattern")?;
    let rhs_atom = decode_atom(map_bytes(map, "rhs")?, "rhs")?;
    let pattern = pattern_atom.to_pattern();
    let rhs = rhs_atom.to_pattern();
    let (
        level_range,
        level_is_tree_depth,
        partial,
        allow_new_wildcards_on_rhs,
        rhs_cache_size,
        non_greedy,
    ) = replacement_options(map)?;

    if !allow_new_wildcards_on_rhs && let Some(wildcard) = pattern.find_new_wildcard(&rhs) {
        return Err(format!(
            "Wildcard {} does not appear in pattern",
            wildcard.get_name()
        ));
    }

    Ok(Replacement::new(pattern, rhs)
        .level_range(level_range)
        .level_is_tree_depth(level_is_tree_depth)
        .partial(partial)
        .allow_new_wildcards_on_rhs(allow_new_wildcards_on_rhs)
        .rhs_cache_size(rhs_cache_size)
        .non_greedy_wildcards(non_greedy))
}

fn repeat_replacements(
    expr: Atom,
    replacements: Vec<Replacement>,
    settings: ReplaceSettings,
    repeat: bool,
) -> Atom {
    let mut current = expr;
    loop {
        let next = current.replace_multiple_with_settings(&replacements, settings);
        if !repeat || next == current {
            return next;
        }
        current = next;
    }
}

fn symbolica_options(namespaces: bool) -> PrintOptions {
    if namespaces {
        PrintOptions::file()
    } else {
        PrintOptions::file_no_namespace()
    }
}

#[derive(Clone, Copy, Debug)]
enum SerializedFloatKind {
    Zero,
    Finite {
        negative: bool,
        mantissa: u128,
        binary_exponent: i64,
    },
    Infinite {
        negative: bool,
    },
    Nan,
}

#[derive(Clone, Copy, Debug)]
struct SerializedFloatValue {
    binary_precision: u32,
    kind: SerializedFloatKind,
}

impl SerializedFloatValue {
    fn is_zero(self) -> bool {
        matches!(self.kind, SerializedFloatKind::Zero)
    }

    fn is_negative(self) -> bool {
        matches!(
            self.kind,
            SerializedFloatKind::Finite { negative: true, .. }
                | SerializedFloatKind::Infinite { negative: true }
        )
    }
}

fn signed_hexadecimal_integer(text: &str) -> Option<i32> {
    let (negative, digits) = text
        .strip_prefix('-')
        .map_or((false, text), |digits| (true, digits));
    let digits = digits.strip_prefix('+').unwrap_or(digits);
    let value = i32::from_str_radix(digits, 16).ok()?;
    Some(if negative { -value } else { value })
}

fn serialized_float_value(bytes: &[u8]) -> Option<SerializedFloatValue> {
    let (precision, text) = bytes.split_at_checked(4)?;
    let binary_precision = u32::from_le_bytes(precision.try_into().ok()?);
    let text = std::str::from_utf8(text).ok()?;
    let kind = match text {
        "0" => SerializedFloatKind::Zero,
        "Inf" => SerializedFloatKind::Infinite { negative: false },
        "-Inf" => SerializedFloatKind::Infinite { negative: true },
        "NaN" => SerializedFloatKind::Nan,
        _ => {
            let (negative, text) = text
                .strip_prefix('-')
                .map_or((false, text), |text| (true, text));
            let (mantissa, exponent) = text.split_once("_e")?;
            let hexadecimal_exponent = i64::from(signed_hexadecimal_integer(exponent)?);

            let mut after_point = false;
            let mut saw_digit = false;
            let mut fractional_digits = 0_i64;
            let mut kept_digits = 0_i64;
            let mut skipped_digits = 0_i64;
            let mut integer_mantissa = 0_u128;
            for digit in mantissa.chars() {
                if digit == '.' {
                    if after_point {
                        return None;
                    }
                    after_point = true;
                    continue;
                }
                let digit = u128::from(digit.to_digit(16)?);
                saw_digit = true;
                if after_point {
                    fractional_digits = fractional_digits.checked_add(1)?;
                }
                if kept_digits < 30 {
                    integer_mantissa = integer_mantissa.checked_mul(16)?.checked_add(digit)?;
                    kept_digits += 1;
                } else {
                    skipped_digits = skipped_digits.checked_add(1)?;
                }
            }
            if !saw_digit || integer_mantissa == 0 {
                return None;
            }

            let binary_exponent = hexadecimal_exponent
                .checked_sub(fractional_digits)?
                .checked_add(skipped_digits)?
                .checked_mul(4)?;
            SerializedFloatKind::Finite {
                negative,
                mantissa: integer_mantissa,
                binary_exponent,
            }
        }
    };

    Some(SerializedFloatValue {
        binary_precision,
        kind,
    })
}

fn fixed_decimal(mantissa: &str, exponent: i64) -> String {
    let digits = mantissa.chars().filter(|c| *c != '.').collect::<String>();
    let decimal_position = exponent + 1;
    if decimal_position <= 0 {
        let zero_count = usize::try_from(-decimal_position).unwrap_or(usize::MAX);
        return format!("0.{}{digits}", "0".repeat(zero_count));
    }

    let decimal_position = usize::try_from(decimal_position).unwrap_or(usize::MAX);
    if decimal_position >= digits.len() {
        return format!("{digits}{}", "0".repeat(decimal_position - digits.len()));
    }
    format!(
        "{}.{}",
        &digits[..decimal_position],
        &digits[decimal_position..]
    )
}

#[derive(Clone, Copy)]
enum FloatRenderStyle {
    Symbolica,
    Typst,
    Latex,
}

fn double_float_from_u128(value: u128) -> DoubleFloat {
    let bit_length = 128_u32 - value.leading_zeros();
    let shift = bit_length.saturating_sub(53);
    let high_integer = value >> shift;
    let high = (high_integer as f64) * 2.0_f64.powi(shift as i32);
    let remainder = value - (high_integer << shift);
    DoubleFloat::from_compensated_sum(high, remainder as f64)
}

fn exact_decimal_parts(
    value: SerializedFloatValue,
    binary_precision: u32,
) -> Option<(String, i64)> {
    let SerializedFloatKind::Finite {
        mantissa,
        binary_exponent,
        ..
    } = value.kind
    else {
        return None;
    };
    let decimal_precision =
        ((f64::from(binary_precision) * std::f64::consts::LOG10_2).floor() as usize).clamp(1, 16);

    let two: DoubleFloat = 2.0.into();
    let ten: DoubleFloat = 10.0.into();
    let natural_log_ten = ten.log();
    let logarithm = double_float_from_u128(mantissa).log() / natural_log_ten
        + DoubleFloat::from(binary_exponent as f64) * (two.log() / natural_log_ten);
    let inner = logarithm.into_inner();
    let mut decimal_exponent = inner.hi().floor() as i64;
    let mut fractional = logarithm - DoubleFloat::from(decimal_exponent as f64);
    let zero: DoubleFloat = 0.0.into();
    let one: DoubleFloat = 1.0.into();
    while fractional < zero {
        decimal_exponent -= 1;
        fractional += &one;
    }
    while fractional >= one {
        decimal_exponent += 1;
        fractional -= &one;
    }

    let normalized = (fractional * natural_log_ten).exp();
    let decimal_scale = 10_u64.pow(decimal_precision.saturating_sub(1) as u32);
    let scaled = (normalized * DoubleFloat::from(decimal_scale as f64)).into_inner();
    let high_floor = scaled.hi().floor();
    let mut rounded = high_floor as u64;
    let mut fractional_part = (scaled.hi() - high_floor) + scaled.lo();
    if fractional_part < 0.0 {
        rounded = rounded.checked_sub(1)?;
        fractional_part += 1.0;
    } else if fractional_part >= 1.0 {
        rounded = rounded.checked_add(1)?;
        fractional_part -= 1.0;
    }
    if fractional_part > 0.5 || (fractional_part == 0.5 && rounded % 2 == 1) {
        rounded = rounded.checked_add(1)?;
    }

    let overflow = decimal_scale.checked_mul(10)?;
    if rounded == overflow {
        rounded = decimal_scale;
        decimal_exponent += 1;
    }
    let digits = rounded.to_string();
    if digits.len() != decimal_precision {
        return None;
    }
    let mut mantissa = if digits.len() == 1 {
        digits
    } else {
        format!("{}.{}", &digits[..1], &digits[1..])
    };
    if mantissa.contains('.') {
        let trimmed = mantissa.trim_end_matches('0').trim_end_matches('.').len();
        mantissa.truncate(trimmed);
    }
    Some((mantissa, decimal_exponent))
}

fn format_decoded_float(value: SerializedFloatValue, style: FloatRenderStyle) -> String {
    match value.kind {
        SerializedFloatKind::Zero => "0".to_owned(),
        SerializedFloatKind::Infinite { .. } => match style {
            FloatRenderStyle::Symbolica => "inf".to_owned(),
            FloatRenderStyle::Typst => "infinity".to_owned(),
            FloatRenderStyle::Latex => r"\infty".to_owned(),
        },
        SerializedFloatKind::Nan => match style {
            FloatRenderStyle::Symbolica => "nan".to_owned(),
            FloatRenderStyle::Typst => r#"upright("NaN")"#.to_owned(),
            FloatRenderStyle::Latex => r"\operatorname{NaN}".to_owned(),
        },
        SerializedFloatKind::Finite { .. } => {
            let Some((mantissa, exponent)) = exact_decimal_parts(value, value.binary_precision)
            else {
                return "nan".to_owned();
            };
            if (-6..=20).contains(&exponent) {
                fixed_decimal(&mantissa, exponent)
            } else {
                match style {
                    FloatRenderStyle::Symbolica => {
                        format!("({mantissa}*10^({exponent}))")
                    }
                    FloatRenderStyle::Typst => {
                        format!("({mantissa} times 10^({exponent}))")
                    }
                    FloatRenderStyle::Latex => {
                        format!(r"\left({mantissa}\cdot 10^{{{exponent}}}\right)")
                    }
                }
            }
        }
    }
}

fn format_serialized_float(bytes: &[u8], style: FloatRenderStyle) -> String {
    serialized_float_value(bytes)
        .map(|value| format_decoded_float(value, style))
        .unwrap_or_else(|| match style {
            FloatRenderStyle::Symbolica => "nan".to_owned(),
            FloatRenderStyle::Typst => r#"upright("NaN")"#.to_owned(),
            FloatRenderStyle::Latex => r"\operatorname{NaN}".to_owned(),
        })
}

fn render_atom(atom: &Atom, opts: PrintOptions, float_style: FloatRenderStyle) -> Vec<u8> {
    let mut replacements = Vec::<(String, String)>::new();
    let mut placeholder_index = 0_usize;
    let masked = atom.replace_map(|view, _, out| {
        let AtomView::Num(number) = view else {
            return;
        };
        let CoefficientView::Float(real, imaginary) = number.get_coeff_view() else {
            return;
        };

        let mut component = |bytes: &[u8]| {
            let value = serialized_float_value(bytes);
            if value.is_some_and(SerializedFloatValue::is_zero) {
                return None;
            }
            let name = format!("tymbolicafloatplaceholderq{placeholder_index}q");
            placeholder_index += 1;
            let placeholder = Atom::var(
                Symbol::parse(&name, "tymbolica")
                    .expect("internal float placeholder is a valid symbol"),
            );
            let token =
                AtomPrinter::new_with_options(placeholder.as_view(), opts.clone()).to_string();
            replacements.push((token, format_serialized_float(bytes, float_style)));
            Some(if value.is_some_and(SerializedFloatValue::is_negative) {
                -placeholder
            } else {
                placeholder
            })
        };

        let real = component(real.0);
        let imaginary = component(imaginary.0).map(|coefficient| {
            coefficient * Atom::num(Coefficient::Complex(Complex::new(Q.zero(), Q.one())))
        });
        **out = match (real, imaginary) {
            (Some(real), Some(imaginary)) => real + imaginary,
            (Some(real), None) => real,
            (None, Some(imaginary)) => imaginary,
            (None, None) => Atom::num(0),
        };
    });

    let mut rendered = AtomPrinter::new_with_options(masked.as_view(), opts).to_string();
    for (placeholder, value) in replacements {
        rendered = rendered.replace(&placeholder, &value);
    }
    rendered.into_bytes()
}

fn render_payload_symbolica(input: &[u8], namespaces: bool) -> Result<Vec<u8>, String> {
    if is_matrix_payload(input) {
        let matrix = decode_matrix(input, "matrix")?;
        Ok(matrix
            .format_string(&symbolica_options(namespaces), PrintState::new())
            .into_bytes())
    } else {
        let expr = decode_atom(input, "expr")?;
        Ok(render_atom(
            &expr,
            symbolica_options(namespaces),
            FloatRenderStyle::Symbolica,
        ))
    }
}

fn render_matrix_typst(matrix: &PluginMatrix) -> Result<Vec<u8>, String> {
    let entries = matrix_entries_to_atoms(matrix);
    let nrows = matrix.nrows();
    let ncols = matrix.ncols();
    let mut out = String::from("mat(");
    for row in 0..nrows {
        if row > 0 {
            out.push_str("; ");
        }
        for col in 0..ncols {
            if col > 0 {
                out.push_str(", ");
            }
            let atom = &entries[row * ncols + col];
            out.push_str(
                &String::from_utf8(render_atom(
                    atom,
                    PrintOptions::typst(),
                    FloatRenderStyle::Typst,
                ))
                .map_err(|e| e.to_string())?,
            );
        }
    }
    out.push(')');
    Ok(out.into_bytes())
}

fn render_payload_typst(input: &[u8]) -> Result<Vec<u8>, String> {
    if is_matrix_payload(input) {
        render_matrix_typst(&decode_matrix(input, "matrix")?)
    } else {
        let expr = decode_atom(input, "expr")?;
        Ok(render_atom(
            &expr,
            PrintOptions::typst(),
            FloatRenderStyle::Typst,
        ))
    }
}

fn decode_nested_matrix(input: &[u8]) -> Result<PluginMatrix, String> {
    match decode_cbor(input, "matrix")? {
        Value::Array(rows) => {
            if rows.is_empty() {
                return Err("matrix must have at least one row".to_owned());
            }
            let nrows = rows.len() as u32;
            let mut ncols = None;
            let mut atoms = Vec::new();
            for (row_index, row) in rows.iter().enumerate() {
                let Value::Array(cols) = row else {
                    return Err(format!("matrix row {row_index} must be an array"));
                };
                if cols.is_empty() {
                    return Err(format!("matrix row {row_index} is empty"));
                }
                if let Some(expected) = ncols {
                    if expected != cols.len() {
                        return Err("matrix is not rectangular".to_owned());
                    }
                } else {
                    ncols = Some(cols.len());
                }
                for (col_index, value) in cols.iter().enumerate() {
                    atoms.push(atom_from_cbor_value(
                        value,
                        &format!("matrix[{row_index}][{col_index}]"),
                    )?);
                }
            }
            atoms_to_matrix(atoms, nrows, ncols.unwrap() as u32)
        }
        other => Err(format!("matrix must be nested array, got {other:?}")),
    }
}

fn matrix_vec_from_values(input: &[u8]) -> Result<PluginMatrix, String> {
    let atoms = decode_atom_array(input, "values")?;
    atoms_to_matrix(atoms, cbor_len(input, "values")? as u32, 1)
}

fn cbor_len(input: &[u8], label: &str) -> Result<usize, String> {
    match decode_cbor(input, label)? {
        Value::Array(values) => Ok(values.len()),
        other => Err(format!("{label} must be array, got {other:?}")),
    }
}

fn matrix_from_diag(input: &[u8]) -> Result<PluginMatrix, String> {
    let atoms = decode_atom_array(input, "diag")?;
    if atoms.is_empty() {
        return Err("diagonal must not be empty".to_owned());
    }
    let mut entries = atoms
        .iter()
        .map(atom_to_matrix_entry)
        .collect::<Result<Vec<_>, _>>()?;
    if let Some((first, rest)) = entries.split_first_mut() {
        for _ in 0..2 {
            for entry in &mut *rest {
                first.unify_variables(entry);
            }
        }
    }
    Ok(Matrix::eye(&entries, RationalPolynomialField::new(Z)))
}

fn unify_matrices(lhs: &PluginMatrix, rhs: &PluginMatrix) -> (PluginMatrix, PluginMatrix) {
    let mut zero = lhs.field().zero();
    let mut lhs_data = lhs.clone().into_vec();
    let mut rhs_data = rhs.clone().into_vec();
    for entry in &mut lhs_data {
        zero.unify_variables(entry);
    }
    for entry in &mut rhs_data {
        zero.unify_variables(entry);
    }
    (
        Matrix::from_linear(
            lhs_data,
            lhs.nrows() as u32,
            lhs.ncols() as u32,
            RationalPolynomialField::new(Z),
        )
        .expect("existing lhs matrix shape"),
        Matrix::from_linear(
            rhs_data,
            rhs.nrows() as u32,
            rhs.ncols() as u32,
            RationalPolynomialField::new(Z),
        )
        .expect("existing rhs matrix shape"),
    )
}

fn unify_matrix_scalar(matrix: &PluginMatrix, scalar: MatrixEntry) -> (PluginMatrix, MatrixEntry) {
    let mut zero = matrix.field().zero();
    let mut data = matrix.clone().into_vec();
    for entry in &mut data {
        zero.unify_variables(entry);
    }
    let mut scalar = scalar;
    zero.unify_variables(&mut scalar);
    (
        Matrix::from_linear(
            data,
            matrix.nrows() as u32,
            matrix.ncols() as u32,
            RationalPolynomialField::new(Z),
        )
        .expect("existing matrix shape"),
        scalar,
    )
}

fn cbor_atom_array(atoms: Vec<Atom>) -> Result<Vec<u8>, String> {
    encode_cbor(Value::Array(
        atoms
            .iter()
            .map(|atom| encode_atom(atom).map(Value::Bytes))
            .collect::<Result<Vec<_>, _>>()?,
    ))
}

fn atoms_cbor_value(atoms: impl IntoIterator<Item = Atom>) -> Result<Value, String> {
    Ok(Value::Array(
        atoms
            .into_iter()
            .map(|atom| encode_atom(&atom).map(Value::Bytes))
            .collect::<Result<Vec<_>, _>>()?,
    ))
}

fn cbor_f64(value: f64) -> Result<Vec<u8>, String> {
    encode_cbor(Value::Float(value))
}

fn cbor_f64_array(values: Vec<f64>) -> Result<Vec<u8>, String> {
    encode_cbor(Value::Array(values.into_iter().map(Value::Float).collect()))
}

fn atom_list(value: &Value, label: &str) -> Result<Vec<Atom>, String> {
    match value {
        Value::Array(values) => values
            .iter()
            .enumerate()
            .map(|(index, value)| atom_from_cbor_value(value, &format!("{label}[{index}]")))
            .collect(),
        Value::Bytes(bytes) => Ok(vec![decode_atom(bytes, label)?]),
        other => Err(format!(
            "{label} must be an expression or array, got {other:?}"
        )),
    }
}

fn collected_coefficients(expr: &Atom, variables: &[Atom]) -> Result<Vec<(Atom, Atom)>, String> {
    if variables.is_empty() {
        return Err("at least one variable or function is required".to_owned());
    }
    if variables
        .iter()
        .any(|variable| !matches!(variable.as_view(), AtomView::Var(_) | AtomView::Fun(_)))
    {
        return Err("collection variables must be variables or functions".to_owned());
    }

    type Exponents = Vec<i16>;
    type Collected = Vec<(Exponents, Atom)>;

    fn add_term(terms: &mut Collected, exponents: Exponents, coefficient: Atom) {
        if coefficient == Atom::num(0) {
            return;
        }
        if let Some((_, existing)) = terms
            .iter_mut()
            .find(|(existing, _)| existing == &exponents)
        {
            *existing = existing.clone() + coefficient;
            if existing == &Atom::num(0) {
                terms.retain(|(_, coefficient)| coefficient != &Atom::num(0));
            }
        } else {
            terms.push((exponents, coefficient));
        }
    }

    fn multiply(left: Collected, right: &Collected) -> Result<Collected, String> {
        let mut product = Vec::new();
        for (left_powers, left_coefficient) in left {
            for (right_powers, right_coefficient) in right {
                let powers = left_powers
                    .iter()
                    .zip(right_powers)
                    .map(|(left, right)| {
                        left.checked_add(*right).ok_or_else(|| {
                            "a collected exponent exceeds the supported i16 range".to_owned()
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                add_term(
                    &mut product,
                    powers,
                    left_coefficient.clone() * right_coefficient.clone(),
                );
            }
        }
        Ok(product)
    }

    fn positive_power(mut base: Collected, mut exponent: u32) -> Result<Collected, String> {
        let width = base.first().map_or(0, |(powers, _)| powers.len());
        let mut result = vec![(vec![0; width], Atom::num(1))];
        while exponent > 0 {
            if exponent & 1 == 1 {
                result = multiply(result, &base)?;
            }
            exponent >>= 1;
            if exponent > 0 {
                base = multiply(base.clone(), &base)?;
            }
        }
        Ok(result)
    }

    fn integer_exponent(value: AtomView<'_>) -> Option<i64> {
        let AtomView::Num(number) = value else {
            return None;
        };
        match number.get_coeff_view() {
            CoefficientView::Natural(numerator, 1, 0, 1) => Some(numerator),
            _ => None,
        }
    }

    fn collect_view(view: AtomView<'_>, variables: &[Atom]) -> Result<Collected, String> {
        let width = variables.len();
        if let Some(index) = variables
            .iter()
            .position(|variable| variable.as_view() == view)
        {
            let mut powers = vec![0; width];
            powers[index] = 1;
            return Ok(vec![(powers, Atom::num(1))]);
        }

        match view {
            AtomView::Add(sum) => {
                let mut collected = Vec::new();
                for term in sum {
                    for (powers, coefficient) in collect_view(term, variables)? {
                        add_term(&mut collected, powers, coefficient);
                    }
                }
                Ok(collected)
            }
            AtomView::Mul(product) => {
                let mut collected = vec![(vec![0; width], Atom::num(1))];
                for factor in product {
                    let factor = collect_view(factor, variables)?;
                    collected = multiply(collected, &factor)?;
                }
                Ok(collected)
            }
            AtomView::Pow(power) => {
                let (base, exponent) = power.get_base_exp();
                let Some(exponent) = integer_exponent(exponent) else {
                    return Ok(vec![(vec![0; width], view.to_owned())]);
                };

                if exponent < 0 {
                    if let Some(index) = variables
                        .iter()
                        .position(|variable| variable.as_view() == base)
                    {
                        let exponent = i16::try_from(exponent).map_err(|_| {
                            "a collected exponent exceeds the supported i16 range".to_owned()
                        })?;
                        let mut powers = vec![0; width];
                        powers[index] = exponent;
                        return Ok(vec![(powers, Atom::num(1))]);
                    }
                    return Ok(vec![(vec![0; width], view.to_owned())]);
                }

                let base = collect_view(base, variables)?;
                if base
                    .iter()
                    .all(|(powers, _)| powers.iter().all(|p| *p == 0))
                {
                    return Ok(vec![(vec![0; width], view.to_owned())]);
                }
                let exponent = u32::try_from(exponent)
                    .map_err(|_| "a collected exponent exceeds the supported range".to_owned())?;
                positive_power(base, exponent)
            }
            AtomView::Num(_) | AtomView::Var(_) | AtomView::Fun(_) => {
                Ok(vec![(vec![0; width], view.to_owned())])
            }
        }
    }

    Ok(collect_view(expr.as_view(), variables)?
        .into_iter()
        .map(|(exponents, coefficient)| {
            let key = exponents.into_iter().zip(variables).fold(
                Atom::num(1),
                |key, (exponent, variable)| {
                    if exponent == 0 {
                        key
                    } else {
                        key * variable.clone().pow(Atom::num(i64::from(exponent)))
                    }
                },
            );
            (key, coefficient)
        })
        .collect())
}

fn requested_binary_precision(decimal_precision: u32) -> u32 {
    (f64::from(decimal_precision) * std::f64::consts::LOG2_10).ceil() as u32
}

fn float_power_u64(mut base: Float, mut exponent: u64) -> Float {
    let mut result = Float::with_val(base.prec(), 1_i64);
    while exponent != 0 {
        if exponent & 1 == 1 {
            result *= &base;
        }
        exponent >>= 1;
        if exponent != 0 {
            base = base.clone() * &base;
        }
    }
    result
}

fn rounded_serialized_float(bytes: &[u8], decimal_precision: u32) -> Option<Float> {
    let value = serialized_float_value(bytes)?;
    let requested_precision = requested_binary_precision(decimal_precision);
    let target_precision = value.binary_precision.min(requested_precision).max(1);
    match value.kind {
        SerializedFloatKind::Zero => Some(Float::new(target_precision)),
        SerializedFloatKind::Finite {
            negative,
            mantissa,
            binary_exponent,
        } => {
            let work_precision = target_precision.max(128);
            let mut rounded;

            if value.binary_precision > requested_precision {
                // Round in base ten so that `decimal_precision` really denotes
                // significant decimal digits, including outside the f64 range.
                let (decimal_mantissa, decimal_exponent) =
                    exact_decimal_parts(value, requested_precision)?;
                let fractional_digits = decimal_mantissa
                    .split_once('.')
                    .map_or(0, |(_, fractional)| fractional.len());
                let significand = decimal_mantissa
                    .chars()
                    .filter(|character| *character != '.')
                    .collect::<String>()
                    .parse::<u64>()
                    .ok()?;
                let exponent =
                    decimal_exponent.checked_sub(i64::try_from(fractional_digits).ok()?)?;
                rounded = Float::with_val(work_precision, significand);
                let scale = float_power_u64(
                    Float::with_val(work_precision, 10_i64),
                    exponent.unsigned_abs(),
                );
                if exponent >= 0 {
                    rounded *= scale;
                } else {
                    rounded /= scale;
                }
            } else {
                // The no-GMP backend used by wasm32 does not reliably convert i128
                // values or exponentiate floats. Rebuild the serialized significand
                // from u64 limbs and use multiplication-based powers instead.
                let upper = (mantissa >> 64) as u64;
                let lower = mantissa as u64;
                rounded = Float::with_val(work_precision, upper);
                rounded *= float_power_u64(Float::with_val(work_precision, 2_i64), 64);
                rounded += &Float::with_val(work_precision, lower);
                let scale = float_power_u64(
                    Float::with_val(work_precision, 2_i64),
                    binary_exponent.unsigned_abs(),
                );
                if binary_exponent >= 0 {
                    rounded *= scale;
                } else {
                    rounded /= scale;
                }
            }
            rounded.set_prec(target_precision);
            Some(if negative { -rounded } else { rounded })
        }
        SerializedFloatKind::Infinite { .. } | SerializedFloatKind::Nan => None,
    }
}

fn float_coefficient(number: CoefficientView<'_>, decimal_precision: u32) -> Option<Coefficient> {
    let value = match number {
        CoefficientView::Float(real, imaginary) => {
            let requested_precision = requested_binary_precision(decimal_precision);
            let real_value = serialized_float_value(real.0)?;
            let imaginary_value = serialized_float_value(imaginary.0)?;
            if real_value.binary_precision <= requested_precision
                && imaginary_value.binary_precision <= requested_precision
            {
                return None;
            }
            Complex::new(
                rounded_serialized_float(real.0, decimal_precision)?,
                rounded_serialized_float(imaginary.0, decimal_precision)?,
            )
        }
        CoefficientView::Infinity(_) | CoefficientView::Indeterminate => return None,
        _ => number
            .to_float(requested_binary_precision(decimal_precision))
            .ok()?,
    };
    Some(Coefficient::Float(value))
}

fn contains_float(expr: AtomView<'_>) -> bool {
    let mut found = false;
    expr.visitor(&mut |view| {
        if matches!(
            view,
            AtomView::Num(number)
                if matches!(number.get_coeff_view(), CoefficientView::Float(_, _))
        ) {
            found = true;
        }
        !found
    });
    found
}

fn finite_nonzero(value: &Complex<f64>) -> bool {
    value.re.is_finite() && value.im.is_finite() && (value.re != 0.0 || value.im != 0.0)
}

fn serialized_float_to_f64(bytes: &[u8]) -> Option<f64> {
    let value = serialized_float_value(bytes)?;
    match value.kind {
        SerializedFloatKind::Zero => Some(0.0),
        SerializedFloatKind::Finite { negative, .. } => {
            let (mantissa, exponent) = exact_decimal_parts(value, value.binary_precision.min(54))?;
            let magnitude = format!("{mantissa}e{exponent}").parse::<f64>().ok()?;
            if !magnitude.is_finite() || magnitude == 0.0 {
                return None;
            }
            let value = if negative { -magnitude } else { magnitude };
            Some(value)
        }
        SerializedFloatKind::Infinite { .. } | SerializedFloatKind::Nan => None,
    }
}

fn safe_f64_inputs(expr: AtomView<'_>) -> bool {
    let mut safe = true;
    expr.visitor(&mut |view| {
        let AtomView::Num(number) = view else {
            return true;
        };
        let within_range = |value: f64| value.is_finite() && value.abs() <= 500.0;
        safe = match number.get_coeff_view() {
            CoefficientView::Natural(real, real_den, imaginary, imaginary_den) => {
                within_range(real as f64 / real_den as f64)
                    && within_range(imaginary as f64 / imaginary_den as f64)
            }
            CoefficientView::Large(real, imaginary) => {
                within_range(real.to_rat().to_f64()) && within_range(imaginary.to_rat().to_f64())
            }
            CoefficientView::Float(real, imaginary) => {
                serialized_float_to_f64(real.0).is_some_and(within_range)
                    && serialized_float_to_f64(imaginary.0).is_some_and(within_range)
            }
            CoefficientView::Infinity(_) | CoefficientView::Indeterminate => false,
            _ => true,
        };
        safe
    });
    safe
}

fn has_unsafe_constant_subexpression(expr: AtomView<'_>) -> bool {
    let mut unsafe_subexpression = false;
    expr.visitor(&mut |view| {
        if unsafe_subexpression {
            return false;
        }
        let AtomView::Fun(function) = view else {
            return true;
        };
        if function.get_symbol().get_evaluation_info().is_none() {
            return false;
        }
        if !safe_f64_inputs(view) {
            unsafe_subexpression = true;
            return false;
        }
        match evaluate_constant_f64(&view.to_owned()) {
            Ok(value) if finite_nonzero(&value) => false,
            Ok(_) => {
                unsafe_subexpression = true;
                false
            }
            Err(_) => true,
        }
    });
    unsafe_subexpression
}

fn float_approximation(expr: &Atom, decimal_precision: u32) -> Atom {
    let precision = requested_binary_precision(decimal_precision);
    let to_float_atom = |value: Complex<f64>| {
        let round = |value: f64| {
            format!(
                "{:.*e}",
                decimal_precision.saturating_sub(1) as usize,
                value
            )
            .parse::<f64>()
            .unwrap_or(value)
        };
        Atom::num(Coefficient::Float(Complex::new(
            Float::with_val(precision, round(value.re)),
            Float::with_val(precision, round(value.im)),
        )))
    };
    if let AtomView::Num(number) = expr.as_view() {
        return float_coefficient(number.get_coeff_view(), decimal_precision)
            .map(Atom::num)
            .unwrap_or_else(|| expr.clone());
    }
    let has_float = contains_float(expr.as_view());
    if has_unsafe_constant_subexpression(expr.as_view()) {
        return expr.clone();
    }
    if !has_float
        && safe_f64_inputs(expr.as_view())
        && let Ok(value) = evaluate_constant_f64(expr)
    {
        if finite_nonzero(&value) {
            return to_float_atom(value);
        }
        return expr.clone();
    }

    expr.replace_map(|view, context, out| {
        if context.parent_type == Some(symbolica::atom::AtomType::Pow) && context.index == 1 {
            **out = view.to_owned();
            return;
        }
        if let AtomView::Fun(function) = view
            && function.get_symbol().get_evaluation_info().is_none()
        {
            **out = view.to_owned();
            return;
        }
        if let AtomView::Num(number) = view {
            if let Some(value) = float_coefficient(number.get_coeff_view(), decimal_precision) {
                **out = Atom::num(value);
            } else {
                **out = view.to_owned();
            }
            return;
        }
        if contains_float(view) {
            return;
        }
        if safe_f64_inputs(view)
            && let Ok(value) = evaluate_constant_f64(&view.to_owned())
            && finite_nonzero(&value)
        {
            **out = to_float_atom(value);
        }
    })
}

fn evaluate_constant_f64(expr: &Atom) -> Result<Complex<f64>, String> {
    if contains_float(expr.as_view()) {
        return Err(
            "constant evaluation of floating-point coefficients is unavailable in this WebAssembly build"
                .to_owned(),
        );
    }
    let expressions = [expr.clone()];
    let variables: [Atom; 0] = [];
    let mut evaluator = Atom::evaluator_multiple(&expressions, &variables)
        .build()
        .map_err(|err| format!("could not build constant evaluator: {err}"))?
        .map_coeff(&|value| Complex::new(value.re.to_f64(), value.im.to_f64()));
    let mut output = [Complex::new(0.0, 0.0)];
    evaluator.evaluate(&[], &mut output);
    Ok(output[0].clone())
}

fn complex_cbor(value: &Complex<f64>) -> Value {
    Value::Map(vec![
        (Value::Text("re".to_owned()), Value::Float(value.re)),
        (Value::Text("im".to_owned()), Value::Float(value.im)),
    ])
}

fn complex_rows_cbor(rows: &[Vec<Complex<f64>>]) -> Value {
    Value::Array(
        rows.iter()
            .map(|row| Value::Array(row.iter().map(complex_cbor).collect()))
            .collect(),
    )
}

fn real_rows_cbor(rows: &[Vec<f64>]) -> Value {
    Value::Array(
        rows.iter()
            .map(|row| Value::Array(row.iter().copied().map(Value::Float).collect()))
            .collect(),
    )
}

fn complex_evaluator(
    map: &[(Value, Value)],
) -> Result<(ExpressionEvaluator<Complex<f64>>, usize), String> {
    let expressions = atom_list(
        map_get(map, "expressions").ok_or_else(|| "missing expressions".to_owned())?,
        "expressions",
    )?;
    if expressions.is_empty() {
        return Err("expressions must not be empty".to_owned());
    }

    let variables = atom_list(
        map_get(map, "variables").ok_or_else(|| "missing variables".to_owned())?,
        "variables",
    )?;
    let output_count = expressions.len();
    let evaluator = Atom::evaluator_multiple(&expressions, &variables)
        .build()
        .map_err(|err| format!("could not build evaluator: {err}"))?
        .map_coeff(&|value| Complex::new(value.re.to_f64(), value.im.to_f64()));
    Ok((evaluator, output_count))
}

fn evaluate_point(
    evaluator: &mut ExpressionEvaluator<Complex<f64>>,
    output_count: usize,
    value: &Value,
    label: &str,
) -> Result<(Vec<Complex<f64>>, Vec<Complex<f64>>), String> {
    let Value::Array(values) = value else {
        return Err(format!("{label} must be an array"));
    };
    if values.len() != evaluator.get_input_len() {
        return Err(format!(
            "{label} has {} values, expected {}",
            values.len(),
            evaluator.get_input_len()
        ));
    }
    let inputs = values
        .iter()
        .enumerate()
        .map(|(index, value)| value_complex(value, &format!("{label}[{index}]")))
        .collect::<Result<Vec<_>, _>>()?;
    let mut outputs = vec![Complex::new(0.0, 0.0); output_count];
    evaluator.evaluate(&inputs, &mut outputs);
    Ok((inputs, outputs))
}

#[derive(Clone, Copy)]
struct GridDomain {
    min: f64,
    max: f64,
    samples: usize,
}

impl GridDomain {
    fn sample(self, index: usize) -> f64 {
        if self.samples == 1 {
            self.min
        } else if index + 1 == self.samples {
            self.max
        } else {
            self.min + (self.max - self.min) * index as f64 / (self.samples - 1) as f64
        }
    }
}

fn grid_domains(value: &Value) -> Result<Vec<GridDomain>, String> {
    let Value::Array(values) = value else {
        return Err("domains must be an array".to_owned());
    };
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let Value::Map(map) = value else {
                return Err(format!("domains[{index}] must be a dictionary"));
            };
            let min = value_f64(
                map_get(map, "min").ok_or_else(|| format!("domains[{index}] missing min"))?,
                &format!("domains[{index}].min"),
            )?;
            let max = value_f64(
                map_get(map, "max").ok_or_else(|| format!("domains[{index}] missing max"))?,
                &format!("domains[{index}].max"),
            )?;
            let samples = map_usize(map, "samples", 200)?;
            if !min.is_finite() || !max.is_finite() || min >= max {
                return Err(format!(
                    "domains[{index}] requires finite min less than max"
                ));
            }
            if samples == 0 {
                return Err(format!("domains[{index}].samples must be positive"));
            }
            Ok(GridDomain { min, max, samples })
        })
        .collect()
}

#[wasm_func]
pub fn from_ast(ast: &[u8], namespace: &[u8]) -> Result<Vec<u8>, String> {
    let namespace = match decode_cbor(namespace, "namespace")? {
        Value::Text(namespace) => namespace,
        other => return Err(format!("namespace must be text, got {other:?}")),
    };
    encode_atom(&atom_from_ast(ast, &namespace, "ast")?)
}

#[wasm_func]
pub fn symbol(name: &[u8], namespace: &[u8]) -> Result<Vec<u8>, String> {
    let name = match decode_cbor(name, "name")? {
        Value::Text(name) => name,
        other => return Err(format!("name must be text, got {other:?}")),
    };
    let namespace = match decode_cbor(namespace, "namespace")? {
        Value::Text(namespace) => namespace,
        other => return Err(format!("namespace must be text, got {other:?}")),
    };
    encode_atom(&symbol_atom(&name, &namespace)?)
}

#[wasm_func]
pub fn canonical(payload: &[u8], namespaces: &[u8]) -> Result<Vec<u8>, String> {
    render_payload_symbolica(payload, cbor_bool(namespaces, "namespaces")?)
}

#[wasm_func]
pub fn to_typst(payload: &[u8]) -> Result<Vec<u8>, String> {
    render_payload_typst(payload)
}

#[wasm_func]
pub fn to_latex(expr: &[u8]) -> Result<Vec<u8>, String> {
    if is_matrix_payload(expr) {
        let matrix = decode_matrix(expr, "matrix")?;
        Ok(matrix
            .format_string(&PrintOptions::latex(), PrintState::new())
            .into_bytes())
    } else {
        let expr = decode_atom(expr, "expr")?;
        Ok(render_atom(
            &expr,
            PrintOptions::latex(),
            FloatRenderStyle::Latex,
        ))
    }
}

#[wasm_func]
pub fn simplify_expr(expr: &[u8]) -> Result<Vec<u8>, String> {
    let expr = decode_atom(expr, "expr")?;
    encode_atom(&expr)
}

#[wasm_func]
pub fn expand(expr: &[u8]) -> Result<Vec<u8>, String> {
    let expr = decode_atom(expr, "expr")?.expand_via_poly::<u16, Atom>(None);
    encode_atom(&expr)
}

#[wasm_func]
pub fn together(expr: &[u8]) -> Result<Vec<u8>, String> {
    encode_atom(&decode_atom(expr, "expr")?.together())
}

#[wasm_func]
pub fn cancel(expr: &[u8]) -> Result<Vec<u8>, String> {
    encode_atom(&decode_atom(expr, "expr")?.cancel())
}

#[wasm_func]
pub fn apart(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "apart request")? else {
        return Err("apart request must be dictionary".to_owned());
    };
    let expr = decode_atom(map_bytes(&map, "expr")?, "expr")?;
    let result = expr.apart(indeterminate_from_bytes(map_bytes(&map, "var")?, "var")?);
    encode_atom(&result)
}

#[wasm_func]
pub fn collect(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "collect request")? else {
        return Err("collect request must be dictionary".to_owned());
    };
    let expr = decode_atom(map_bytes(&map, "expr")?, "expr")?;
    let variables = atom_list(
        map_get(&map, "variables").ok_or_else(|| "missing variables".to_owned())?,
        "variables",
    )?;
    let result = collected_coefficients(&expr, &variables)?
        .into_iter()
        .fold(Atom::num(0), |sum, (key, coefficient)| {
            sum + key * coefficient
        });
    encode_atom(&result)
}

#[wasm_func]
pub fn coefficient(expr: &[u8], monomial: &[u8]) -> Result<Vec<u8>, String> {
    encode_atom(&decode_atom(expr, "expr")?.coefficient(decode_atom(monomial, "monomial")?))
}

#[wasm_func]
pub fn coefficient_list(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "coefficient-list request")? else {
        return Err("coefficient-list request must be dictionary".to_owned());
    };
    let expr = decode_atom(map_bytes(&map, "expr")?, "expr")?;
    let variables = atom_list(
        map_get(&map, "variables").ok_or_else(|| "missing variables".to_owned())?,
        "variables",
    )?;
    let pairs = collected_coefficients(&expr, &variables)?
        .into_iter()
        .map(|(key, coefficient)| {
            Ok(Value::Array(vec![
                Value::Bytes(encode_atom(&key)?),
                Value::Bytes(encode_atom(&coefficient)?),
            ]))
        })
        .collect::<Result<Vec<_>, String>>()?;
    encode_cbor(Value::Array(pairs))
}

#[wasm_func]
pub fn terms(expr: &[u8]) -> Result<Vec<u8>, String> {
    cbor_atom_array(
        decode_atom(expr, "expr")?
            .terms()
            .map(|value| value.to_owned())
            .collect(),
    )
}

#[wasm_func]
pub fn indeterminates(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "indeterminates request")? else {
        return Err("indeterminates request must be dictionary".to_owned());
    };
    let expr = decode_atom(map_bytes(&map, "expr")?, "expr")?;
    let mut values = expr
        .get_all_indeterminates(map_bool(&map, "enter-functions", true)?)
        .into_iter()
        .map(|value| value.to_owned())
        .collect::<Vec<_>>();
    values.sort();
    cbor_atom_array(values)
}

#[wasm_func]
pub fn contains(expr: &[u8], subexpression: &[u8]) -> Result<Vec<u8>, String> {
    encode_cbor(Value::Bool(
        decode_atom(expr, "expr")?.contains(decode_atom(subexpression, "subexpression")?),
    ))
}

#[wasm_func]
pub fn is_constant(expr: &[u8]) -> Result<Vec<u8>, String> {
    encode_cbor(Value::Bool(decode_atom(expr, "expr")?.is_constant()))
}

#[wasm_func]
pub fn to_float(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "to-float request")? else {
        return Err("to-float request must be dictionary".to_owned());
    };
    let decimal_precision = map_usize(&map, "decimal-prec", 16)?;
    if !(1..=16).contains(&decimal_precision) {
        return Err("decimal-prec must be between 1 and 16 in the WebAssembly build".to_owned());
    }
    let expr = decode_atom(map_bytes(&map, "expr")?, "expr")?;
    encode_atom(&float_approximation(&expr, decimal_precision as u32))
}

#[wasm_func]
pub fn factor(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "factor request")? else {
        return Err("factor request must be dictionary".to_owned());
    };
    let expr = decode_atom(map_bytes(&map, "expr")?, "expr")?;
    let complex = map_bool(&map, "complex", false)?;
    let square_free = map_bool(&map, "square-free", false)?;
    if complex && square_free {
        return Err("complex and square-free factorization cannot be combined".to_owned());
    }
    let expr = if square_free {
        expr.as_view().factor_square_free()
    } else if complex {
        expr.factor_complex()
    } else {
        expr.factor()
    };
    encode_atom(&expr)
}

#[wasm_func]
pub fn derivative(expr: &[u8], var: &[u8]) -> Result<Vec<u8>, String> {
    let expr = decode_atom(expr, "expr")?;
    let var = Indeterminate::try_from(decode_atom(var, "var")?)?;
    encode_atom(&expr.derivative(var))
}

fn integrate_polynomial_term(
    term: &Atom,
    variables: Arc<Vec<PolyVariable>>,
) -> Result<Atom, String> {
    let rational = term
        .try_to_rational_polynomial::<_, _, u16>(&Q, &Z, Some(variables))
        .map_err(|err| format!("expression must be a rational function of the variable: {err}"))?;
    if !rational.denominator.is_one() {
        return Err("integration currently supports polynomial expressions only".to_owned());
    }

    let integral = rational.integrate(0);
    let mut parts = integral
        .rational_parts
        .into_iter()
        .map(|part| part.to_expression())
        .collect::<Vec<_>>();
    parts.extend(
        integral
            .logarithmic_parts
            .into_iter()
            .map(|part| part.coefficient.to_expression() * part.argument.to_expression().log()),
    );
    Ok(Atom::add_many(parts))
}

fn rational_integral_atoms(expr: Atom, var_atom: Atom) -> Result<(Atom, Vec<Atom>), String> {
    let variable = PolyVariable::from(
        Indeterminate::try_from(var_atom)
            .map_err(|err| format!("integration variable must be a variable: {err}"))?,
    );
    let variables = Arc::new(vec![variable]);
    let expanded = expr.expand_via_poly::<u16, Atom>(None);
    let input_terms = expanded
        .as_add_view()
        .map(|sum| sum.into_iter().map(Atom::from).collect::<Vec<_>>())
        .unwrap_or_else(|| vec![expanded]);
    let steps = input_terms
        .iter()
        .map(|term| integrate_polynomial_term(term, variables.clone()))
        .collect::<Result<Vec<_>, _>>()?;
    Ok((Atom::add_many(steps.clone()), steps))
}

fn rational_integral(expr: &[u8], var: &[u8]) -> Result<(Atom, Vec<Atom>), String> {
    rational_integral_atoms(decode_atom(expr, "expr")?, decode_atom(var, "var")?)
}

#[wasm_func]
pub fn integrate(expr: &[u8], var: &[u8]) -> Result<Vec<u8>, String> {
    encode_atom(&rational_integral(expr, var)?.0)
}

#[wasm_func]
pub fn integrate_with_steps(expr: &[u8], var: &[u8]) -> Result<Vec<u8>, String> {
    let (result, steps) = rational_integral(expr, var)?;
    encode_cbor(Value::Map(vec![
        (
            Value::Text("result".to_owned()),
            Value::Bytes(encode_atom(&result)?),
        ),
        (Value::Text("steps".to_owned()), atoms_cbor_value(steps)?),
    ]))
}

#[wasm_func]
pub fn series(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "series request")? else {
        return Err("series request must be dictionary".to_owned());
    };
    let expr = decode_atom(map_bytes(&map, "expr")?, "expr")?;
    let var = indeterminate_from_bytes(map_bytes(&map, "var")?, "var")?;
    let expansion_point = decode_atom(map_bytes(&map, "expansion-point")?, "expansion-point")?;
    let depth = map_i64(&map, "depth", 0)?;
    let depth_denom = map_i64(&map, "depth-denom", 1)?;
    let depth = if map_bool(&map, "depth-is-absolute", true)? {
        SeriesDepth::absolute((depth, depth_denom))
    } else {
        SeriesDepth::relative((depth, depth_denom))
    };
    encode_atom(
        &expr
            .series(var, expansion_point.as_view(), depth)
            .map_err(|err| err.to_string())?
            .to_atom(),
    )
}

#[wasm_func]
pub fn replace(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "replace request")? else {
        return Err("replace request must be dictionary".to_owned());
    };
    let expr = decode_atom(map_bytes(&map, "expr")?, "expr")?;
    let replacement = build_replacement(&map)?;
    let settings = build_replace_settings(&map)?;
    let repeat = map_bool(&map, "repeat", false)?;
    encode_atom(&repeat_replacements(
        expr,
        vec![replacement],
        settings,
        repeat,
    ))
}

#[wasm_func]
pub fn replace_multiple(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "replace-multiple request")? else {
        return Err("replace-multiple request must be dictionary".to_owned());
    };
    let expr = decode_atom(map_bytes(&map, "expr")?, "expr")?;
    let Value::Array(rules) = map_get(&map, "rules").ok_or_else(|| "missing rules".to_owned())?
    else {
        return Err("rules must be array".to_owned());
    };
    let replacements = rules
        .iter()
        .enumerate()
        .map(|(index, rule)| {
            let Value::Map(rule) = rule else {
                return Err(format!("rules[{index}] must be dictionary"));
            };
            build_replacement(rule)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let settings = build_replace_settings(&map)?;
    let repeat = map_bool(&map, "repeat", false)?;
    encode_atom(&repeat_replacements(expr, replacements, settings, repeat))
}

#[wasm_func]
pub fn replace_wildcards(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "replace-wildcards request")? else {
        return Err("replace-wildcards request must be dictionary".to_owned());
    };
    let pattern = decode_atom(map_bytes(&map, "pattern")?, "pattern")?.to_pattern();
    let Value::Array(replacements) =
        map_get(&map, "replacements").ok_or_else(|| "missing replacements".to_owned())?
    else {
        return Err("replacements must be array".to_owned());
    };
    let mut map = HashMap::default();
    for (index, replacement) in replacements.iter().enumerate() {
        let Value::Array(pair) = replacement else {
            return Err(format!("replacements[{index}] must be pair"));
        };
        if pair.len() != 2 {
            return Err(format!("replacements[{index}] must have two entries"));
        }
        let wildcard = symbol_from_atom_bytes(value_atom_bytes(&pair[0], "wildcard")?, "wildcard")?;
        if wildcard.get_wildcard_level() == 0 {
            return Err("only wildcards can be replaced".to_owned());
        }
        map.insert(
            wildcard,
            decode_atom(value_atom_bytes(&pair[1], "replacement")?, "replacement")?,
        );
    }
    encode_atom(
        &pattern
            .replace_wildcards(&map)
            .map_err(|err| err.to_string())?,
    )
}

#[wasm_func]
pub fn evaluate(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "evaluate request")? else {
        return Err("evaluate request must be dictionary".to_owned());
    };
    let expr = decode_atom(map_bytes(&map, "expr")?, "expr")?;
    let mut constants = HashMap::default();
    if let Some(Value::Array(values)) = map_get(&map, "values") {
        for (index, value) in values.iter().enumerate() {
            let Value::Array(pair) = value else {
                return Err(format!("values[{index}] must be pair"));
            };
            if pair.len() != 2 {
                return Err(format!("values[{index}] must have two entries"));
            }
            constants.insert(
                decode_atom(value_atom_bytes(&pair[0], "value key")?, "value key")?,
                value_complex(&pair[1], "value")?,
            );
        }
    }
    let result = expr
        .evaluate(&constants)
        .map_err(|err| format!("could not evaluate expression: {err}"))?;
    encode_cbor(Value::Map(vec![
        (Value::Text("re".to_owned()), Value::Float(result.re)),
        (Value::Text("im".to_owned()), Value::Float(result.im)),
    ]))
}

#[wasm_func]
pub fn evaluate_many(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "evaluate-many request")? else {
        return Err("evaluate-many request must be dictionary".to_owned());
    };
    let (mut evaluator, output_count) = complex_evaluator(&map)?;
    let Value::Array(points) =
        map_get(&map, "points").ok_or_else(|| "missing points".to_owned())?
    else {
        return Err("points must be an array".to_owned());
    };
    let outputs = points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            evaluate_point(
                &mut evaluator,
                output_count,
                point,
                &format!("points[{index}]"),
            )
            .map(|(_, outputs)| outputs)
        })
        .collect::<Result<Vec<_>, _>>()?;
    encode_cbor(complex_rows_cbor(&outputs))
}

#[wasm_func]
pub fn evaluate_grid(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "evaluate-grid request")? else {
        return Err("evaluate-grid request must be dictionary".to_owned());
    };
    let (mut evaluator, output_count) = complex_evaluator(&map)?;
    let domains =
        grid_domains(map_get(&map, "domains").ok_or_else(|| "missing domains".to_owned())?)?;
    if domains.len() != evaluator.get_input_len() {
        return Err(format!(
            "domains has {} entries, expected {}",
            domains.len(),
            evaluator.get_input_len()
        ));
    }
    let total = domains.iter().try_fold(1usize, |total, domain| {
        total
            .checked_mul(domain.samples)
            .ok_or_else(|| "grid sample count overflows usize".to_owned())
    })?;

    let mut indices = vec![0usize; domains.len()];
    let mut points = Vec::with_capacity(total);
    let mut outputs = Vec::with_capacity(total);
    for _ in 0..total {
        let point = domains
            .iter()
            .zip(&indices)
            .map(|(domain, index)| domain.sample(*index))
            .collect::<Vec<_>>();
        let inputs = point
            .iter()
            .map(|value| Complex::new(*value, 0.0))
            .collect::<Vec<_>>();
        let mut output = vec![Complex::new(0.0, 0.0); output_count];
        evaluator.evaluate(&inputs, &mut output);
        points.push(point);
        outputs.push(output);

        for axis in (0..indices.len()).rev() {
            indices[axis] += 1;
            if indices[axis] < domains[axis].samples {
                break;
            }
            indices[axis] = 0;
        }
    }

    encode_cbor(Value::Map(vec![
        (
            Value::Text("shape".to_owned()),
            Value::Array(
                domains
                    .iter()
                    .map(|domain| Value::Integer((domain.samples as i64).into()))
                    .collect(),
            ),
        ),
        (Value::Text("points".to_owned()), real_rows_cbor(&points)),
        (
            Value::Text("values".to_owned()),
            complex_rows_cbor(&outputs),
        ),
    ]))
}

#[wasm_func]
pub fn solve_linear(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "solve-linear request")? else {
        return Err("solve-linear request must be dictionary".to_owned());
    };
    let system = match map_get(&map, "system") {
        Some(Value::Array(values)) => values
            .iter()
            .enumerate()
            .map(|(index, value)| atom_from_cbor_value(value, &format!("system[{index}]")))
            .collect::<Result<Vec<_>, _>>()?,
        Some(Value::Bytes(bytes)) if is_matrix_payload(bytes) => {
            let matrix = decode_matrix(bytes, "system")?;
            matrix_entries_to_atoms(&matrix)
        }
        Some(other) => {
            return Err(format!(
                "system must be array or vector matrix, got {other:?}"
            ));
        }
        None => return Err("missing system".to_owned()),
    };
    let vars = match map_get(&map, "variables") {
        Some(Value::Array(values)) => values
            .iter()
            .enumerate()
            .map(|(index, value)| atom_from_cbor_value(value, &format!("variables[{index}]")))
            .collect::<Result<Vec<_>, _>>()?,
        Some(other) => return Err(format!("variables must be array, got {other:?}")),
        None => return Err("missing variables".to_owned()),
    };
    let result = match AtomView::solve_linear_system::<u16, _, Atom>(&system, &vars) {
        Ok(result) => result,
        Err(SolveError::Underdetermined {
            partial_solution, ..
        }) => partial_solution,
        Err(err) => return Err(err.to_string()),
    };
    cbor_atom_array(result)
}

#[wasm_func]
pub fn solve_system(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "solve-system request")? else {
        return Err("solve-system request must be dictionary".to_owned());
    };
    let system = map_array(&map, "system")?
        .iter()
        .enumerate()
        .map(|(index, value)| atom_from_cbor_value(value, &format!("system[{index}]")))
        .collect::<Result<Vec<_>, _>>()?;
    let variables = map_array(&map, "variables")?
        .iter()
        .enumerate()
        .map(|(index, value)| atom_from_cbor_value(value, &format!("variables[{index}]")))
        .collect::<Result<Vec<_>, _>>()?;
    let keys = variables
        .iter()
        .map(|variable| {
            Indeterminate::try_from(variable.clone())
                .map(PolyVariable::from)
                .map_err(|err| format!("solve variable must be a variable: {err}"))
        })
        .collect::<Result<Vec<PolyVariable>, _>>()?;
    let solutions = AtomView::solve::<u16, _, Atom>(&system, &variables)
        .map_err(|err| format!("could not solve system: {err}"))?;
    let rows = solutions
        .into_iter()
        .map(|solution| {
            keys.iter()
                .map(|key| {
                    solution
                        .get(key)
                        .cloned()
                        .ok_or_else(|| "solver omitted a requested variable".to_owned())
                })
                .collect::<Result<Vec<_>, _>>()
                .and_then(atoms_cbor_value)
        })
        .collect::<Result<Vec<_>, _>>()?;
    encode_cbor(Value::Array(rows))
}

#[wasm_func]
pub fn nsolve(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "nsolve request")? else {
        return Err("nsolve request must be dictionary".to_owned());
    };
    let expr = decode_atom(map_bytes(&map, "expr")?, "expr")?;
    let var = indeterminate_from_bytes(map_bytes(&map, "var")?, "var")?;
    let init = map_f64(&map, "init", 1.0)?;
    let prec = map_f64(&map, "prec", 1e-4)?;
    let max_iterations = map_usize(&map, "max-iterations", 1000)?;
    let result: F64 = expr
        .nsolve::<F64, _>(var, init.into(), prec.into(), max_iterations)
        .map_err(|err| format!("could not solve expression: {err}"))?;
    cbor_f64(result.into_inner())
}

#[wasm_func]
pub fn nsolve_system(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "nsolve-system request")? else {
        return Err("nsolve-system request must be dictionary".to_owned());
    };
    let Value::Array(system_values) =
        map_get(&map, "system").ok_or_else(|| "missing system".to_owned())?
    else {
        return Err("system must be array".to_owned());
    };
    let system = system_values
        .iter()
        .enumerate()
        .map(|(index, value)| atom_from_cbor_value(value, &format!("system[{index}]")))
        .collect::<Result<Vec<_>, _>>()?;
    let system_views = system.iter().map(|atom| atom.as_view()).collect::<Vec<_>>();

    let Value::Array(variable_values) =
        map_get(&map, "variables").ok_or_else(|| "missing variables".to_owned())?
    else {
        return Err("variables must be array".to_owned());
    };
    let vars = variable_values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            indeterminate_from_bytes(
                value_atom_bytes(value, "variable")?,
                &format!("variables[{index}]"),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    let Value::Array(init_values) =
        map_get(&map, "init").ok_or_else(|| "missing init".to_owned())?
    else {
        return Err("init must be array".to_owned());
    };
    let init = init_values
        .iter()
        .enumerate()
        .map(|(index, value)| value_f64(value, &format!("init[{index}]")).map(F64::from))
        .collect::<Result<Vec<_>, _>>()?;

    let prec = map_f64(&map, "prec", 1e-4)?;
    let max_iterations = map_usize(&map, "max-iterations", 1000)?;
    let result: Vec<F64> =
        AtomView::nsolve_system(&system_views, &vars, &init, prec.into(), max_iterations)
            .map_err(|err| format!("could not solve system: {err}"))?;
    cbor_f64_array(result.into_iter().map(F64::into_inner).collect())
}

#[wasm_func]
pub fn add(args: &[u8]) -> Result<Vec<u8>, String> {
    let args = decode_atom_array(args, "args")?;
    encode_atom(&Atom::add_many(args))
}

#[wasm_func]
pub fn mul(args: &[u8]) -> Result<Vec<u8>, String> {
    let args = decode_atom_array(args, "args")?;
    encode_atom(&Atom::mul_many(args))
}

#[wasm_func]
pub fn neg(expr: &[u8]) -> Result<Vec<u8>, String> {
    encode_atom(&(-decode_atom(expr, "expr")?))
}

#[wasm_func]
pub fn sub(lhs: &[u8], rhs: &[u8]) -> Result<Vec<u8>, String> {
    encode_atom(&(decode_atom(lhs, "lhs")? - decode_atom(rhs, "rhs")?))
}

#[wasm_func]
pub fn div(lhs: &[u8], rhs: &[u8]) -> Result<Vec<u8>, String> {
    encode_atom(&(decode_atom(lhs, "lhs")? / decode_atom(rhs, "rhs")?))
}

#[wasm_func]
pub fn power(base: &[u8], exp: &[u8]) -> Result<Vec<u8>, String> {
    encode_atom(&decode_atom(base, "base")?.pow(decode_atom(exp, "exp")?))
}

#[wasm_func]
pub fn matrix_from_nested(input: &[u8]) -> Result<Vec<u8>, String> {
    encode_matrix(&decode_nested_matrix(input)?)
}

#[wasm_func]
pub fn matrix_vec(values: &[u8]) -> Result<Vec<u8>, String> {
    encode_matrix(&matrix_vec_from_values(values)?)
}

#[wasm_func]
pub fn matrix_identity(n: &[u8]) -> Result<Vec<u8>, String> {
    let n = cbor_usize(n, "n")?;
    if n == 0 {
        return Err("identity matrix must be non-empty".to_owned());
    }
    encode_matrix(&Matrix::identity(n as u32, RationalPolynomialField::new(Z)))
}

#[wasm_func]
pub fn matrix_eye(diag: &[u8]) -> Result<Vec<u8>, String> {
    encode_matrix(&matrix_from_diag(diag)?)
}

#[wasm_func]
pub fn matrix_add(lhs: &[u8], rhs: &[u8]) -> Result<Vec<u8>, String> {
    let (lhs, rhs) = unify_matrices(&decode_matrix(lhs, "lhs")?, &decode_matrix(rhs, "rhs")?);
    encode_matrix(&(&lhs + &rhs))
}

#[wasm_func]
pub fn matrix_sub(lhs: &[u8], rhs: &[u8]) -> Result<Vec<u8>, String> {
    let (lhs, rhs) = unify_matrices(&decode_matrix(lhs, "lhs")?, &decode_matrix(rhs, "rhs")?);
    encode_matrix(&(&lhs - &rhs))
}

#[wasm_func]
pub fn matrix_mul(lhs: &[u8], rhs: &[u8]) -> Result<Vec<u8>, String> {
    let lhs = decode_matrix(lhs, "lhs")?;
    if is_matrix_payload(rhs) {
        let (lhs, rhs) = unify_matrices(&lhs, &decode_matrix(rhs, "rhs")?);
        encode_matrix(&(&lhs * &rhs))
    } else {
        let scalar = atom_to_matrix_entry(&decode_atom(rhs, "rhs")?)?;
        let (lhs, scalar) = unify_matrix_scalar(&lhs, scalar);
        encode_matrix(&lhs.mul_scalar(&scalar))
    }
}

#[wasm_func]
pub fn matrix_div_scalar(lhs: &[u8], rhs: &[u8]) -> Result<Vec<u8>, String> {
    let lhs = decode_matrix(lhs, "lhs")?;
    let scalar = atom_to_matrix_entry(&decode_atom(rhs, "rhs")?)?;
    let (lhs, scalar) = unify_matrix_scalar(&lhs, scalar);
    if scalar.is_zero() {
        return Err("cannot divide a matrix by zero".to_owned());
    }
    encode_matrix(&lhs.div_scalar(&scalar))
}

#[wasm_func]
pub fn transpose(matrix: &[u8]) -> Result<Vec<u8>, String> {
    encode_matrix(&decode_matrix(matrix, "matrix")?.transpose())
}

#[wasm_func]
pub fn det(matrix: &[u8]) -> Result<Vec<u8>, String> {
    let det = decode_matrix(matrix, "matrix")?
        .det()
        .map_err(|err| err.to_string())?;
    encode_atom(&det.to_expression())
}

#[wasm_func]
pub fn inv(matrix: &[u8]) -> Result<Vec<u8>, String> {
    encode_matrix(
        &decode_matrix(matrix, "matrix")?
            .inv()
            .map_err(|err| err.to_string())?,
    )
}

#[wasm_func]
pub fn matrix_solve(lhs: &[u8], rhs: &[u8]) -> Result<Vec<u8>, String> {
    let (lhs, rhs) = unify_matrices(&decode_matrix(lhs, "lhs")?, &decode_matrix(rhs, "rhs")?);
    encode_matrix(&lhs.solve(&rhs).map_err(|err| err.to_string())?)
}

#[wasm_func]
pub fn matrix_solve_any(lhs: &[u8], rhs: &[u8]) -> Result<Vec<u8>, String> {
    let (lhs, rhs) = unify_matrices(&decode_matrix(lhs, "lhs")?, &decode_matrix(rhs, "rhs")?);
    encode_matrix(&lhs.solve_any(&rhs).map_err(|err| err.to_string())?)
}

#[wasm_func]
pub fn row_reduce(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "row-reduce request")? else {
        return Err("row-reduce request must be dictionary".to_owned());
    };
    let mut matrix = decode_matrix(map_bytes(&map, "matrix")?, "matrix")?;
    let max_col = map_usize(&map, "max-col", matrix.ncols())? as u32;
    let rank = matrix.row_reduce(max_col);
    encode_cbor(Value::Map(vec![
        (
            Value::Text("matrix".to_owned()),
            Value::Bytes(encode_matrix(&matrix)?),
        ),
        (
            Value::Text("rank".to_owned()),
            Value::Integer((rank as i64).into()),
        ),
    ]))
}

#[wasm_func]
pub fn augment(lhs: &[u8], rhs: &[u8]) -> Result<Vec<u8>, String> {
    let (lhs, rhs) = unify_matrices(&decode_matrix(lhs, "lhs")?, &decode_matrix(rhs, "rhs")?);
    encode_matrix(&lhs.augment(&rhs).map_err(|err| err.to_string())?)
}

#[wasm_func]
pub fn split_col(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "split-col request")? else {
        return Err("split-col request must be dictionary".to_owned());
    };
    let matrix = decode_matrix(map_bytes(&map, "matrix")?, "matrix")?;
    let index = map_usize(&map, "index", 0)? as u32;
    let (lhs, rhs) = matrix.split_col(index).map_err(|err| err.to_string())?;
    encode_cbor(Value::Array(vec![
        Value::Bytes(encode_matrix(&lhs)?),
        Value::Bytes(encode_matrix(&rhs)?),
    ]))
}

#[wasm_func]
pub fn primitive_part(matrix: &[u8]) -> Result<Vec<u8>, String> {
    encode_matrix(&decode_matrix(matrix, "matrix")?.primitive_part())
}

#[wasm_func]
pub fn content(matrix: &[u8]) -> Result<Vec<u8>, String> {
    let content = decode_matrix(matrix, "matrix")?.content();
    encode_atom(&content.to_expression())
}

#[wasm_func]
pub fn matrix_at(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "matrix-at request")? else {
        return Err("matrix-at request must be dictionary".to_owned());
    };
    let matrix = decode_matrix(map_bytes(&map, "matrix")?, "matrix")?;
    let row = map_usize(&map, "row", 0)?;
    let col = map_usize(&map, "col", 0)?;
    if row >= matrix.nrows() || col >= matrix.ncols() {
        return Err("matrix index out of bounds".to_owned());
    }
    encode_atom(&matrix[(row as u32, col as u32)].clone().to_expression())
}

#[wasm_func]
pub fn matrix_shape(matrix: &[u8]) -> Result<Vec<u8>, String> {
    let matrix = decode_matrix(matrix, "matrix")?;
    encode_cbor(Value::Array(vec![
        Value::Integer((matrix.nrows() as i64).into()),
        Value::Integer((matrix.ncols() as i64).into()),
    ]))
}

#[wasm_func]
pub fn matrix_is_zero(matrix: &[u8]) -> Result<Vec<u8>, String> {
    encode_cbor(Value::Bool(decode_matrix(matrix, "matrix")?.is_zero()))
}

#[wasm_func]
pub fn matrix_is_diagonal(matrix: &[u8]) -> Result<Vec<u8>, String> {
    encode_cbor(Value::Bool(decode_matrix(matrix, "matrix")?.is_diagonal()))
}

#[wasm_func]
pub fn matrix_derivative(matrix: &[u8], var: &[u8]) -> Result<Vec<u8>, String> {
    let matrix = decode_matrix(matrix, "matrix")?;
    let var = PolyVariable::try_from(decode_atom(var, "var")?)
        .map_err(|err| format!("matrix derivative variable must be an indeterminate: {err}"))?;
    encode_matrix(&matrix.derivative(&var))
}

#[cfg(test)]
#[unsafe(export_name = "wasm_minimal_protocol_send_result_to_host")]
extern "C" fn test_send_result_to_host(_: *const u8, _: usize) {}

#[cfg(test)]
#[unsafe(export_name = "wasm_minimal_protocol_write_args_to_buffer")]
extern "C" fn test_write_args_to_buffer(_: *mut u8) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integration_steps_are_per_expanded_term() {
        let (result, steps) = rational_integral_atoms(
            symbolica::parse!("(x + 1) * (x + 2)"),
            symbolica::parse!("x"),
        )
        .unwrap();

        assert_eq!(
            steps,
            vec![
                symbolica::parse!("2*x"),
                symbolica::parse!("3/2*x^2"),
                symbolica::parse!("1/3*x^3"),
            ]
        );
        assert_eq!(result, symbolica::parse!("2*x + 3/2*x^2 + 1/3*x^3"));
    }

    #[test]
    fn factored_and_expanded_inputs_have_the_same_integral() {
        let (factored, _) = rational_integral_atoms(
            symbolica::parse!("(x + 1) * (x + 2)"),
            symbolica::parse!("x"),
        )
        .unwrap();
        let (expanded, _) =
            rational_integral_atoms(symbolica::parse!("x^2 + 3*x + 2"), symbolica::parse!("x"))
                .unwrap();

        assert_eq!(factored, expanded);
    }

    #[test]
    fn integration_still_rejects_rational_denominators() {
        let error = rational_integral_atoms(symbolica::parse!("1/(x + 1)"), symbolica::parse!("x"))
            .unwrap_err();

        assert_eq!(
            error,
            "integration currently supports polynomial expressions only"
        );
    }

    #[test]
    fn collection_bridge_round_trips_an_expression() {
        let expr = symbolica::parse!("5*x+x*y+x^2+5");
        let x = symbolica::parse!("x");
        let request = encode_cbor(Value::Map(vec![
            (
                Value::Text("expr".to_owned()),
                Value::Bytes(encode_atom(&expr).unwrap()),
            ),
            (
                Value::Text("variables".to_owned()),
                Value::Bytes(encode_atom(&x).unwrap()),
            ),
        ]))
        .unwrap();

        let collected = decode_atom(&collect(&request).unwrap(), "collected").unwrap();
        assert_eq!(collected.expand(), expr.expand());

        let pairs = decode_cbor(&coefficient_list(&request).unwrap(), "pairs").unwrap();
        let Value::Array(pairs) = pairs else {
            panic!("coefficient list must be an array");
        };
        assert_eq!(pairs.len(), 3);

        let y = symbolica::parse!("y");
        let laurent = symbolica::parse!("x^-1*y+2*x^-1+3*y");
        let pairs = collected_coefficients(&laurent, &[x.clone(), y]).unwrap();
        let rebuilt = pairs
            .into_iter()
            .fold(Atom::num(0), |sum, (key, coefficient)| {
                sum + key * coefficient
            });
        assert_eq!((rebuilt - laurent).together(), Atom::num(0));

        assert!(collected_coefficients(&expr, &[symbolica::parse!("x^2")]).is_err());

        let nested = symbolica::parse!("(1+x)^2*x+(1+y)^100");
        let nested_pairs = collected_coefficients(&nested, &[x.clone()]).unwrap();
        let nested_collected = nested_pairs
            .into_iter()
            .fold(Atom::num(0), |sum, (key, coefficient)| {
                sum + key * coefficient
            });
        assert_eq!(nested_collected, symbolica::parse!("x+2*x^2+x^3+(1+y)^100"));

        let function = symbolica::parse!("f(x)");
        let function_power = symbolica::parse!("(1+f(x))^2");
        let by_same_named_variable =
            collected_coefficients(&function_power, &[symbolica::parse!("f")]).unwrap();
        assert_eq!(
            by_same_named_variable,
            vec![(Atom::num(1), function_power.clone())]
        );
        let by_function = collected_coefficients(&function_power, &[function]).unwrap();
        let function_collected = by_function
            .into_iter()
            .fold(Atom::num(0), |sum, (key, coefficient)| {
                sum + key * coefficient
            });
        assert_eq!(function_collected, symbolica::parse!("1+2*f(x)+f(x)^2"));

        let negative_compound = symbolica::parse!("(1+x)^-2+x");
        let negative_pairs = collected_coefficients(&negative_compound, &[x.clone()]).unwrap();
        assert!(negative_pairs.iter().any(|(key, coefficient)| {
            key == &Atom::num(1) && coefficient == &symbolica::parse!("(1+x)^-2")
        }));

        for (candidate, variables) in [
            (expr.clone(), vec![x.clone()]),
            (nested, vec![x.clone()]),
            (function_power, vec![symbolica::parse!("f(x)")]),
            (negative_compound, vec![x]),
        ] {
            let actual = collected_coefficients(&candidate, &variables).unwrap();
            let expected = candidate.coefficient_list::<i16>(&variables);
            assert_eq!(actual.len(), expected.len());
            for (key, coefficient) in expected {
                assert!(actual.iter().any(|(actual_key, actual_coefficient)| {
                    actual_key == &key && actual_coefficient == &coefficient
                }));
            }
        }
    }

    #[test]
    fn float_atoms_render_through_the_wasm_safe_path() {
        let render_typst = |atom: &Atom| {
            String::from_utf8(render_atom(
                atom,
                PrintOptions::typst(),
                FloatRenderStyle::Typst,
            ))
            .unwrap()
        };
        let exact_half = symbolica::parse!("1/2");
        for half in [
            atom_from_value(&Value::Float(0.5), "symbolica").unwrap(),
            atom_from_cbor_value(&Value::Float(0.5), "half").unwrap(),
            atom_from_leaf("0.5", "symbolica").unwrap(),
        ] {
            assert!(matches!(
                half.as_view(),
                AtomView::Num(number)
                    if matches!(number.get_coeff_view(), CoefficientView::Float(_, _))
            ));
            assert_ne!(half, exact_half);
            assert_eq!(render_typst(&half), "0.5");
        }

        let third = symbolica::parse!("1/3");
        let request = encode_cbor(Value::Map(vec![
            (
                Value::Text("expr".to_owned()),
                Value::Bytes(encode_atom(&third).unwrap()),
            ),
            (
                Value::Text("decimal-prec".to_owned()),
                Value::Integer(6.into()),
            ),
        ]))
        .unwrap();
        let approximated = decode_atom(&to_float(&request).unwrap(), "approximated").unwrap();
        let rendered = render_typst(&approximated);
        assert_eq!(rendered, "0.333333");

        let approximated = float_approximation(&third, 16);
        let rendered = render_typst(&approximated);
        assert_eq!(rendered, "0.3333333333333333");

        let builtin = float_approximation(&symbolica::parse!("cos(1/3)+1/2"), 6);
        let rendered = render_typst(&builtin);
        assert_eq!(rendered, "1.44496");

        let rational_builtin = float_approximation(&symbolica::parse!("cos(1/2)"), 6);
        let rendered = render_typst(&rational_builtin);
        assert_eq!(rendered, "0.877583");
        assert!(evaluate_constant_f64(&Atom::num(0.5)).is_err());

        let existing_float = float_approximation(&Atom::num(1.23456789), 3);
        let rendered = render_typst(&existing_float);
        assert_eq!(rendered, "1.23");

        let stored_precision = |atom: &Atom| {
            let AtomView::Num(number) = atom.as_view() else {
                panic!("expected a number");
            };
            let CoefficientView::Float(real, _) = number.get_coeff_view() else {
                panic!("expected a float");
            };
            serialized_float_value(real.0).unwrap().binary_precision
        };
        let low_precision = Atom::num(Coefficient::Float(Complex::new(
            Float::with_val(10, 1.2345),
            Float::new(10),
        )));
        let low_precision_result = float_approximation(&low_precision, 16);
        assert_eq!(stored_precision(&low_precision_result), 10);
        assert_eq!(low_precision_result, low_precision);
        let high_precision = Atom::num(Coefficient::Float(Complex::new(
            Float::with_val(100, 1.2345),
            Float::new(100),
        )));
        assert_eq!(
            stored_precision(&float_approximation(&high_precision, 6)),
            20
        );

        for (exact, expected) in [
            ("1234567890123456*10^385", "(1.23 times 10^(400))"),
            ("1234567890123456/10^415", "(1.23 times 10^(-400))"),
        ] {
            let full = float_approximation(&symbolica::parse!(exact), 16);
            assert_eq!(render_typst(&float_approximation(&full, 3)), expected);
        }

        for (exact, expected) in [
            ("10^400", "(1 times 10^(400))"),
            ("10^-400", "(1 times 10^(-400))"),
        ] {
            let approximated = float_approximation(&symbolica::parse!(exact), 6);
            let rendered = render_typst(&approximated);
            assert_eq!(rendered, expected);
        }

        let exact_huge_builtin = symbolica::parse!("sinh(1000)");
        assert_eq!(
            float_approximation(&exact_huge_builtin, 6),
            exact_huge_builtin
        );
        for mixed in [
            symbolica::parse!("x/3+sinh(1000)"),
            symbolica::parse!("0.5*x+sinh(1000)"),
        ] {
            assert_eq!(float_approximation(&mixed, 3), mixed);
        }

        for (value, expected) in [
            (f64::INFINITY, "infinity"),
            (f64::NEG_INFINITY, "-infinity"),
            (f64::NAN, r#"upright("NaN")"#),
        ] {
            let special = Atom::num(value);
            let rendered = render_typst(&special);
            assert_eq!(rendered, expected);
            assert_eq!(render_typst(&float_approximation(&special, 6)), expected);
        }

        let exponent = symbolica::parse!("x^(1/3)");
        assert_eq!(float_approximation(&exponent, 6), exponent);
        let opaque_function = symbolica::parse!("f(1/3)");
        assert_eq!(float_approximation(&opaque_function, 6), opaque_function);
    }

    #[test]
    fn rational_bridge_operations_are_inverse_forms() {
        let expr = symbolica::parse!("((x+3)*(2*x+5))/(x^3+6*x^2+11*x+6)");
        let reduced =
            decode_atom(&cancel(&encode_atom(&expr).unwrap()).unwrap(), "reduced").unwrap();
        assert_ne!(reduced, expr);
        let x = symbolica::parse!("x");
        let request = encode_cbor(Value::Map(vec![
            (
                Value::Text("expr".to_owned()),
                Value::Bytes(encode_atom(&reduced).unwrap()),
            ),
            (
                Value::Text("var".to_owned()),
                Value::Bytes(encode_atom(&x).unwrap()),
            ),
        ]))
        .unwrap();
        let modes = decode_atom(&apart(&request).unwrap(), "modes").unwrap();
        assert_eq!((modes - reduced).together(), Atom::num(0));
    }
}
