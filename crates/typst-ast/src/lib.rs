//! Shared conversion from Parsely's CBOR tree to a Symbolica Atom.

use std::io::Cursor;

use ciborium::value::Value;
use symbolica::prelude::{Atom, AtomCore, Symbol};
use tymbolica_atom_payload::decode_atom as decode_shared_atom;

/// Decode a CBOR-encoded Parsely tree and convert it to a Symbolica Atom.
pub fn atom_from_ast(input: &[u8], namespace: &str, label: &str) -> Result<Atom, String> {
    tymbolica_symbol_registry::initialize();
    let value = ciborium::from_reader::<Value, _>(Cursor::new(input))
        .map_err(|error| format!("{label} must be CBOR-encoded: {error}"))?;
    atom_from_value(&value, namespace)
}

/// Convert one value in a walked Parsely tree to a Symbolica Atom.
pub fn atom_from_value(value: &Value, namespace: &str) -> Result<Atom, String> {
    tymbolica_symbol_registry::initialize();
    match value {
        Value::Bytes(bytes) => decode_shared_atom(bytes)
            .map_err(|error| format!("embedded Atom payload is invalid: {error}")),
        Value::Integer(number) => {
            let number: i64 = (*number)
                .try_into()
                .map_err(|_| "integer literal is out of range".to_owned())?;
            Ok(Atom::num(number))
        }
        Value::Float(number) => Ok(Atom::num(*number)),
        Value::Text(text) => atom_from_leaf(text, namespace),
        Value::Map(map) => atom_from_node(map, namespace),
        other => Err(format!("unsupported Parsely AST value: {other:?}")),
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
            if let Some(exponent) = slot_atom(slots, "t", namespace) {
                Ok(base.pow(exponent?))
            } else {
                Ok(base)
            }
        }
        "factorial" => {
            let argument = atom_arg(args, 0, head, namespace)?;
            Ok(Symbol::parse("gamma", "symbolica")?.call(argument + Atom::num(1)))
        }
        "semantic-metadata" => atom_from_semantic_metadata(args, slots, namespace),
        "frac" => {
            let numerator = slot_or_arg_atom(slots, "num", args, 0, head, namespace)?;
            let denominator = slot_or_arg_atom(slots, "denom", args, 1, head, namespace)?;
            Ok(numerator / denominator)
        }
        "pow" => {
            let base = slot_or_arg_atom(slots, "base", args, 0, head, namespace)?;
            let exponent = slot_or_arg_atom(slots, "exp", args, 1, head, namespace)?;
            Ok(base.pow(exponent))
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
                .or_else(|| args.first().map(|value| atom_from_value(value, namespace)))
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
            let arguments = slot_atoms(slots, "body", namespace)?.unwrap_or_default();
            Ok(symbol.call_args(arguments))
        }
        "op-call" => {
            let symbol = slot_symbol(slots, "op", namespace)?;
            let arguments = slot_atoms(slots, "args", namespace)?.unwrap_or_default();
            Ok(symbol.call_args(arguments))
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

fn atom_from_semantic_metadata(
    args: &[Value],
    slots: &[(Value, Value)],
    namespace: &str,
) -> Result<Atom, String> {
    let visible = || atom_arg(args, 0, "semantic-metadata", namespace);
    let Some(Value::Map(payload)) = map_get(slots, "value") else {
        return visible();
    };

    if map_get(payload, "protocol") != Some(&Value::Text("tymbolica".to_owned())) {
        return visible();
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
                Some(Value::Bytes(bytes)) => decode_shared_atom(bytes)
                    .map_err(|error| format!("tymbolica metadata Atom is invalid: {error}")),
                Some(other) => Err(format!(
                    "tymbolica metadata atom must be bytes, got {other:?}"
                )),
                None => Err("tymbolica metadata missing atom".to_owned()),
            }
        }
        _ => visible(),
    }
}

fn atoms_from_values(values: &[Value], namespace: &str) -> Result<Vec<Atom>, String> {
    values
        .iter()
        .map(|value| atom_from_value(value, namespace))
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
        .and_then(|value| symbol_from_value(value, namespace))
}

fn atom_arg(args: &[Value], index: usize, head: &str, namespace: &str) -> Result<Atom, String> {
    args.get(index)
        .ok_or_else(|| format!("{head} missing argument {index}"))
        .and_then(|value| atom_from_value(value, namespace))
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
    map_get(slots, key).map(|value| atom_from_value(value, namespace))
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
    use tymbolica_atom_payload::encode_atom;

    fn node(head: &str, args: Vec<Value>, slots: Vec<(Value, Value)>) -> Value {
        Value::Map(vec![
            (Value::Text("head".to_owned()), Value::Text(head.to_owned())),
            (Value::Text("args".to_owned()), Value::Array(args)),
            (Value::Text("slots".to_owned()), Value::Map(slots)),
        ])
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
}
