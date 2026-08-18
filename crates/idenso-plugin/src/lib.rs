//! Idenso tensor transformations over Tymbolica's common Atom payload.

use ciborium::value::Value;
use idenso::color::ColorSimplifier;
use idenso::dirac::GammaSimplifier;
use idenso::selective_expand::SelectiveExpand;
use idenso::shorthands::{metric::MetricSimplifier, schoonschip::Schoonschip};
use idenso::{Cookable, IndexTooling};
use spenso::structure::abstract_index::AbstractIndex;
use symbolica::atom::{Atom, AtomView, Symbol};
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
    idenso::representations::initialize();
    decode_shared_atom(input)
        .map_err(|error| format!("{label} must be Atom payload bytes: {error}"))
}

fn encode_atom(atom: &Atom) -> Result<Vec<u8>, String> {
    encode_shared_atom(atom).map_err(|error| format!("could not encode Idenso result: {error}"))
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
        .map_err(|error| format!("could not encode Idenso result array: {error}"))?;
    Ok(output)
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
