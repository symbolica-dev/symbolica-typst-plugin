use ciborium::value::Value;
use symbolica::atom::Atom;
use symbolica_typst_plugin::{
    expression_tree::{ExpressionTree, Node},
    math_display::MathDisplay,
    payload::{Attachment, AttachmentKey, AttachmentSet, encode_atom_from_set, parse_payload},
    typst_ast::{attached_atom_from_value, preflight_payloads_from_value},
};

// Exercise the public API as another plugin would: inspect declarations before
// import, transform an expression, and return its notation and unknown sidecars.
#[test]
fn a_consumer_preserves_notation_and_foreign_attachments() {
    let display = MathDisplay::Attach {
        base: Box::new(MathDisplay::Symbol("C".into())),
        slots: [
            None,
            Some(Box::new(MathDisplay::Sequence(vec![
                MathDisplay::Symbol("i".into()),
                MathDisplay::Symbol("j".into()),
            ]))),
            None,
            None,
            None,
            None,
        ],
    };
    let symbol = display.register("shared_api_test").unwrap();
    let variable = Atom::var(symbol);
    let key = AttachmentKey::new("other-plugin.tensor", 1, b"C".to_vec()).unwrap();
    let foreign_data = b"opaque declaration";
    let attachments =
        AttachmentSet::from_attachments([
            Attachment::new(key.clone(), foreign_data.to_vec()).unwrap()
        ])
        .unwrap();
    let input = Value::Bytes(encode_atom_from_set(&variable, &attachments).unwrap());

    let preflight = preflight_payloads_from_value(&input).unwrap();
    assert_eq!(
        preflight.attachments.get(&key),
        Some(foreign_data.as_slice())
    );
    let parsed = attached_atom_from_value(&input, "consumer_namespace").unwrap();
    assert_eq!(parsed.attachments, preflight.attachments);
    assert_eq!(parsed.atom, variable);

    let transformed = parsed.atom * Atom::num(2);
    let output = encode_atom_from_set(&transformed, &parsed.attachments).unwrap();
    let received = parse_payload(&output).unwrap();
    assert_eq!(received.attachment(&key), Some(foreign_data.as_slice()));
    assert_eq!(received.import_atom().unwrap(), transformed);
    assert_eq!(MathDisplay::from_symbol(symbol).unwrap(), Some(display));
}

#[test]
fn a_consumer_edits_a_semantic_tree_without_parsely() {
    use symbolica::atom::AtomCore;
    let input = Atom::num(2).pow(Atom::num(3));
    let tree = ExpressionTree::from_atom(&input, &AttachmentSet::new()).unwrap();
    let mut decoded = ExpressionTree::decode(&tree.encode().unwrap()).unwrap();
    assert_eq!(
        decoded.root,
        Node::Rational {
            numerator: "8".into(),
            denominator: "1".into()
        }
    );
    decoded.root = Node::Sum {
        terms: vec![
            decoded.root,
            Node::Rational {
                numerator: "1".into(),
                denominator: "3".into(),
            },
        ],
    };
    let result = decoded.to_atom().unwrap();
    assert_eq!(result.atom, Atom::num(25) / Atom::num(3));
}
