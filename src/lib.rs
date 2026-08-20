use std::io::Cursor;

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
#[cfg(feature = "rubi")]
use symbolica_integrate::{Integrate, IntegrationExplanation, IntegrationStep};
use tymbolica_atom_payload::{
    AttachmentSet, encode_atom as encode_shared_atom, encode_atom_from_set, parse_payload,
};
use tymbolica_typst_ast::AttachedAtom;
use wasm_minimal_protocol::*;

initiate_protocol!();

type MatrixField = RationalPolynomialField<IntegerRing, u16>;
type MatrixEntry = RationalPolynomial<IntegerRing, u16>;
type PluginMatrix = Matrix<MatrixField>;

const MATRIX_PAYLOAD_MAGIC: &[u8; 4] = b"SMTP";
const MATRIX_PAYLOAD_VERSION: u8 = 1;

struct AttachedMatrix {
    matrix: PluginMatrix,
    attachments: AttachmentSet,
}

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

fn decode_atom(input: &[u8], label: &str) -> Result<Atom, String> {
    decode_attached_atom(input, label).map(|payload| payload.atom)
}

fn decode_attached_atom(input: &[u8], label: &str) -> Result<AttachedAtom, String> {
    initialize_shared_symbol_registry();
    let payload =
        parse_payload(input).map_err(|err| format!("{label} must be Atom payload bytes: {err}"))?;
    let attachments = payload.attachment_set();
    tymbolica_symbol_registry::register_representation_attachments(&attachments)
        .map_err(|error| format!("{label} has invalid representation attachments: {error}"))?;
    let atom = payload
        .import_atom()
        .map_err(|err| format!("{label} must be Atom payload bytes: {err}"))?;
    Ok(AttachedAtom { atom, attachments })
}

fn encode_atom(atom: &Atom) -> Result<Vec<u8>, String> {
    encode_shared_atom(atom).map_err(|err| format!("failed to encode Atom payload: {err}"))
}

fn encode_attached_atom(atom: &Atom, attachments: &AttachmentSet) -> Result<Vec<u8>, String> {
    encode_atom_from_set(atom, attachments)
        .map_err(|err| format!("failed to encode Atom payload: {err}"))
}

fn merge_attachments(
    target: &mut AttachmentSet,
    source: &AttachmentSet,
    label: &str,
) -> Result<(), String> {
    target
        .merge(source)
        .map_err(|err| format!("could not merge {label} attachments: {err}"))
}

fn merge_attached_atom(
    attachments: &mut AttachmentSet,
    payload: AttachedAtom,
    label: &str,
) -> Result<Atom, String> {
    merge_attachments(attachments, &payload.attachments, label)?;
    Ok(payload.atom)
}

struct AttachedAtoms {
    atoms: Vec<Atom>,
    attachments: AttachmentSet,
}

fn attached_atoms_from_values(values: &[Value], label: &str) -> Result<AttachedAtoms, String> {
    let mut attachments = AttachmentSet::new();
    let atoms = values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let item_label = format!("{label}[{index}]");
            let payload = attached_atom_from_cbor_value(value, &item_label)?;
            merge_attached_atom(&mut attachments, payload, &item_label)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(AttachedAtoms { atoms, attachments })
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
    encode_attached_matrix(matrix, &AttachmentSet::new())
}

fn encode_attached_matrix(
    matrix: &PluginMatrix,
    attachments: &AttachmentSet,
) -> Result<Vec<u8>, String> {
    let atoms = matrix_entries_to_atoms(matrix);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(MATRIX_PAYLOAD_MAGIC);
    bytes.push(MATRIX_PAYLOAD_VERSION);
    write_u32(&mut bytes, matrix.nrows() as u32);
    write_u32(&mut bytes, matrix.ncols() as u32);
    write_u32(&mut bytes, atoms.len() as u32);
    for (index, atom) in atoms.into_iter().enumerate() {
        // SMTP stores the matrix-wide attachment environment once, using
        // the common Atom envelope on the first (always present) entry.
        let atom_bytes = if index == 0 {
            encode_attached_atom(&atom, attachments)?
        } else {
            encode_atom(&atom)?
        };
        write_u32(&mut bytes, atom_bytes.len() as u32);
        bytes.extend_from_slice(&atom_bytes);
    }
    Ok(bytes)
}

fn decode_matrix(input: &[u8], label: &str) -> Result<PluginMatrix, String> {
    decode_attached_matrix(input, label).map(|payload| payload.matrix)
}

fn decode_attached_matrix(input: &[u8], label: &str) -> Result<AttachedMatrix, String> {
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
    let expected_len = (nrows as usize)
        .checked_mul(ncols as usize)
        .ok_or_else(|| format!("{label} shape overflows usize"))?;
    if len != expected_len {
        return Err(format!(
            "{label} has {len} entries but shape is {nrows}x{ncols}"
        ));
    }
    if len > rest.len() / 4 {
        return Err(format!("{label} entry table is truncated"));
    }
    let mut atoms = Vec::with_capacity(len);
    let mut attachments = AttachmentSet::new();
    for index in 0..len {
        let atom_len = read_u32(&mut rest, label)? as usize;
        let atom = read_bytes(&mut rest, atom_len, label)?;
        let item_label = format!("{label}[{index}]");
        let payload = decode_attached_atom(atom, &item_label)?;
        atoms.push(merge_attached_atom(&mut attachments, payload, &item_label)?);
    }
    if !rest.is_empty() {
        return Err(format!("{label} has trailing bytes"));
    }
    Ok(AttachedMatrix {
        matrix: atoms_to_matrix(atoms, nrows, ncols)?,
        attachments,
    })
}

fn decode_atom_array(input: &[u8], label: &str) -> Result<AttachedAtoms, String> {
    match decode_cbor(input, label)? {
        Value::Array(values) => attached_atoms_from_values(&values, label),
        other => Err(format!(
            "{label} must be an array of Atom bytes, got {other:?}"
        )),
    }
}

fn atom_from_cbor_value(value: &Value, label: &str) -> Result<Atom, String> {
    attached_atom_from_cbor_value(value, label).map(|payload| payload.atom)
}

fn attached_atom_from_cbor_value(value: &Value, label: &str) -> Result<AttachedAtom, String> {
    match value {
        Value::Bytes(bytes) => decode_attached_atom(bytes, label),
        Value::Integer(n) => {
            let n: i64 = (*n)
                .try_into()
                .map_err(|_| format!("{label} integer is out of range"))?;
            Ok(AttachedAtom {
                atom: Atom::num(n),
                attachments: AttachmentSet::new(),
            })
        }
        Value::Float(n) => Ok(AttachedAtom {
            atom: Atom::num(*n),
            attachments: AttachmentSet::new(),
        }),
        Value::Text(text) => Ok(AttachedAtom {
            atom: symbol_atom(text, "typst")?,
            attachments: AttachmentSet::new(),
        }),
        other => Err(format!("{label} must be Atom bytes, got {other:?}")),
    }
}

fn attached_atom_from_ast(
    input: &[u8],
    namespace: &str,
    label: &str,
) -> Result<AttachedAtom, String> {
    let preflight = tymbolica_typst_ast::preflight_payloads_from_ast(input, label)?;
    tymbolica_symbol_registry::register_representation_attachments(&preflight.attachments)
        .map_err(|error| format!("{label} has invalid representation attachments: {error}"))?;
    let attached = tymbolica_typst_ast::attached_atom_from_ast(input, namespace, label)?;
    debug_assert_eq!(attached.attachments, preflight.attachments);
    Ok(attached)
}

fn symbol_atom(name: &str, namespace: &str) -> Result<Atom, String> {
    initialize_shared_symbol_registry();
    Symbol::parse(name.trim(), namespace.to_owned()).map(Atom::var)
}

fn initialize_shared_symbol_registry() {
    tymbolica_symbol_registry::initialize();
}

fn map_get<'a>(map: &'a [(Value, Value)], key: &str) -> Option<&'a Value> {
    map.iter().find_map(|(candidate, value)| match candidate {
        Value::Text(candidate) if candidate == key => Some(value),
        _ => None,
    })
}

fn map_array<'a>(map: &'a [(Value, Value)], key: &str) -> Result<&'a [Value], String> {
    match map_get(map, key) {
        Some(Value::Array(values)) => Ok(values),
        Some(_) => Err(format!("{key} must be an array")),
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

fn indeterminate_from_bytes(bytes: &[u8], label: &str) -> Result<Indeterminate, String> {
    Indeterminate::try_from(decode_atom(bytes, label)?).map_err(|err| err.to_string())
}

fn replacement_options(
    map: &[(Value, Value)],
    attachments: &mut AttachmentSet,
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
                let label = format!("non-greedy-wildcards[{index}]");
                let bytes = value_atom_bytes(value, &label)?;
                let payload = decode_attached_atom(bytes, &label)?;
                let atom = merge_attached_atom(attachments, payload, &label)?;
                match atom.as_view() {
                    AtomView::Var(var) => Ok(var.get_symbol()),
                    _ => Err(format!("{label} must be a symbol")),
                }
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

fn build_replacement(
    map: &[(Value, Value)],
    attachments: &mut AttachmentSet,
) -> Result<Replacement, String> {
    let pattern_atom = merge_attached_atom(
        attachments,
        decode_attached_atom(map_bytes(map, "pattern")?, "pattern")?,
        "pattern",
    )?;
    let rhs_atom = merge_attached_atom(
        attachments,
        decode_attached_atom(map_bytes(map, "rhs")?, "rhs")?,
        "rhs",
    )?;
    let pattern = pattern_atom.to_pattern();
    let rhs = rhs_atom.to_pattern();
    let (
        level_range,
        level_is_tree_depth,
        partial,
        allow_new_wildcards_on_rhs,
        rhs_cache_size,
        non_greedy,
    ) = replacement_options(map, attachments)?;

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

fn decode_nested_matrix(input: &[u8]) -> Result<AttachedMatrix, String> {
    match decode_cbor(input, "matrix")? {
        Value::Array(rows) => {
            if rows.is_empty() {
                return Err("matrix must have at least one row".to_owned());
            }
            let nrows = rows.len() as u32;
            let mut ncols = None;
            let mut atoms = Vec::new();
            let mut attachments = AttachmentSet::new();
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
                    let label = format!("matrix[{row_index}][{col_index}]");
                    let payload = attached_atom_from_cbor_value(value, &label)?;
                    atoms.push(merge_attached_atom(&mut attachments, payload, &label)?);
                }
            }
            Ok(AttachedMatrix {
                matrix: atoms_to_matrix(atoms, nrows, ncols.unwrap() as u32)?,
                attachments,
            })
        }
        other => Err(format!("matrix must be nested array, got {other:?}")),
    }
}

fn matrix_vec_from_values(input: &[u8]) -> Result<AttachedMatrix, String> {
    let atoms = decode_atom_array(input, "values")?;
    Ok(AttachedMatrix {
        matrix: atoms_to_matrix(atoms.atoms, cbor_len(input, "values")? as u32, 1)?,
        attachments: atoms.attachments,
    })
}

fn cbor_len(input: &[u8], label: &str) -> Result<usize, String> {
    match decode_cbor(input, label)? {
        Value::Array(values) => Ok(values.len()),
        other => Err(format!("{label} must be array, got {other:?}")),
    }
}

fn matrix_from_diag(input: &[u8]) -> Result<AttachedMatrix, String> {
    let atoms = decode_atom_array(input, "diag")?;
    if atoms.atoms.is_empty() {
        return Err("diagonal must not be empty".to_owned());
    }
    let mut entries = atoms
        .atoms
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
    Ok(AttachedMatrix {
        matrix: Matrix::eye(&entries, RationalPolynomialField::new(Z)),
        attachments: atoms.attachments,
    })
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

fn decode_unified_matrices(
    lhs: &[u8],
    rhs: &[u8],
) -> Result<(PluginMatrix, PluginMatrix, AttachmentSet), String> {
    let lhs = decode_attached_matrix(lhs, "lhs")?;
    let rhs = decode_attached_matrix(rhs, "rhs")?;
    let mut attachments = lhs.attachments;
    merge_attachments(&mut attachments, &rhs.attachments, "rhs")?;
    let (lhs, rhs) = unify_matrices(&lhs.matrix, &rhs.matrix);
    Ok((lhs, rhs, attachments))
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

fn cbor_atom_array(atoms: Vec<Atom>, attachments: &AttachmentSet) -> Result<Vec<u8>, String> {
    encode_cbor(Value::Array(
        atoms
            .iter()
            .map(|atom| encode_attached_atom(atom, attachments).map(Value::Bytes))
            .collect::<Result<Vec<_>, _>>()?,
    ))
}

fn atoms_cbor_value(
    atoms: impl IntoIterator<Item = Atom>,
    attachments: &AttachmentSet,
) -> Result<Value, String> {
    Ok(Value::Array(
        atoms
            .into_iter()
            .map(|atom| encode_attached_atom(&atom, attachments).map(Value::Bytes))
            .collect::<Result<Vec<_>, _>>()?,
    ))
}

fn cbor_f64(value: f64) -> Result<Vec<u8>, String> {
    encode_cbor(Value::Float(value))
}

fn cbor_f64_array(values: Vec<f64>) -> Result<Vec<u8>, String> {
    encode_cbor(Value::Array(values.into_iter().map(Value::Float).collect()))
}

fn atom_list(value: &Value, label: &str) -> Result<AttachedAtoms, String> {
    match value {
        Value::Array(values) => attached_atoms_from_values(values, label),
        Value::Bytes(bytes) => {
            let payload = decode_attached_atom(bytes, label)?;
            Ok(AttachedAtoms {
                atoms: vec![payload.atom],
                attachments: payload.attachments,
            })
        }
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
    )?
    .atoms;
    if expressions.is_empty() {
        return Err("expressions must not be empty".to_owned());
    }

    let variables = atom_list(
        map_get(map, "variables").ok_or_else(|| "missing variables".to_owned())?,
        "variables",
    )?
    .atoms;
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
    let parsed = attached_atom_from_ast(ast, &namespace, "ast")?;
    encode_attached_atom(&parsed.atom, &parsed.attachments)
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
    let expr = decode_attached_atom(expr, "expr")?;
    encode_attached_atom(&expr.atom, &expr.attachments)
}

#[wasm_func]
pub fn expand(expr: &[u8]) -> Result<Vec<u8>, String> {
    let expr = decode_attached_atom(expr, "expr")?;
    let expanded = expr.atom.expand_via_poly::<u16, Atom>(None);
    encode_attached_atom(&expanded, &expr.attachments)
}

#[wasm_func]
pub fn together(expr: &[u8]) -> Result<Vec<u8>, String> {
    let expr = decode_attached_atom(expr, "expr")?;
    encode_attached_atom(&expr.atom.together(), &expr.attachments)
}

#[wasm_func]
pub fn cancel(expr: &[u8]) -> Result<Vec<u8>, String> {
    let expr = decode_attached_atom(expr, "expr")?;
    encode_attached_atom(&expr.atom.cancel(), &expr.attachments)
}

#[wasm_func]
pub fn apart(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "apart request")? else {
        return Err("apart request must be dictionary".to_owned());
    };
    let expr = decode_attached_atom(map_bytes(&map, "expr")?, "expr")?;
    let var = decode_attached_atom(map_bytes(&map, "var")?, "var")?;
    let mut attachments = expr.attachments;
    merge_attachments(&mut attachments, &var.attachments, "var")?;
    let var = Indeterminate::try_from(var.atom).map_err(|err| err.to_string())?;
    encode_attached_atom(&expr.atom.apart(var), &attachments)
}

#[wasm_func]
pub fn collect(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "collect request")? else {
        return Err("collect request must be dictionary".to_owned());
    };
    let expr = decode_attached_atom(map_bytes(&map, "expr")?, "expr")?;
    let variables = atom_list(
        map_get(&map, "variables").ok_or_else(|| "missing variables".to_owned())?,
        "variables",
    )?;
    let mut attachments = expr.attachments;
    merge_attachments(&mut attachments, &variables.attachments, "variables")?;
    let result = collected_coefficients(&expr.atom, &variables.atoms)?
        .into_iter()
        .fold(Atom::num(0), |sum, (key, coefficient)| {
            sum + key * coefficient
        });
    encode_attached_atom(&result, &attachments)
}

#[wasm_func]
pub fn coefficient(expr: &[u8], monomial: &[u8]) -> Result<Vec<u8>, String> {
    let expr = decode_attached_atom(expr, "expr")?;
    let monomial = decode_attached_atom(monomial, "monomial")?;
    let mut attachments = expr.attachments;
    merge_attachments(&mut attachments, &monomial.attachments, "monomial")?;
    encode_attached_atom(&expr.atom.coefficient(monomial.atom), &attachments)
}

#[wasm_func]
pub fn coefficient_list(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "coefficient-list request")? else {
        return Err("coefficient-list request must be dictionary".to_owned());
    };
    let expr = decode_attached_atom(map_bytes(&map, "expr")?, "expr")?;
    let variables = atom_list(
        map_get(&map, "variables").ok_or_else(|| "missing variables".to_owned())?,
        "variables",
    )?;
    let mut attachments = expr.attachments;
    merge_attachments(&mut attachments, &variables.attachments, "variables")?;
    let pairs = collected_coefficients(&expr.atom, &variables.atoms)?
        .into_iter()
        .map(|(key, coefficient)| {
            Ok(Value::Array(vec![
                Value::Bytes(encode_attached_atom(&key, &attachments)?),
                Value::Bytes(encode_attached_atom(&coefficient, &attachments)?),
            ]))
        })
        .collect::<Result<Vec<_>, String>>()?;
    encode_cbor(Value::Array(pairs))
}

#[wasm_func]
pub fn terms(expr: &[u8]) -> Result<Vec<u8>, String> {
    let expr = decode_attached_atom(expr, "expr")?;
    cbor_atom_array(
        expr.atom.terms().map(|value| value.to_owned()).collect(),
        &expr.attachments,
    )
}

#[wasm_func]
pub fn indeterminates(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "indeterminates request")? else {
        return Err("indeterminates request must be dictionary".to_owned());
    };
    let expr = decode_attached_atom(map_bytes(&map, "expr")?, "expr")?;
    let mut values = expr
        .atom
        .get_all_indeterminates(map_bool(&map, "enter-functions", true)?)
        .into_iter()
        .map(|value| value.to_owned())
        .collect::<Vec<_>>();
    values.sort();
    cbor_atom_array(values, &expr.attachments)
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
    let expr = decode_attached_atom(map_bytes(&map, "expr")?, "expr")?;
    encode_attached_atom(
        &float_approximation(&expr.atom, decimal_precision as u32),
        &expr.attachments,
    )
}

#[wasm_func]
pub fn factor(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "factor request")? else {
        return Err("factor request must be dictionary".to_owned());
    };
    let expr = decode_attached_atom(map_bytes(&map, "expr")?, "expr")?;
    let complex = map_bool(&map, "complex", false)?;
    let square_free = map_bool(&map, "square-free", false)?;
    if complex && square_free {
        return Err("complex and square-free factorization cannot be combined".to_owned());
    }
    let factored = if square_free {
        expr.atom.as_view().factor_square_free()
    } else if complex {
        expr.atom.factor_complex()
    } else {
        expr.atom.factor()
    };
    encode_attached_atom(&factored, &expr.attachments)
}

#[wasm_func]
pub fn derivative(expr: &[u8], var: &[u8]) -> Result<Vec<u8>, String> {
    let expr = decode_attached_atom(expr, "expr")?;
    let var = decode_attached_atom(var, "var")?;
    let mut attachments = expr.attachments;
    merge_attachments(&mut attachments, &var.attachments, "var")?;
    let var = Indeterminate::try_from(var.atom)?;
    encode_attached_atom(&expr.atom.derivative(var), &attachments)
}

#[cfg(feature = "rubi")]
fn integration_variable(var: Atom) -> Result<Symbol, String> {
    match var.as_view() {
        AtomView::Var(var) => Ok(var.get_symbol()),
        _ => Err("integration variable must be a symbol".to_owned()),
    }
}

#[cfg(feature = "rubi")]
fn rubi_integral_atoms(expr: Atom, var: Atom) -> Result<Result<Atom, Atom>, String> {
    Ok(expr.integrate(integration_variable(var)?))
}

#[cfg(feature = "rubi")]
fn rubi_integration_explanation_atoms(
    expr: Atom,
    var: Atom,
) -> Result<IntegrationExplanation, String> {
    Ok(expr.integrate_with_steps(integration_variable(var)?))
}

#[cfg(feature = "rubi")]
fn integration_step_cbor(
    step: IntegrationStep,
    attachments: &AttachmentSet,
) -> Result<Value, String> {
    Ok(Value::Map(vec![
        (
            Value::Text("rule".to_owned()),
            step.rule
                .map(|rule| Value::Integer((rule as i64).into()))
                .unwrap_or(Value::Null),
        ),
        (
            Value::Text("depth".to_owned()),
            Value::Integer((step.depth as i64).into()),
        ),
        (
            Value::Text("description".to_owned()),
            Value::Text(step.description.to_owned()),
        ),
        (
            Value::Text("references".to_owned()),
            Value::Array(
                step.references
                    .iter()
                    .map(|reference| Value::Text((*reference).to_owned()))
                    .collect(),
            ),
        ),
        (
            Value::Text("source".to_owned()),
            Value::Text(step.source.to_owned()),
        ),
        (
            Value::Text("input".to_owned()),
            Value::Bytes(encode_attached_atom(&step.input, attachments)?),
        ),
        (
            Value::Text("output".to_owned()),
            Value::Bytes(encode_attached_atom(&step.output, attachments)?),
        ),
    ]))
}

#[cfg(feature = "rubi")]
#[wasm_func]
pub fn integrate(expr: &[u8], var: &[u8]) -> Result<Vec<u8>, String> {
    let expr = decode_attached_atom(expr, "expr")?;
    let var = decode_attached_atom(var, "var")?;
    let mut attachments = expr.attachments;
    merge_attachments(&mut attachments, &var.attachments, "var")?;
    let result = rubi_integral_atoms(expr.atom, var.atom)?;
    encode_attached_atom(
        match &result {
            Ok(result) | Err(result) => result,
        },
        &attachments,
    )
}

#[cfg(feature = "rubi")]
#[wasm_func]
pub fn integrate_with_steps(expr: &[u8], var: &[u8]) -> Result<Vec<u8>, String> {
    let expr = decode_attached_atom(expr, "expr")?;
    let var = decode_attached_atom(var, "var")?;
    let mut attachments = expr.attachments;
    merge_attachments(&mut attachments, &var.attachments, "var")?;
    let explanation = rubi_integration_explanation_atoms(expr.atom, var.atom)?;
    let (complete, result) = match explanation.result {
        Ok(result) => (true, result),
        Err(result) => (false, result),
    };
    let steps = explanation
        .steps
        .into_iter()
        .map(|step| integration_step_cbor(step, &attachments))
        .collect::<Result<Vec<_>, _>>()?;
    encode_cbor(Value::Map(vec![
        (
            Value::Text("result".to_owned()),
            Value::Bytes(encode_attached_atom(&result, &attachments)?),
        ),
        (Value::Text("complete".to_owned()), Value::Bool(complete)),
        (Value::Text("steps".to_owned()), Value::Array(steps)),
    ]))
}

#[wasm_func]
pub fn series(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "series request")? else {
        return Err("series request must be dictionary".to_owned());
    };
    let expr = decode_attached_atom(map_bytes(&map, "expr")?, "expr")?;
    let var = decode_attached_atom(map_bytes(&map, "var")?, "var")?;
    let expansion_point =
        decode_attached_atom(map_bytes(&map, "expansion-point")?, "expansion-point")?;
    let mut attachments = expr.attachments;
    merge_attachments(&mut attachments, &var.attachments, "var")?;
    merge_attachments(
        &mut attachments,
        &expansion_point.attachments,
        "expansion-point",
    )?;
    let var = Indeterminate::try_from(var.atom).map_err(|err| err.to_string())?;
    let depth = map_i64(&map, "depth", 0)?;
    let depth_denom = map_i64(&map, "depth-denom", 1)?;
    let depth = if map_bool(&map, "depth-is-absolute", true)? {
        SeriesDepth::absolute((depth, depth_denom))
    } else {
        SeriesDepth::relative((depth, depth_denom))
    };
    encode_attached_atom(
        &expr
            .atom
            .series(var, expansion_point.atom.as_view(), depth)
            .map_err(|err| err.to_string())?
            .to_atom(),
        &attachments,
    )
}

#[wasm_func]
pub fn replace(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "replace request")? else {
        return Err("replace request must be dictionary".to_owned());
    };
    let expr = decode_attached_atom(map_bytes(&map, "expr")?, "expr")?;
    let mut attachments = expr.attachments;
    let replacement = build_replacement(&map, &mut attachments)?;
    let settings = build_replace_settings(&map)?;
    let repeat = map_bool(&map, "repeat", false)?;
    encode_attached_atom(
        &repeat_replacements(expr.atom, vec![replacement], settings, repeat),
        &attachments,
    )
}

#[wasm_func]
pub fn replace_multiple(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "replace-multiple request")? else {
        return Err("replace-multiple request must be dictionary".to_owned());
    };
    let expr = decode_attached_atom(map_bytes(&map, "expr")?, "expr")?;
    let mut attachments = expr.attachments;
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
            build_replacement(rule, &mut attachments)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let settings = build_replace_settings(&map)?;
    let repeat = map_bool(&map, "repeat", false)?;
    encode_attached_atom(
        &repeat_replacements(expr.atom, replacements, settings, repeat),
        &attachments,
    )
}

#[wasm_func]
pub fn replace_wildcards(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "replace-wildcards request")? else {
        return Err("replace-wildcards request must be dictionary".to_owned());
    };
    let pattern = decode_attached_atom(map_bytes(&map, "pattern")?, "pattern")?;
    let mut attachments = pattern.attachments;
    let pattern = pattern.atom.to_pattern();
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
        let wildcard = decode_attached_atom(value_atom_bytes(&pair[0], "wildcard")?, "wildcard")?;
        let wildcard = merge_attached_atom(&mut attachments, wildcard, "wildcard")?;
        let wildcard = match wildcard.as_view() {
            AtomView::Var(var) => var.get_symbol(),
            _ => return Err("wildcard must be a symbol".to_owned()),
        };
        if wildcard.get_wildcard_level() == 0 {
            return Err("only wildcards can be replaced".to_owned());
        }
        let replacement =
            decode_attached_atom(value_atom_bytes(&pair[1], "replacement")?, "replacement")?;
        let replacement = merge_attached_atom(&mut attachments, replacement, "replacement")?;
        map.insert(wildcard, replacement);
    }
    encode_attached_atom(
        &pattern
            .replace_wildcards(&map)
            .map_err(|err| err.to_string())?,
        &attachments,
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
        Some(Value::Array(values)) => attached_atoms_from_values(values, "system")?,
        Some(Value::Bytes(bytes)) if is_matrix_payload(bytes) => {
            let matrix = decode_attached_matrix(bytes, "system")?;
            AttachedAtoms {
                atoms: matrix_entries_to_atoms(&matrix.matrix),
                attachments: matrix.attachments,
            }
        }
        Some(other) => {
            return Err(format!(
                "system must be array or vector matrix, got {other:?}"
            ));
        }
        None => return Err("missing system".to_owned()),
    };
    let vars = match map_get(&map, "variables") {
        Some(Value::Array(values)) => attached_atoms_from_values(values, "variables")?,
        Some(other) => return Err(format!("variables must be array, got {other:?}")),
        None => return Err("missing variables".to_owned()),
    };
    let mut attachments = system.attachments;
    merge_attachments(&mut attachments, &vars.attachments, "variables")?;
    let result = match AtomView::solve_linear_system::<u16, _, Atom>(&system.atoms, &vars.atoms) {
        Ok(result) => result,
        Err(SolveError::Underdetermined {
            partial_solution, ..
        }) => partial_solution,
        Err(err) => return Err(err.to_string()),
    };
    cbor_atom_array(result, &attachments)
}

#[wasm_func]
pub fn solve_system(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "solve-system request")? else {
        return Err("solve-system request must be dictionary".to_owned());
    };
    let system = attached_atoms_from_values(map_array(&map, "system")?, "system")?;
    let variables = attached_atoms_from_values(map_array(&map, "variables")?, "variables")?;
    let mut attachments = system.attachments;
    merge_attachments(&mut attachments, &variables.attachments, "variables")?;
    let keys = variables
        .atoms
        .iter()
        .map(|variable| {
            Indeterminate::try_from(variable.clone())
                .map(PolyVariable::from)
                .map_err(|err| format!("solve variable must be a variable: {err}"))
        })
        .collect::<Result<Vec<PolyVariable>, _>>()?;
    let solutions = AtomView::solve::<u16, _, Atom>(&system.atoms, &variables.atoms)
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
                .and_then(|atoms| atoms_cbor_value(atoms, &attachments))
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
    encode_attached_atom(&Atom::add_many(args.atoms), &args.attachments)
}

#[wasm_func]
pub fn mul(args: &[u8]) -> Result<Vec<u8>, String> {
    let args = decode_atom_array(args, "args")?;
    encode_attached_atom(&Atom::mul_many(args.atoms), &args.attachments)
}

#[wasm_func]
pub fn neg(expr: &[u8]) -> Result<Vec<u8>, String> {
    let expr = decode_attached_atom(expr, "expr")?;
    encode_attached_atom(&(-expr.atom), &expr.attachments)
}

#[wasm_func]
pub fn sub(lhs: &[u8], rhs: &[u8]) -> Result<Vec<u8>, String> {
    let lhs = decode_attached_atom(lhs, "lhs")?;
    let rhs = decode_attached_atom(rhs, "rhs")?;
    let mut attachments = lhs.attachments;
    merge_attachments(&mut attachments, &rhs.attachments, "rhs")?;
    encode_attached_atom(&(lhs.atom - rhs.atom), &attachments)
}

#[wasm_func]
pub fn div(lhs: &[u8], rhs: &[u8]) -> Result<Vec<u8>, String> {
    let lhs = decode_attached_atom(lhs, "lhs")?;
    let rhs = decode_attached_atom(rhs, "rhs")?;
    let mut attachments = lhs.attachments;
    merge_attachments(&mut attachments, &rhs.attachments, "rhs")?;
    encode_attached_atom(&(lhs.atom / rhs.atom), &attachments)
}

#[wasm_func]
pub fn power(base: &[u8], exp: &[u8]) -> Result<Vec<u8>, String> {
    let base = decode_attached_atom(base, "base")?;
    let exp = decode_attached_atom(exp, "exp")?;
    let mut attachments = base.attachments;
    merge_attachments(&mut attachments, &exp.attachments, "exp")?;
    encode_attached_atom(&base.atom.pow(exp.atom), &attachments)
}

#[wasm_func]
pub fn matrix_from_nested(input: &[u8]) -> Result<Vec<u8>, String> {
    let matrix = decode_nested_matrix(input)?;
    encode_attached_matrix(&matrix.matrix, &matrix.attachments)
}

#[wasm_func]
pub fn matrix_vec(values: &[u8]) -> Result<Vec<u8>, String> {
    let matrix = matrix_vec_from_values(values)?;
    encode_attached_matrix(&matrix.matrix, &matrix.attachments)
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
    let matrix = matrix_from_diag(diag)?;
    encode_attached_matrix(&matrix.matrix, &matrix.attachments)
}

#[wasm_func]
pub fn matrix_add(lhs: &[u8], rhs: &[u8]) -> Result<Vec<u8>, String> {
    let (lhs, rhs, attachments) = decode_unified_matrices(lhs, rhs)?;
    encode_attached_matrix(&(&lhs + &rhs), &attachments)
}

#[wasm_func]
pub fn matrix_sub(lhs: &[u8], rhs: &[u8]) -> Result<Vec<u8>, String> {
    let (lhs, rhs, attachments) = decode_unified_matrices(lhs, rhs)?;
    encode_attached_matrix(&(&lhs - &rhs), &attachments)
}

#[wasm_func]
pub fn matrix_mul(lhs: &[u8], rhs: &[u8]) -> Result<Vec<u8>, String> {
    let lhs = decode_attached_matrix(lhs, "lhs")?;
    if is_matrix_payload(rhs) {
        let rhs = decode_attached_matrix(rhs, "rhs")?;
        let mut attachments = lhs.attachments;
        merge_attachments(&mut attachments, &rhs.attachments, "rhs")?;
        let (lhs, rhs) = unify_matrices(&lhs.matrix, &rhs.matrix);
        encode_attached_matrix(&(&lhs * &rhs), &attachments)
    } else {
        let scalar = decode_attached_atom(rhs, "rhs")?;
        let mut attachments = lhs.attachments;
        merge_attachments(&mut attachments, &scalar.attachments, "rhs")?;
        let scalar = atom_to_matrix_entry(&scalar.atom)?;
        let (lhs, scalar) = unify_matrix_scalar(&lhs.matrix, scalar);
        encode_attached_matrix(&lhs.mul_scalar(&scalar), &attachments)
    }
}

#[wasm_func]
pub fn matrix_div_scalar(lhs: &[u8], rhs: &[u8]) -> Result<Vec<u8>, String> {
    let lhs = decode_attached_matrix(lhs, "lhs")?;
    let scalar = decode_attached_atom(rhs, "rhs")?;
    let mut attachments = lhs.attachments;
    merge_attachments(&mut attachments, &scalar.attachments, "rhs")?;
    let scalar = atom_to_matrix_entry(&scalar.atom)?;
    let (lhs, scalar) = unify_matrix_scalar(&lhs.matrix, scalar);
    if scalar.is_zero() {
        return Err("cannot divide a matrix by zero".to_owned());
    }
    encode_attached_matrix(&lhs.div_scalar(&scalar), &attachments)
}

#[wasm_func]
pub fn transpose(matrix: &[u8]) -> Result<Vec<u8>, String> {
    let matrix = decode_attached_matrix(matrix, "matrix")?;
    encode_attached_matrix(&matrix.matrix.transpose(), &matrix.attachments)
}

#[wasm_func]
pub fn det(matrix: &[u8]) -> Result<Vec<u8>, String> {
    let matrix = decode_attached_matrix(matrix, "matrix")?;
    let det = matrix.matrix.det().map_err(|err| err.to_string())?;
    encode_attached_atom(&det.to_expression(), &matrix.attachments)
}

#[wasm_func]
pub fn inv(matrix: &[u8]) -> Result<Vec<u8>, String> {
    let matrix = decode_attached_matrix(matrix, "matrix")?;
    encode_attached_matrix(
        &matrix.matrix.inv().map_err(|err| err.to_string())?,
        &matrix.attachments,
    )
}

#[wasm_func]
pub fn matrix_solve(lhs: &[u8], rhs: &[u8]) -> Result<Vec<u8>, String> {
    let (lhs, rhs, attachments) = decode_unified_matrices(lhs, rhs)?;
    encode_attached_matrix(
        &lhs.solve(&rhs).map_err(|err| err.to_string())?,
        &attachments,
    )
}

#[wasm_func]
pub fn matrix_solve_any(lhs: &[u8], rhs: &[u8]) -> Result<Vec<u8>, String> {
    let (lhs, rhs, attachments) = decode_unified_matrices(lhs, rhs)?;
    encode_attached_matrix(
        &lhs.solve_any(&rhs).map_err(|err| err.to_string())?,
        &attachments,
    )
}

#[wasm_func]
pub fn row_reduce(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "row-reduce request")? else {
        return Err("row-reduce request must be dictionary".to_owned());
    };
    let mut matrix = decode_attached_matrix(map_bytes(&map, "matrix")?, "matrix")?;
    let max_col = map_usize(&map, "max-col", matrix.matrix.ncols())? as u32;
    let rank = matrix.matrix.row_reduce(max_col);
    encode_cbor(Value::Map(vec![
        (
            Value::Text("matrix".to_owned()),
            Value::Bytes(encode_attached_matrix(&matrix.matrix, &matrix.attachments)?),
        ),
        (
            Value::Text("rank".to_owned()),
            Value::Integer((rank as i64).into()),
        ),
    ]))
}

#[wasm_func]
pub fn augment(lhs: &[u8], rhs: &[u8]) -> Result<Vec<u8>, String> {
    let (lhs, rhs, attachments) = decode_unified_matrices(lhs, rhs)?;
    encode_attached_matrix(
        &lhs.augment(&rhs).map_err(|err| err.to_string())?,
        &attachments,
    )
}

#[wasm_func]
pub fn split_col(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "split-col request")? else {
        return Err("split-col request must be dictionary".to_owned());
    };
    let matrix = decode_attached_matrix(map_bytes(&map, "matrix")?, "matrix")?;
    let index = map_usize(&map, "index", 0)? as u32;
    let (lhs, rhs) = matrix
        .matrix
        .split_col(index)
        .map_err(|err| err.to_string())?;
    encode_cbor(Value::Array(vec![
        Value::Bytes(encode_attached_matrix(&lhs, &matrix.attachments)?),
        Value::Bytes(encode_attached_matrix(&rhs, &matrix.attachments)?),
    ]))
}

#[wasm_func]
pub fn primitive_part(matrix: &[u8]) -> Result<Vec<u8>, String> {
    let matrix = decode_attached_matrix(matrix, "matrix")?;
    encode_attached_matrix(&matrix.matrix.primitive_part(), &matrix.attachments)
}

#[wasm_func]
pub fn content(matrix: &[u8]) -> Result<Vec<u8>, String> {
    let matrix = decode_attached_matrix(matrix, "matrix")?;
    let content = matrix.matrix.content();
    encode_attached_atom(&content.to_expression(), &matrix.attachments)
}

#[wasm_func]
pub fn matrix_at(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "matrix-at request")? else {
        return Err("matrix-at request must be dictionary".to_owned());
    };
    let matrix = decode_attached_matrix(map_bytes(&map, "matrix")?, "matrix")?;
    let row = map_usize(&map, "row", 0)?;
    let col = map_usize(&map, "col", 0)?;
    if row >= matrix.matrix.nrows() || col >= matrix.matrix.ncols() {
        return Err("matrix index out of bounds".to_owned());
    }
    encode_attached_atom(
        &matrix.matrix[(row as u32, col as u32)]
            .clone()
            .to_expression(),
        &matrix.attachments,
    )
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
    let matrix = decode_attached_matrix(matrix, "matrix")?;
    let var = decode_attached_atom(var, "var")?;
    let mut attachments = matrix.attachments;
    merge_attachments(&mut attachments, &var.attachments, "var")?;
    let var = PolyVariable::try_from(var.atom)
        .map_err(|err| format!("matrix derivative variable must be an indeterminate: {err}"))?;
    encode_attached_matrix(&matrix.matrix.derivative(&var), &attachments)
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
    use tymbolica_atom_payload::{Attachment, AttachmentKey};

    fn test_attachment_key(identity: &[u8]) -> AttachmentKey {
        AttachmentKey::new("org.tymbolica.test", 1, identity.to_vec()).unwrap()
    }

    fn attached_test_atom(atom: &Atom, key: &AttachmentKey, data: &[u8]) -> Vec<u8> {
        let attachments =
            AttachmentSet::from_attachments([Attachment::new(key.clone(), data.to_vec()).unwrap()])
                .unwrap();
        encode_attached_atom(atom, &attachments).unwrap()
    }

    fn assert_payload_attachment(payload: &[u8], key: &AttachmentKey, data: &[u8]) {
        let parsed = parse_payload(payload).unwrap();
        assert_eq!(parsed.attachment(key), Some(data));
        let _ = parsed.import_atom().unwrap();
    }

    #[cfg(feature = "rubi")]
    #[test]
    fn rubi_integration_records_nested_rule_transformations() {
        let integrand = symbolica::parse!("x/(x + 1)");
        let x = symbolica::symbol!("x");
        let explanation =
            rubi_integration_explanation_atoms(integrand.clone(), Atom::var(x)).unwrap();

        let result = explanation.result.as_ref().unwrap();
        let residual = (result.derivative(x) - integrand).expand().together();
        assert!(residual.is_zero());
        assert_eq!(explanation.steps.first().unwrap().depth, 0);
        assert!(explanation.steps.iter().any(|step| step.depth > 0));
        assert!(
            explanation
                .steps
                .iter()
                .any(|step| step.rule.is_some() && !step.source.is_empty())
        );
        assert!(
            explanation
                .steps
                .iter()
                .all(|step| !step.description.is_empty() && step.input != step.output)
        );
    }

    #[cfg(feature = "rubi")]
    #[test]
    fn rubi_integrates_a_rational_denominator() {
        let x = symbolica::symbol!("x");
        let result = rubi_integral_atoms(symbolica::parse!("1/(x^2 + 1)"), Atom::var(x))
            .unwrap()
            .unwrap();

        assert_eq!(result, symbolica::parse!("atan(x)"));
    }

    #[cfg(feature = "rubi")]
    #[test]
    fn rubi_step_bridge_preserves_complete_incomplete_and_substitution_steps() {
        let x = Atom::var(symbolica::symbol!("x"));
        let encoded_x = encode_atom(&x).unwrap();

        let decode_explanation = |integrand: Atom| {
            let payload = integrate_with_steps(&encode_atom(&integrand).unwrap(), &encoded_x)
                .expect("integration bridge should encode its explanation");
            let Value::Map(map) = decode_cbor(&payload, "integration explanation").unwrap() else {
                panic!("integration explanation must be a dictionary");
            };
            map
        };

        let complete = decode_explanation(symbolica::parse!("x/(x + 1)"));
        assert_eq!(map_get(&complete, "complete"), Some(&Value::Bool(true)));
        assert!(map_get(&complete, "overview").is_none());
        let Value::Bytes(result) = map_get(&complete, "result").unwrap() else {
            panic!("integration result must be Atom bytes");
        };
        let result = decode_atom(result, "integration result").unwrap();
        assert_eq!(
            (result.derivative(symbolica::symbol!("x")) - symbolica::parse!("x/(x + 1)"))
                .together(),
            Atom::num(0)
        );
        let Value::Array(steps) = map_get(&complete, "steps").unwrap() else {
            panic!("integration steps must be an array");
        };
        assert!(steps.len() > 1);
        for step in steps {
            let Value::Map(step) = step else {
                panic!("each integration step must be a dictionary");
            };
            assert!(matches!(map_get(step, "rule"), Some(Value::Integer(_))));
            assert!(matches!(map_get(step, "depth"), Some(Value::Integer(_))));
            assert!(matches!(map_get(step, "description"), Some(Value::Text(_))));
            assert!(matches!(map_get(step, "references"), Some(Value::Array(_))));
            assert!(matches!(map_get(step, "source"), Some(Value::Text(_))));
            for field in ["input", "output"] {
                let Some(Value::Bytes(atom)) = map_get(step, field) else {
                    panic!("{field} must be Atom bytes");
                };
                let _ = decode_atom(atom, field).expect("step Atom should round-trip");
            }
        }

        let substitution = decode_explanation(symbolica::parse!("exp(x)/(1 + exp(x))"));
        let Value::Array(steps) = map_get(&substitution, "steps").unwrap() else {
            panic!("integration steps must be an array");
        };
        assert!(steps.iter().any(|step| {
            let Value::Map(step) = step else {
                return false;
            };
            matches!(map_get(step, "rule"), Some(Value::Null))
        }));

        let incomplete = decode_explanation(symbolica::parse!("x + x^x"));
        assert_eq!(map_get(&incomplete, "complete"), Some(&Value::Bool(false)));
        let Value::Bytes(result) = map_get(&incomplete, "result").unwrap() else {
            panic!("incomplete integration result must be Atom bytes");
        };
        assert!(
            decode_atom(result, "incomplete integration result")
                .unwrap()
                .to_string()
                .contains("unintegrable")
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
        let encoded_leaf = encode_cbor(Value::Text("0.5".to_owned())).unwrap();
        for half in [
            tymbolica_typst_ast::atom_from_value(&Value::Float(0.5), "symbolica").unwrap(),
            atom_from_cbor_value(&Value::Float(0.5), "half").unwrap(),
            tymbolica_typst_ast::atom_from_ast(&encoded_leaf, "symbolica", "leaf").unwrap(),
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

    #[test]
    fn atom_operations_preserve_merge_and_fan_out_attachments() {
        let x = symbolica::parse!("x");
        let y = symbolica::parse!("y");
        let x_key = test_attachment_key(b"x");
        let y_key = test_attachment_key(b"y");
        let x_payload = attached_test_atom(&x, &x_key, b"x declaration");
        let y_payload = attached_test_atom(&y, &y_key, b"y declaration");

        let negated = neg(&x_payload).unwrap();
        assert_payload_attachment(&negated, &x_key, b"x declaration");

        let args = encode_cbor(Value::Array(vec![
            Value::Bytes(x_payload.clone()),
            Value::Bytes(y_payload.clone()),
        ]))
        .unwrap();
        let sum = add(&args).unwrap();
        assert_payload_attachment(&sum, &x_key, b"x declaration");
        assert_payload_attachment(&sum, &y_key, b"y declaration");

        let Value::Array(terms) = decode_cbor(&terms(&sum).unwrap(), "terms").unwrap() else {
            panic!("terms must be an array");
        };
        assert_eq!(terms.len(), 2);
        for term in terms {
            let Value::Bytes(term) = term else {
                panic!("term must be Atom bytes");
            };
            assert_payload_attachment(&term, &x_key, b"x declaration");
            assert_payload_attachment(&term, &y_key, b"y declaration");
        }

        let ast = encode_cbor(Value::Bytes(x_payload)).unwrap();
        let namespace = encode_cbor(Value::Text("test".to_owned())).unwrap();
        let parsed = from_ast(&ast, &namespace).unwrap();
        assert_payload_attachment(&parsed, &x_key, b"x declaration");
    }

    #[test]
    fn atom_operation_attachment_conflicts_fail_closed() {
        let key = test_attachment_key(b"same");
        let x = attached_test_atom(&symbolica::parse!("x"), &key, b"one");
        let y = attached_test_atom(&symbolica::parse!("y"), &key, b"two");
        let error = sub(&x, &y).unwrap_err();
        assert!(error.contains("conflicting data"));
    }

    #[test]
    fn replacement_inputs_merge_into_the_result_environment() {
        let expr_key = test_attachment_key(b"replace-expr");
        let rhs_key = test_attachment_key(b"replace-rhs");
        let x = symbolica::parse!("x");
        let y = symbolica::parse!("y");
        let request = encode_cbor(Value::Map(vec![
            (
                Value::Text("expr".to_owned()),
                Value::Bytes(attached_test_atom(&x, &expr_key, b"expr declaration")),
            ),
            (
                Value::Text("pattern".to_owned()),
                Value::Bytes(encode_atom(&x).unwrap()),
            ),
            (
                Value::Text("rhs".to_owned()),
                Value::Bytes(attached_test_atom(&y, &rhs_key, b"rhs declaration")),
            ),
        ]))
        .unwrap();

        let replaced = replace(&request).unwrap();
        assert_payload_attachment(&replaced, &expr_key, b"expr declaration");
        assert_payload_attachment(&replaced, &rhs_key, b"rhs declaration");
        assert_eq!(decode_atom(&replaced, "replaced").unwrap(), y);
    }

    #[test]
    fn matrix_payloads_carry_the_merged_attachment_environment_once() {
        let x_key = test_attachment_key(b"matrix-x");
        let y_key = test_attachment_key(b"matrix-y");
        let x = attached_test_atom(&symbolica::parse!("x"), &x_key, b"x declaration");
        let y = attached_test_atom(&symbolica::parse!("y"), &y_key, b"y declaration");
        let zero = encode_atom(&Atom::num(0)).unwrap();
        let nested = encode_cbor(Value::Array(vec![
            Value::Array(vec![Value::Bytes(x), Value::Bytes(zero.clone())]),
            Value::Array(vec![Value::Bytes(zero), Value::Bytes(y)]),
        ]))
        .unwrap();

        let matrix = matrix_from_nested(&nested).unwrap();
        assert_eq!(&matrix[..4], MATRIX_PAYLOAD_MAGIC);
        assert_eq!(matrix[4], MATRIX_PAYLOAD_VERSION);
        let decoded = decode_attached_matrix(&matrix, "matrix").unwrap();
        assert_eq!(
            decoded.attachments.get(&x_key),
            Some(b"x declaration".as_slice())
        );
        assert_eq!(
            decoded.attachments.get(&y_key),
            Some(b"y declaration".as_slice())
        );

        let transposed = transpose(&matrix).unwrap();
        let transposed = decode_attached_matrix(&transposed, "transposed").unwrap();
        assert_eq!(
            transposed.attachments.get(&x_key),
            Some(b"x declaration".as_slice())
        );
        assert_eq!(
            transposed.attachments.get(&y_key),
            Some(b"y declaration".as_slice())
        );

        let determinant = det(&matrix).unwrap();
        assert_payload_attachment(&determinant, &x_key, b"x declaration");
        assert_payload_attachment(&determinant, &y_key, b"y declaration");

        let request = encode_cbor(Value::Map(vec![
            (Value::Text("matrix".to_owned()), Value::Bytes(matrix)),
            (Value::Text("row".to_owned()), Value::Integer(0.into())),
            (Value::Text("col".to_owned()), Value::Integer(0.into())),
        ]))
        .unwrap();
        let entry = matrix_at(&request).unwrap();
        assert_payload_attachment(&entry, &x_key, b"x declaration");
        assert_payload_attachment(&entry, &y_key, b"y declaration");
    }
}
