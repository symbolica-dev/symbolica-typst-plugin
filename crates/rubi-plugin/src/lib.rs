//! Rubi integration extension for Tymbolica Atom payloads.

use ciborium::value::Value;
use symbolica::prelude::{Atom, AtomView, Symbol};
use symbolica_integrate::{Integrate, IntegrationExplanation, IntegrationStep};
use tymbolica_atom_payload::{AttachmentSet, encode_atom_from_set, parse_payload};
use wasm_minimal_protocol::*;

initiate_protocol!();

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
#[unsafe(no_mangle)]
unsafe extern "Rust" fn __getrandom_v03_custom(
    destination: *mut u8,
    length: usize,
) -> Result<(), getrandom::Error> {
    if destination.is_null() && length != 0 {
        return Err(getrandom::Error::new_custom(1));
    }

    let mut random_state = 0x9e37_79b9_7f4a_7c15u64 ^ length as u64;
    for index in 0..length {
        random_state ^= random_state << 13;
        random_state ^= random_state >> 7;
        random_state ^= random_state << 17;
        unsafe { destination.add(index).write((random_state >> 56) as u8) };
    }

    Ok(())
}

struct AttachedAtom {
    atom: Atom,
    attachments: AttachmentSet,
}

fn decode_attached_atom(input: &[u8], label: &str) -> Result<AttachedAtom, String> {
    tymbolica_symbol_registry::initialize();
    let payload = parse_payload(input)
        .map_err(|error| format!("{label} must be Atom payload bytes: {error}"))?;
    let attachments = payload.attachment_set();
    tymbolica_symbol_registry::register_representation_attachments(&attachments)
        .map_err(|error| format!("{label} has invalid representation attachments: {error}"))?;
    let atom = payload
        .import_atom()
        .map_err(|error| format!("{label} must be Atom payload bytes: {error}"))?;
    Ok(AttachedAtom { atom, attachments })
}

fn encode_attached_atom(atom: &Atom, attachments: &AttachmentSet) -> Result<Vec<u8>, String> {
    encode_atom_from_set(atom, attachments)
        .map_err(|error| format!("failed to encode Atom payload: {error}"))
}

fn merge_attachments(
    target: &mut AttachmentSet,
    source: &AttachmentSet,
    label: &str,
) -> Result<(), String> {
    target
        .merge(source)
        .map_err(|error| format!("could not merge {label} attachments: {error}"))
}

fn integration_variable(variable: Atom) -> Result<Symbol, String> {
    match variable.as_view() {
        AtomView::Var(variable) => Ok(variable.get_symbol()),
        _ => Err("integration variable must be a symbol".to_owned()),
    }
}

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

fn encode_cbor(value: Value) -> Result<Vec<u8>, String> {
    let mut output = Vec::new();
    ciborium::into_writer(&value, &mut output)
        .map_err(|error| format!("failed to encode CBOR: {error}"))?;
    Ok(output)
}

#[wasm_func]
pub fn integrate(expression: &[u8], variable: &[u8]) -> Result<Vec<u8>, String> {
    let expression = decode_attached_atom(expression, "expr")?;
    let variable = decode_attached_atom(variable, "var")?;
    let mut attachments = expression.attachments;
    merge_attachments(&mut attachments, &variable.attachments, "var")?;
    let result = expression
        .atom
        .integrate(integration_variable(variable.atom)?);
    encode_attached_atom(
        match &result {
            Ok(result) | Err(result) => result,
        },
        &attachments,
    )
}

#[wasm_func]
pub fn integrate_with_steps(expression: &[u8], variable: &[u8]) -> Result<Vec<u8>, String> {
    let expression = decode_attached_atom(expression, "expr")?;
    let variable = decode_attached_atom(variable, "var")?;
    let mut attachments = expression.attachments;
    merge_attachments(&mut attachments, &variable.attachments, "var")?;
    let explanation = expression
        .atom
        .integrate_with_steps(integration_variable(variable.atom)?);
    encode_integration_explanation(explanation, &attachments)
}

fn encode_integration_explanation(
    explanation: IntegrationExplanation,
    attachments: &AttachmentSet,
) -> Result<Vec<u8>, String> {
    let (complete, result) = match explanation.result {
        Ok(result) => (true, result),
        Err(result) => (false, result),
    };
    let steps = explanation
        .steps
        .into_iter()
        .map(|step| integration_step_cbor(step, attachments))
        .collect::<Result<Vec<_>, _>>()?;
    encode_cbor(Value::Map(vec![
        (
            Value::Text("result".to_owned()),
            Value::Bytes(encode_attached_atom(&result, attachments)?),
        ),
        (Value::Text("complete".to_owned()), Value::Bool(complete)),
        (Value::Text("steps".to_owned()), Value::Array(steps)),
    ]))
}

/// Materialize the immutable Symbolica and Rubi tables in Wizer's build-time
/// instance. Wizer removes this export from the delivered module.
#[cfg(feature = "wizer-preinitialize")]
#[unsafe(export_name = "wizer.initialize")]
pub extern "C" fn wizer_initialize() {
    // Symbolica must finish its registered state initialization before Rubi's
    // LazyLocks are entered, otherwise the registry callback re-enters them.
    let _ = symbolica::state::State::is_builtin("x");
    tymbolica_symbol_registry::initialize();
    symbolica_integrate::preinitialize();
}

#[cfg(test)]
#[unsafe(export_name = "wasm_minimal_protocol_send_result_to_host")]
extern "C" fn test_send_result_to_host(_: *const u8, _: usize) {}

#[cfg(test)]
#[unsafe(export_name = "wasm_minimal_protocol_write_args_to_buffer")]
extern "C" fn test_write_args_to_buffer(_: *mut u8) {}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use tymbolica_atom_payload::{Attachment, AttachmentKey, encode_atom, parse_payload};

    fn attachment(identity: &[u8], data: &[u8]) -> Attachment {
        Attachment::new(
            AttachmentKey::new("org.tymbolica.test", 1, identity.to_vec()).unwrap(),
            data.to_vec(),
        )
        .unwrap()
    }

    fn attached_atom(atom: &Atom, attachment: Attachment) -> Vec<u8> {
        let attachments = AttachmentSet::from_attachments([attachment]).unwrap();
        encode_atom_from_set(atom, &attachments).unwrap()
    }

    fn cbor_map(input: &[u8]) -> Vec<(Value, Value)> {
        let Value::Map(map) = ciborium::from_reader::<Value, _>(Cursor::new(input)).unwrap() else {
            panic!("integration explanation must be a dictionary");
        };
        map
    }

    fn map_get<'a>(map: &'a [(Value, Value)], key: &str) -> &'a Value {
        map.iter()
            .find_map(|(candidate, value)| {
                matches!(candidate, Value::Text(candidate) if candidate == key).then_some(value)
            })
            .unwrap_or_else(|| panic!("missing {key}"))
    }

    fn assert_attachments(payload: &[u8], expected: &[(&AttachmentKey, &[u8])]) {
        let parsed = parse_payload(payload).unwrap();
        for (key, data) in expected {
            assert_eq!(parsed.attachment(key), Some(*data));
        }
        let _ = parsed.import_atom().unwrap();
    }

    #[test]
    fn integrate_merges_expression_and_variable_attachments() {
        let expression_key = AttachmentKey::new("org.tymbolica.test", 1, b"expr".to_vec()).unwrap();
        let variable_key = AttachmentKey::new("org.tymbolica.test", 1, b"var".to_vec()).unwrap();
        let expression = attached_atom(
            &symbolica::parse!("x"),
            attachment(b"expr", b"expression metadata"),
        );
        let variable = attached_atom(
            &symbolica::parse!("x"),
            attachment(b"var", b"variable metadata"),
        );

        let result = integrate(&expression, &variable).unwrap();
        assert_attachments(
            &result,
            &[
                (&expression_key, b"expression metadata"),
                (&variable_key, b"variable metadata"),
            ],
        );
        assert_eq!(
            parse_payload(&result).unwrap().import_atom().unwrap(),
            symbolica::parse!("x^2/2")
        );
    }

    #[test]
    fn integrate_rejects_conflicting_attachments() {
        let expression = attached_atom(&symbolica::parse!("x"), attachment(b"shared", b"first"));
        let variable = attached_atom(&symbolica::parse!("x"), attachment(b"shared", b"second"));

        let error = integrate(&expression, &variable).unwrap_err();
        assert!(error.contains("could not merge var attachments"));
    }

    #[test]
    fn step_bridge_preserves_attachments_on_result_and_every_step_atom() {
        let expression_key = AttachmentKey::new("org.tymbolica.test", 1, b"expr".to_vec()).unwrap();
        let variable_key = AttachmentKey::new("org.tymbolica.test", 1, b"var".to_vec()).unwrap();
        let expression = attached_atom(
            &symbolica::parse!("x/(x+1)"),
            attachment(b"expr", b"expression metadata"),
        );
        let variable = attached_atom(
            &symbolica::parse!("x"),
            attachment(b"var", b"variable metadata"),
        );
        let expected = [
            (&expression_key, b"expression metadata".as_slice()),
            (&variable_key, b"variable metadata".as_slice()),
        ];

        let explanation = cbor_map(&integrate_with_steps(&expression, &variable).unwrap());
        assert_eq!(map_get(&explanation, "complete"), &Value::Bool(true));
        let Value::Bytes(result) = map_get(&explanation, "result") else {
            panic!("result must be Atom payload bytes");
        };
        assert_attachments(result, &expected);

        let Value::Array(steps) = map_get(&explanation, "steps") else {
            panic!("steps must be an array");
        };
        assert!(!steps.is_empty());
        for step in steps {
            let Value::Map(step) = step else {
                panic!("step must be a dictionary");
            };
            for field in ["input", "output"] {
                let Value::Bytes(atom) = map_get(step, field) else {
                    panic!("{field} must be Atom payload bytes");
                };
                assert_attachments(atom, &expected);
            }
        }
    }

    #[test]
    fn integration_variable_must_be_a_symbol() {
        let expression = encode_atom(&symbolica::parse!("x")).unwrap();
        let variable = encode_atom(&symbolica::parse!("x+1")).unwrap();
        assert_eq!(
            integrate(&expression, &variable).unwrap_err(),
            "integration variable must be a symbol"
        );
    }
}
