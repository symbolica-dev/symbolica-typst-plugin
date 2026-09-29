//! Semantic expression interchange, independent of Parsely and print layout.
//!
//! Decode and inspect a tree before registering application-specific symbol
//! data, then call [`ExpressionTree::to_atom`] to import it. Native leaf payloads
//! have the same trust and Symbolica-version requirements as [`super::parse_payload`].

use std::io::Cursor;

use ciborium::value::Value;
use serde::{Deserialize, Serialize};
use symbolica::prelude::{
    Atom, AtomCore, AtomView, Coefficient, Complex, Integer, Rational, RealLike, Symbol,
};

use super::{
    AttachedAtom, Attachment, AttachmentKey, AttachmentSet, MAX_PAYLOAD_BYTES, parse_payload,
};

pub const VERSION: u16 = 1;
pub const MAX_DEPTH: usize = 64;
pub const MAX_NODES: usize = 8192;
const MAX_INTEGER_DIGITS: usize = 4096;

/// Exact rational components are decimal strings, including integers too large
/// for CBOR/Typst integers. The denominator must be positive.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RationalNumber {
    pub numerator: String,
    pub denominator: String,
}

impl RationalNumber {
    fn from_rational(value: &Rational) -> Self {
        Self {
            numerator: value.numerator().to_string(),
            denominator: value.denominator().to_string(),
        }
    }

    fn validate(&self) -> Result<(), String> {
        fn digits(s: &str) -> bool {
            !s.is_empty() && s.len() <= MAX_INTEGER_DIGITS && s.bytes().all(|b| b.is_ascii_digit())
        }
        if !digits(self.numerator.strip_prefix('-').unwrap_or(&self.numerator)) {
            return Err(
                "rational numerator must be a decimal integer of at most 4096 digits".into(),
            );
        }
        if !digits(&self.denominator) || self.denominator.bytes().all(|b| b == b'0') {
            return Err(
                "rational denominator must be a positive decimal integer of at most 4096 digits"
                    .into(),
            );
        }
        Ok(())
    }

    fn to_rational(&self) -> Result<Rational, String> {
        self.validate()?;
        Ok(Rational::new(
            self.numerator
                .parse::<Integer>()
                .map_err(|e| e.to_string())?,
            self.denominator
                .parse::<Integer>()
                .map_err(|e| e.to_string())?,
        ))
    }
}

/// A symbol's readable identity and an exact payload preserving its native data,
/// tags, and attributes. The descriptor must agree with the payload on import.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct SymbolLeaf {
    pub name: String,
    pub namespace: String,
    pub short_name: String,
    pub tags: Vec<String>,
    pub attributes: Vec<String>,
    #[serde(with = "byte_string")]
    pub payload: Vec<u8>,
}

impl SymbolLeaf {
    fn from_symbol(symbol: Symbol) -> Result<Self, String> {
        Ok(Self {
            name: symbol.get_name().into(),
            namespace: symbol.get_namespace().into(),
            short_name: symbol.get_stripped_name().into(),
            tags: symbol.get_tags().to_vec(),
            attributes: symbol
                .get_attributes()
                .into_iter()
                .map(|a| super::symbol_attribute_name(a).into())
                .collect(),
            payload: encode_leaf(&Atom::var(symbol))?,
        })
    }

    fn to_symbol(&self) -> Result<Symbol, String> {
        let atom = parse_payload(&self.payload)
            .map_err(|e| e.to_string())?
            .import_atom()
            .map_err(|e| e.to_string())?;
        let AtomView::Var(var) = atom.as_view() else {
            return Err("symbol payload must contain one variable".into());
        };
        let symbol = var.get_symbol();
        let actual = Self::from_symbol(symbol)?;
        if self.name != actual.name
            || self.namespace != actual.namespace
            || self.short_name != actual.short_name
            || self.tags != actual.tags
            || self.attributes != actual.attributes
        {
            return Err("symbol descriptor does not match its payload".into());
        }
        Ok(symbol)
    }
}

/// Algebraic nodes in Symbolica's normalized expression, not source syntax.
/// Subtraction and division are represented by sums/products and negative powers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Node {
    Rational {
        numerator: String,
        denominator: String,
    },
    Complex {
        real: RationalNumber,
        imaginary: RationalNumber,
    },
    /// `real` and `imaginary` are approximate f64 views; `payload` is authoritative
    /// and retains the original precision. Editing a view alone is rejected.
    Float {
        real: f64,
        imaginary: f64,
        #[serde(with = "byte_string")]
        payload: Vec<u8>,
    },
    /// Other coefficient domains (finite fields, rational polynomials, infinity,
    /// indeterminate) stay exact and opaque. Consumers may reject this node.
    Coefficient {
        #[serde(with = "byte_string")]
        payload: Vec<u8>,
    },
    Symbol {
        symbol: SymbolLeaf,
    },
    Call {
        head: SymbolLeaf,
        arguments: Vec<Node>,
    },
    Sum {
        terms: Vec<Node>,
    },
    Product {
        factors: Vec<Node>,
    },
    Power {
        base: Box<Node>,
        exponent: Box<Node>,
    },
}

impl Node {
    fn from_view(view: AtomView<'_>, depth: usize, remaining: &mut usize) -> Result<Self, String> {
        visit(depth, remaining)?;
        let mut child = |v| Self::from_view(v, depth + 1, remaining);
        Ok(match view {
            AtomView::Num(number) => match number.get_coeff_view().to_owned() {
                Coefficient::Complex(c) => {
                    let real = RationalNumber::from_rational(&c.re);
                    if c.im == Rational::zero() {
                        Self::Rational {
                            numerator: real.numerator,
                            denominator: real.denominator,
                        }
                    } else {
                        Self::Complex {
                            real,
                            imaginary: RationalNumber::from_rational(&c.im),
                        }
                    }
                }
                Coefficient::Float(c) => Self::Float {
                    real: c.re.to_f64(),
                    imaginary: c.im.to_f64(),
                    payload: encode_leaf(&view.to_owned())?,
                },
                _ => Self::Coefficient {
                    payload: encode_leaf(&view.to_owned())?,
                },
            },
            AtomView::Var(v) => Self::Symbol {
                symbol: SymbolLeaf::from_symbol(v.get_symbol())?,
            },
            AtomView::Fun(f) => Self::Call {
                head: SymbolLeaf::from_symbol(f.get_symbol())?,
                arguments: f.iter().map(&mut child).collect::<Result<_, _>>()?,
            },
            AtomView::Add(a) => Self::Sum {
                terms: a.iter().map(&mut child).collect::<Result<_, _>>()?,
            },
            AtomView::Mul(m) => Self::Product {
                factors: m.iter().map(&mut child).collect::<Result<_, _>>()?,
            },
            AtomView::Pow(p) => {
                let (base, exponent) = p.get_base_exp();
                Self::Power {
                    base: Box::new(child(base)?),
                    exponent: Box::new(child(exponent)?),
                }
            }
        })
    }

    fn validate(
        &self,
        depth: usize,
        remaining: &mut usize,
        attachments: &mut AttachmentSet,
    ) -> Result<(), String> {
        visit(depth, remaining)?;
        let mut leaf = |bytes: &[u8]| {
            let parsed = parse_payload(bytes).map_err(|e| e.to_string())?;
            attachments
                .merge(&parsed.attachment_set())
                .map_err(|e| e.to_string())
        };
        match self {
            Self::Rational {
                numerator,
                denominator,
            } => RationalNumber {
                numerator: numerator.clone(),
                denominator: denominator.clone(),
            }
            .validate()?,
            Self::Complex { real, imaginary } => {
                real.validate()?;
                imaginary.validate()?;
            }
            Self::Float { payload, .. } | Self::Coefficient { payload } => leaf(payload)?,
            Self::Symbol { symbol } => leaf(&symbol.payload)?,
            Self::Call { head, arguments } => {
                leaf(&head.payload)?;
                for arg in arguments {
                    arg.validate(depth + 1, remaining, attachments)?;
                }
            }
            Self::Sum { terms } | Self::Product { factors: terms } => {
                for term in terms {
                    term.validate(depth + 1, remaining, attachments)?;
                }
            }
            Self::Power { base, exponent } => {
                base.validate(depth + 1, remaining, attachments)?;
                exponent.validate(depth + 1, remaining, attachments)?;
            }
        }
        Ok(())
    }

    fn to_atom(&self) -> Result<Atom, String> {
        Ok(match self {
            Self::Rational {
                numerator,
                denominator,
            } => Atom::num(
                RationalNumber {
                    numerator: numerator.clone(),
                    denominator: denominator.clone(),
                }
                .to_rational()?,
            ),
            Self::Complex { real, imaginary } => {
                Atom::num(Complex::new(real.to_rational()?, imaginary.to_rational()?))
            }
            Self::Float {
                real,
                imaginary,
                payload,
            } => {
                let atom = parse_payload(payload)
                    .map_err(|e| e.to_string())?
                    .import_atom()
                    .map_err(|e| e.to_string())?;
                let AtomView::Num(n) = atom.as_view() else {
                    return Err("float payload must contain a number".into());
                };
                let Coefficient::Float(c) = n.get_coeff_view().to_owned() else {
                    return Err("float payload must contain a floating-point coefficient".into());
                };
                let equal = |a: f64, b: f64| a == b || (a.is_nan() && b.is_nan());
                if !equal(*real, c.re.to_f64()) || !equal(*imaginary, c.im.to_f64()) {
                    return Err("float approximation does not match its payload".into());
                }
                atom
            }
            Self::Coefficient { payload } => {
                let atom = parse_payload(payload)
                    .map_err(|e| e.to_string())?
                    .import_atom()
                    .map_err(|e| e.to_string())?;
                if !matches!(atom.as_view(), AtomView::Num(_)) {
                    return Err("coefficient payload must contain a number".into());
                }
                atom
            }
            Self::Symbol { symbol } => Atom::var(symbol.to_symbol()?),
            Self::Call { head, arguments } => head.to_symbol()?.call_args(
                arguments
                    .iter()
                    .map(Self::to_atom)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            Self::Sum { terms } => Atom::add_many(
                terms
                    .iter()
                    .map(Self::to_atom)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            Self::Product { factors } => Atom::mul_many(
                factors
                    .iter()
                    .map(Self::to_atom)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            Self::Power { base, exponent } => base.to_atom()?.pow(exponent.to_atom()?),
        })
    }
}

// The complete annotation environment lives on the tree. Regenerating it in
// each leaf could conflict with equivalent declarations from another runtime.
fn encode_leaf(atom: &Atom) -> Result<Vec<u8>, String> {
    let raw = super::export_raw_atom(atom).map_err(|e| e.to_string())?;
    super::encode_exported_atom_from_set(&raw, &AttachmentSet::new()).map_err(|e| e.to_string())
}

fn visit(depth: usize, remaining: &mut usize) -> Result<(), String> {
    if depth > MAX_DEPTH || *remaining == 0 {
        return Err("expression tree exceeds the depth or node limit".into());
    }
    *remaining -= 1;
    Ok(())
}

/// A scalar expression and its complete portable attachment environment.
#[derive(Clone, Debug, PartialEq)]
pub struct ExpressionTree {
    pub root: Node,
    pub attachments: AttachmentSet,
}

impl ExpressionTree {
    pub fn from_atom(atom: &Atom, attachments: &AttachmentSet) -> Result<Self, String> {
        let mut remaining = MAX_NODES;
        let tree = Self {
            root: Node::from_view(atom.as_view(), 0, &mut remaining)?,
            attachments: super::with_math_displays(atom, attachments).map_err(|e| e.to_string())?,
        };
        tree.validate()?;
        Ok(tree)
    }

    /// Validate and merge every leaf's attachments without importing any Atom.
    pub fn validate(&self) -> Result<AttachmentSet, String> {
        let mut attachments = self.attachments.clone();
        let mut remaining = MAX_NODES;
        self.root.validate(0, &mut remaining, &mut attachments)?;
        Ok(attachments)
    }

    /// Import native leaves and rebuild the algebra, applying Symbolica's normal
    /// canonicalization. Register attachment-specific readers before this call.
    pub fn to_atom(&self) -> Result<AttachedAtom, String> {
        let attachments = self.validate()?;
        Ok(AttachedAtom {
            atom: self.root.to_atom()?,
            attachments,
        })
    }

    pub fn to_value(&self) -> Result<Value, String> {
        let attachments = self.validate()?;
        Value::serialized(&WireTree {
            protocol: "symbolica".into(),
            version: VERSION,
            kind: "expression-tree".into(),
            root: self.root.clone(),
            attachments: attachments
                .iter()
                .map(|a| WireAttachment {
                    schema: a.schema().into(),
                    version: a.version(),
                    identity: a.identity().into(),
                    data: a.data().into(),
                })
                .collect(),
        })
        .map_err(|e| e.to_string())
    }

    /// Validate the contract and native payload envelopes, without importing or
    /// registering symbols. Embedded native Atom bodies are not validated here.
    pub fn from_value(value: &Value) -> Result<Self, String> {
        // Bound already-decoded values too, before Serde recursively visits them.
        let mut pending = vec![(value, 0usize)];
        let mut bytes = 0usize;
        let mut values = 0usize;
        while let Some((value, depth)) = pending.pop() {
            values += 1;
            bytes = bytes.saturating_add(match value {
                Value::Text(s) => s.len(),
                Value::Bytes(b) => b.len(),
                _ => 1,
            });
            if depth > 192 || values > MAX_NODES * 32 || bytes > MAX_PAYLOAD_BYTES {
                return Err("expression tree exceeds the value limits".into());
            }
            match value {
                Value::Array(items) => pending.extend(items.iter().map(|v| (v, depth + 1))),
                Value::Map(fields) => pending.extend(
                    fields
                        .iter()
                        .flat_map(|(k, v)| [(k, depth + 1), (v, depth + 1)]),
                ),
                Value::Tag(_, inner) => pending.push((inner, depth + 1)),
                _ => {}
            }
        }
        let wire: WireTree = value
            .deserialized()
            .map_err(|e| format!("invalid expression tree: {e}"))?;
        if wire.protocol != "symbolica" || wire.version != VERSION || wire.kind != "expression-tree"
        {
            return Err("unsupported expression tree protocol, version, or kind".into());
        }
        let mut attachments = AttachmentSet::new();
        for a in wire.attachments {
            let key =
                AttachmentKey::new(a.schema, a.version, a.identity).map_err(|e| e.to_string())?;
            attachments
                .insert(Attachment::new(key, a.data).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        }
        let mut tree = Self {
            root: wire.root,
            attachments,
        };
        tree.attachments = tree.validate()?;
        Ok(tree)
    }

    pub fn encode(&self) -> Result<Vec<u8>, String> {
        let mut bytes = Vec::new();
        ciborium::into_writer(&self.to_value()?, &mut bytes).map_err(|e| e.to_string())?;
        if bytes.len() > MAX_PAYLOAD_BYTES {
            return Err("expression tree exceeds the byte limit".into());
        }
        Ok(bytes)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_PAYLOAD_BYTES {
            return Err("expression tree exceeds the byte limit".into());
        }
        let mut input = Cursor::new(bytes);
        let value = ciborium::de::from_reader_with_recursion_limit::<Value, _>(&mut input, 192)
            .map_err(|e| format!("invalid expression tree CBOR: {e}"))?;
        if input.position() != bytes.len() as u64 {
            return Err("trailing data after expression tree".into());
        }
        Self::from_value(&value)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireTree {
    protocol: String,
    version: u16,
    kind: String,
    root: Node,
    attachments: Vec<WireAttachment>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireAttachment {
    schema: String,
    version: u32,
    #[serde(with = "byte_string")]
    identity: Vec<u8>,
    #[serde(with = "byte_string")]
    data: Vec<u8>,
}

mod byte_string {
    use serde::{Deserialize, Deserializer, Serializer, de::Error};
    pub fn serialize<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(bytes)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        match ciborium::value::Value::deserialize(deserializer)? {
            ciborium::value::Value::Bytes(bytes) => Ok(bytes),
            _ => Err(D::Error::custom("expected CBOR bytes")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::payload::math_display::MathDisplay;
    use symbolica::prelude::Float;
    use symbolica::{parse, symbol};

    fn tree(atom: &Atom) -> ExpressionTree {
        ExpressionTree::from_atom(atom, &AttachmentSet::new()).unwrap()
    }
    fn roundtrip(atom: &Atom) {
        let original = tree(atom);
        let decoded = ExpressionTree::decode(&original.encode().unwrap()).unwrap();
        assert_eq!(decoded.to_atom().unwrap().atom, *atom);
    }
    fn field_mut<'a>(value: &'a mut Value, key: &str) -> &'a mut Value {
        value
            .as_map_mut()
            .unwrap()
            .iter_mut()
            .find(|(k, _)| k.as_text() == Some(key))
            .map(|(_, v)| v)
            .unwrap()
    }

    #[test]
    fn normalized_algebra_and_exact_numbers_roundtrip() {
        for atom in [
            parse!("x^2/(1+x)"),
            parse!("f(x, y^2)+3*x-2/7"),
            parse!("123456789012345678901234567890123456789/7"),
            Atom::num(Complex::new(Rational::new(2, 3), Rational::new(-5, 7))),
            Atom::num(0),
            Atom::num(1),
        ] {
            roundtrip(&atom);
        }
        let t = tree(&parse!("2/7"));
        assert_eq!(
            t.root,
            Node::Rational {
                numerator: "2".into(),
                denominator: "7".into()
            }
        );
    }

    #[test]
    fn preserves_float_precision_and_rejects_inconsistent_views() {
        let atom = Atom::num(Coefficient::Float(Complex::new(
            Float::parse("1.2345678901234567890123456789", Some(180)).unwrap(),
            Float::with_val(180, -0.75),
        )));
        roundtrip(&atom);
        let mut t = tree(&atom);
        let Node::Float { real, .. } = &mut t.root else {
            panic!()
        };
        *real = 99.0;
        assert!(t.to_atom().unwrap_err().contains("approximation"));
        roundtrip(&Atom::num(Coefficient::Indeterminate));
    }

    #[test]
    fn labels_function_heads_and_foreign_attachments_survive_edits() {
        let display = MathDisplay::Attach {
            base: Box::new(MathDisplay::Symbol("h".into())),
            slots: [
                None,
                Some(Box::new(MathDisplay::Sequence(vec![
                    MathDisplay::Symbol("j".into()),
                    MathDisplay::Symbol("i".into()),
                ]))),
                None,
                None,
                None,
                None,
            ],
        };
        let head = display.register("expression_tree_labels").unwrap();
        let tagged =
            symbol!("expression_tree_labels::tagged"; Symmetric; tags = ["consumer::test"]);
        let atom = head.call(Atom::var(tagged));
        let foreign = Attachment::new(
            AttachmentKey::new("consumer.data", 1, b"object".to_vec()).unwrap(),
            b"opaque data".to_vec(),
        )
        .unwrap();
        let attachments = AttachmentSet::from_attachments([foreign.clone()]).unwrap();
        let original = ExpressionTree::from_atom(&atom, &attachments).unwrap();
        let mut decoded = ExpressionTree::decode(&original.encode().unwrap()).unwrap();
        assert_eq!(decoded.to_atom().unwrap().atom, atom);
        let Node::Call { arguments, .. } = &mut decoded.root else {
            panic!()
        };
        arguments.push(Node::Rational {
            numerator: "2".into(),
            denominator: "1".into(),
        });
        let edited = decoded.to_atom().unwrap();
        assert_eq!(
            edited.atom,
            head.call_args(vec![Atom::var(tagged), Atom::num(2)])
        );
        assert_eq!(edited.attachments.get(foreign.key()), Some(foreign.data()));
        assert!(
            edited
                .attachments
                .iter()
                .any(|a| a.schema() == super::super::math_display::MATH_DISPLAY_SCHEMA)
        );
    }

    #[test]
    fn editing_children_rebuilds_and_normalizes_the_result() {
        let x = tree(&parse!("x")).root;
        let t = ExpressionTree {
            root: Node::Sum {
                terms: vec![x.clone(), x],
            },
            attachments: AttachmentSet::new(),
        };
        assert_eq!(t.to_atom().unwrap().atom, parse!("2*x"));
        let empty = ExpressionTree {
            root: Node::Product { factors: vec![] },
            attachments: AttachmentSet::new(),
        };
        assert_eq!(empty.to_atom().unwrap().atom, Atom::num(1));
    }

    #[test]
    fn validates_schema_version_fields_and_rationals() {
        let t = tree(&Atom::num(3));
        let value = t.to_value().unwrap();
        let mut bad = value.clone();
        *field_mut(&mut bad, "version") = Value::Integer(2.into());
        assert!(
            ExpressionTree::from_value(&bad)
                .unwrap_err()
                .contains("version")
        );
        let mut bad = value.clone();
        field_mut(&mut bad, "root")
            .as_map_mut()
            .unwrap()
            .push((Value::Text("ignored".into()), Value::Bool(true)));
        assert!(ExpressionTree::from_value(&bad).is_err());
        let mut bad = value.clone();
        field_mut(&mut bad, "root")
            .as_map_mut()
            .unwrap()
            .push((Value::Text("numerator".into()), Value::Text("9".into())));
        assert!(ExpressionTree::from_value(&bad).is_err());
        let mut bad = value;
        *field_mut(field_mut(&mut bad, "root"), "denominator") = Value::Text("0".into());
        assert!(
            ExpressionTree::from_value(&bad)
                .unwrap_err()
                .contains("positive")
        );
        let bytes = t.encode().unwrap();
        for end in 0..bytes.len() {
            assert!(ExpressionTree::decode(&bytes[..end]).is_err());
        }
        assert!(
            ExpressionTree::decode(&[bytes, vec![0]].concat())
                .unwrap_err()
                .contains("trailing")
        );
    }

    #[test]
    fn preflight_does_not_import_native_leaves_and_import_checks_identity() {
        let mut t = tree(&parse!("expression_tree_preflight::x"));
        let Node::Symbol { symbol } = &mut t.root else {
            panic!()
        };
        symbol.name = "expression_tree_preflight::different".into();
        let decoded = ExpressionTree::decode(&t.encode().unwrap()).unwrap();
        assert!(decoded.to_atom().unwrap_err().contains("descriptor"));
        // A valid envelope with only a native export header can be inspected.
        // Do not import this deliberately incomplete, untrusted native body.
        let Node::Symbol { symbol } = &mut t.root else {
            panic!()
        };
        let raw = super::super::export_raw_atom(&Atom::num(1)).unwrap();
        symbol.payload =
            super::super::encode_exported_atom_from_set(&raw[..8], &AttachmentSet::new()).unwrap();
        assert!(ExpressionTree::decode(&t.encode().unwrap()).is_ok());
    }

    #[test]
    fn depth_and_node_limits_are_enforced() {
        let mut root = tree(&Atom::num(1)).root;
        for _ in 0..MAX_DEPTH + 1 {
            root = Node::Sum { terms: vec![root] };
        }
        let t = ExpressionTree {
            root,
            attachments: AttachmentSet::new(),
        };
        assert!(t.validate().unwrap_err().contains("limit"));
        let t = ExpressionTree {
            root: Node::Sum {
                terms: vec![tree(&Atom::num(1)).root; MAX_NODES],
            },
            attachments: AttachmentSet::new(),
        };
        assert!(t.validate().unwrap_err().contains("limit"));
    }
}
