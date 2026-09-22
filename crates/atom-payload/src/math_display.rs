//! Portable, non-executable notation for decorated mathematical symbols.
//!
//! Juxtaposition in a label is an ordered sequence, not multiplication. Exact
//! embedded atoms retain their native identity in Symbolica's symbol data.

use ciborium::value::Value;
use symbolica::{
    atom::{
        Atom, AtomCore, AtomView, DefaultNamespace, NamespacedSymbol, Symbol, SymbolBuilder,
        UserData,
    },
    parser::Token,
    printer::PrintOptions,
};

use crate::{Attachment, AttachmentKey, AttachmentSet, parse_payload};

pub const MATH_DISPLAY_SCHEMA: &str = "symbolica.math-display";
pub const MATH_DISPLAY_VERSION: u32 = 1;
pub const ATTACHMENT_SLOTS: [&str; 6] = ["t", "b", "tl", "tr", "bl", "br"];
const MAX_DEPTH: usize = 64;
const MAX_NODES: usize = 4096;
const MAX_TEXT: usize = 4096;
const MAX_BYTES: usize = crate::MAX_ATTACHMENT_DATA_BYTES;
const SYMBOL_PREFIX: &str = "__math_display_";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MathDisplay {
    Symbol(String),
    Text(String),
    Number(String),
    Sequence(Vec<Self>),
    List(Vec<Self>),
    Math {
        head: String,
        arguments: Vec<Self>,
    },
    Attach {
        base: Box<Self>,
        slots: [Option<Box<Self>>; 6],
    },
    Primes(u32),
    /// Exact semantics and portable sidecars from an interpolated Atom.
    Atom {
        atom: Atom,
        attachments: AttachmentSet,
    },
}

impl MathDisplay {
    pub fn from_ast(value: &Value) -> Result<Self, String> {
        let display = parse_ast(value, 0, &mut 0)?;
        display.validate()?;
        Ok(display)
    }

    pub fn from_value(value: &Value) -> Result<Self, String> {
        let display = decode_node(value, 0, &mut 0)?;
        display.validate()?;
        Ok(display)
    }

    /// A tagged, version-independent node value; its containing declaration
    /// carries the schema version.
    pub fn to_value(&self) -> Value {
        self.node_value(false)
    }

    fn node_value(&self, identity: bool) -> Value {
        let array = |kind: &str, mut fields: Vec<Value>| {
            fields.insert(0, Value::Text(kind.to_owned()));
            Value::Array(fields)
        };
        let children =
            |items: &[Self]| Value::Array(items.iter().map(|x| x.node_value(identity)).collect());
        match self {
            Self::Symbol(text) => array("symbol", vec![Value::Text(text.clone())]),
            Self::Text(text) => array("text", vec![Value::Text(text.clone())]),
            Self::Number(text) => array("number", vec![Value::Text(text.clone())]),
            Self::Sequence(items) => array("sequence", vec![children(items)]),
            Self::List(items) => array("list", vec![children(items)]),
            Self::Math { head, arguments } => {
                array("math", vec![Value::Text(head.clone()), children(arguments)])
            }
            Self::Attach { base, slots } => {
                let mut fields = vec![base.node_value(identity)];
                fields.extend(
                    slots
                        .iter()
                        .map(|x| x.as_ref().map_or(Value::Null, |x| x.node_value(identity))),
                );
                array("attach", fields)
            }
            Self::Primes(count) => array("primes", vec![Value::Integer((*count).into())]),
            Self::Atom { atom, attachments } if identity => array(
                "atom-identity",
                vec![
                    Value::Text(atom.to_canonical_string()),
                    // Sidecars are part of this declaration, so equal identities
                    // never register conflicting per-symbol metadata.
                    Value::Bytes(
                        encode_display_atom(&Atom::Zero, attachments)
                            .expect("validated attachments"),
                    ),
                ],
            ),
            Self::Atom { atom, attachments } => array(
                "atom",
                vec![Value::Bytes(
                    encode_display_atom(atom, attachments).expect("validated display atom"),
                )],
            ),
        }
    }

    fn validate(&self) -> Result<(), String> {
        validate_display(self, 0, &mut 0)?;
        if encode_value(&self.to_value())?.len() > MAX_BYTES {
            return Err(format!("math display exceeds the {MAX_BYTES}-byte limit"));
        }
        Ok(())
    }

    pub fn symbol_name(&self, namespace: &str) -> Result<String, String> {
        self.validate()?;
        if namespace.is_empty()
            || namespace
                .split("::")
                .any(|p| p.is_empty() || p.chars().any(|c| c.is_control() || c.is_whitespace()))
        {
            return Err("math display requires a nonempty canonical namespace".to_owned());
        }
        let mut hash = blake3::Hasher::new();
        hash.update(b"symbolica.math-display\0v1\0");
        hash.update(&encode_value(&self.node_value(true))?);
        Ok(format!(
            "{namespace}::{SYMBOL_PREFIX}{}",
            hash.finalize().to_hex()
        ))
    }

    /// Register a self-contained display symbol without custom callbacks.
    /// Existing hashed registrations retain their original version-one format.
    pub fn register(&self, namespace: &str) -> Result<Symbol, String> {
        self.register_hashed(namespace, Vec::new())
    }

    fn register_hashed(&self, namespace: &str, tags: Vec<String>) -> Result<Symbol, String> {
        let name = self.symbol_name(namespace)?;
        self.register_display(NamespacedSymbol::parse(&name), tags, false)
    }

    /// Create one ordinary variable whose appearance is this complete tree.
    ///
    /// Bare symbol leaves reuse the usual named symbol identity. Other unnamed
    /// displays use the same identity as parsed decorated symbols. An explicit
    /// `name` associates the complete display with that named ordinary symbol;
    /// changing its display or tags is an incompatible redeclaration.
    pub fn register_literal(
        &self,
        namespace: &str,
        name: Option<&str>,
        tags: Vec<String>,
    ) -> Result<Symbol, String> {
        self.validate()?;
        validate_tags(&tags)?;
        if let Some(name) = name {
            let name = literal_name(name, namespace)?;
            return self.register_display(name, tags, true);
        }
        if let Self::Symbol(name) = self {
            // Visible underscores are permitted as content. Give such labels
            // a hashed ordinary identity rather than a wildcard identity.
            if !name.ends_with('_')
                && !name
                    .rsplit("::")
                    .next()
                    .unwrap_or(name)
                    .starts_with(SYMBOL_PREFIX)
                && Token::check_symbol_name(name.rsplit("::").next().unwrap_or(name)).is_ok()
            {
                let qualified = literal_name(name, namespace)?;
                return SymbolBuilder::new(qualified)
                    .with_tags(tags)
                    .build()
                    .map_err(|error| error.to_string());
            }
        }
        self.register_hashed(namespace, tags)
    }

    fn register_display(
        &self,
        name: NamespacedSymbol,
        tags: Vec<String>,
        named: bool,
    ) -> Result<Symbol, String> {
        let mut fields = vec![
            UserData::String(MATH_DISPLAY_SCHEMA.to_owned()),
            UserData::Integer(MATH_DISPLAY_VERSION.into()),
            self.user_data()?,
        ];
        if named {
            // This optional identity field extends the native declaration;
            // the portable display node and schema version remain unchanged.
            fields.push(UserData::List(vec![
                UserData::String("named".to_owned()),
                UserData::String(name.symbol.to_string()),
            ]));
        }
        SymbolBuilder::new(name)
            .with_tags(tags)
            .with_user_data(UserData::List(fields))
            .build()
            .map_err(|error| error.to_string())
    }

    pub fn from_symbol(symbol: Symbol) -> Result<Option<Self>, String> {
        let UserData::List(fields) = symbol.get_data() else {
            return Ok(None);
        };
        if !matches!(fields.first(), Some(UserData::String(schema)) if schema == MATH_DISPLAY_SCHEMA)
        {
            return Ok(None);
        }
        let (version, data, named) = match fields.as_slice() {
            [_, UserData::Integer(version), data] => (*version, data, None),
            [
                _,
                UserData::Integer(version),
                data,
                UserData::List(identity),
            ] => match identity.as_slice() {
                [UserData::String(kind), UserData::String(name)] if kind == "named" => {
                    (*version, data, Some(name.as_str()))
                }
                _ => return Err("invalid named math-display identity".to_owned()),
            },
            _ => return Err("invalid math-display symbol declaration".to_owned()),
        };
        if version != i64::from(MATH_DISPLAY_VERSION) {
            return Err(format!("unsupported math-display symbol version {version}"));
        }
        let display = Self::from_value(&user_data_value(data, 0, &mut 0)?)?;
        if let Some(name) = named {
            let declared = literal_name(name, symbol.get_namespace())?;
            if declared.symbol.as_ref() != symbol.get_name() {
                return Err("named math-display identity does not match its symbol".to_owned());
            }
        } else if display.symbol_name(symbol.get_namespace())? != symbol.get_name() {
            return Err("math-display symbol identity does not match its data".to_owned());
        }
        if symbol.get_wildcard_level() != 0 {
            return Err("a math-display literal cannot be a wildcard".to_owned());
        }
        Ok(Some(display))
    }

    fn user_data(&self) -> Result<UserData, String> {
        match self {
            Self::Atom { atom, attachments } => Ok(UserData::List(vec![
                UserData::String("atom".to_owned()),
                UserData::Atom(atom.clone()),
                UserData::Serialized(
                    encode_display_atom(&Atom::Zero, attachments).map_err(|e| e.to_string())?,
                ),
            ])),
            Self::Sequence(items) | Self::List(items) => Ok(UserData::List(vec![
                UserData::String(
                    if matches!(self, Self::Sequence(_)) {
                        "sequence"
                    } else {
                        "list"
                    }
                    .to_owned(),
                ),
                UserData::List(
                    items
                        .iter()
                        .map(Self::user_data)
                        .collect::<Result<_, _>>()?,
                ),
            ])),
            Self::Math { head, arguments } => Ok(UserData::List(vec![
                UserData::String("math".to_owned()),
                UserData::String(head.clone()),
                UserData::List(
                    arguments
                        .iter()
                        .map(Self::user_data)
                        .collect::<Result<_, _>>()?,
                ),
            ])),
            Self::Attach { base, slots } => {
                let mut values = vec![UserData::String("attach".to_owned()), base.user_data()?];
                for slot in slots {
                    values.push(
                        slot.as_ref()
                            .map_or(Ok(UserData::None), |x| x.user_data())?,
                    );
                }
                Ok(UserData::List(values))
            }
            _ => value_user_data(&self.to_value()),
        }
    }

    pub fn attachment(&self, symbol: Symbol) -> Result<Attachment, String> {
        if Self::from_symbol(symbol)?.as_ref() != Some(self) {
            return Err("math display attachment does not match its symbol declaration".to_owned());
        }
        Attachment::new(
            AttachmentKey::new(
                MATH_DISPLAY_SCHEMA,
                MATH_DISPLAY_VERSION,
                symbol.get_name().as_bytes(),
            )
            .map_err(|e| e.to_string())?,
            encode_value(&self.to_value())?,
        )
        .map_err(|e| e.to_string())
    }

    pub fn embedded_attachments(&self) -> Result<AttachmentSet, String> {
        let mut result = AttachmentSet::new();
        self.collect_attachments(&mut result)?;
        Ok(result)
    }

    fn collect_attachments(&self, result: &mut AttachmentSet) -> Result<(), String> {
        match self {
            Self::Atom { attachments, .. } => {
                result.merge(attachments).map_err(|e| e.to_string())?
            }
            Self::Sequence(items)
            | Self::List(items)
            | Self::Math {
                arguments: items, ..
            } => {
                for item in items {
                    item.collect_attachments(result)?;
                }
            }
            Self::Attach { base, slots } => {
                base.collect_attachments(result)?;
                for item in slots.iter().flatten() {
                    item.collect_attachments(result)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Emit only fixed Typst functions and escaped string literals. No input
    /// string is treated as executable Typst source.
    pub fn to_typst_source(&self) -> String {
        self.source_at_depth(0)
    }

    /// Source for this display when it denotes one indivisible symbol.
    /// Composite notation is grouped even when the surrounding algebra sees a
    /// variable, preventing a label `x+y` from rendering its square as `x+y^2`.
    pub fn to_symbol_typst_source(&self) -> String {
        self.symbol_source_at_depth(0)
    }

    fn symbol_source_at_depth(&self, depth: usize) -> String {
        let source = self.source_at_depth(depth);
        if self.needs_symbol_grouping() {
            format!("lr(({source}))")
        } else {
            source
        }
    }

    fn needs_symbol_grouping(&self) -> bool {
        match self {
            Self::Sequence(items) | Self::List(items) => match items.as_slice() {
                [item] => item.needs_symbol_grouping(),
                _ => !items.is_empty(),
            },
            Self::Math { head, .. } => matches!(
                head.as_str(),
                "arg"
                    | "add"
                    | "sub"
                    | "plus"
                    | "neg"
                    | "times"
                    | "dot"
                    | "mul"
                    | "sequence"
                    | "union"
                    | "inter"
                    | "pow"
            ),
            Self::Attach { base, slots } => slots[0].is_some() || base.needs_symbol_grouping(),
            Self::Atom { atom, .. } => matches!(
                atom.as_view(),
                AtomView::Add(_) | AtomView::Mul(_) | AtomView::Pow(_)
            ),
            Self::Number(number) => number.starts_with('-'),
            _ => false,
        }
    }

    fn source_at_depth(&self, depth: usize) -> String {
        if depth > MAX_DEPTH {
            return "upright(\"?\")".to_owned();
        }
        match self {
            Self::Symbol(text)
                if text.chars().count() == 1 && text.chars().all(char::is_alphanumeric) =>
            {
                text.clone()
            }
            Self::Symbol(text) if text.chars().count() == 1 => {
                format!("#symbol({})", quoted(text))
            }
            Self::Symbol(text) => format!("italic({})", quoted(text)),
            Self::Text(text) => format!("upright({})", quoted(text)),
            Self::Number(text) if valid_number(text) => text.clone(),
            Self::Number(text) => format!("upright({})", quoted(text)),
            Self::Sequence(items) if items.is_empty() => "\"\"".to_owned(),
            Self::Sequence(items) => items
                .iter()
                .map(|item| item.source_at_depth(depth + 1))
                .collect::<Vec<_>>()
                .join(" "),
            Self::List(items) => items
                .iter()
                .map(|item| item.source_at_depth(depth + 1))
                .collect::<Vec<_>>()
                .join(","),
            Self::Primes(count) => format!("primes(#{count})"),
            Self::Attach { base, slots } => {
                let mut result = format!("attach({}", base.source_at_depth(depth + 1));
                for (name, item) in ATTACHMENT_SLOTS.iter().zip(slots) {
                    if let Some(item) = item {
                        result.push_str(&format!(",{name}:{}", item.source_at_depth(depth + 1)));
                    }
                }
                result.push(')');
                result
            }
            Self::Math { head, arguments } => math_source(head, arguments, depth + 1),
            Self::Atom { atom, .. } => atom_source(atom.as_view(), depth + 1),
        }
    }
}

fn validate_tags(tags: &[String]) -> Result<(), String> {
    for (index, tag) in tags.iter().enumerate() {
        if !tag.contains("::")
            || tag.split("::").any(|part| part.is_empty())
            || tag.chars().any(|c| c.is_control() || c.is_whitespace())
        {
            return Err("literal tags must be canonical namespaced names".to_owned());
        }
        if tags[..index].contains(tag) {
            return Err(format!("duplicate literal tag {tag:?}"));
        }
    }
    Ok(())
}

fn literal_name(name: &str, namespace: &str) -> Result<NamespacedSymbol, String> {
    let name = name.trim();
    if name.is_empty() || name.ends_with('_') {
        return Err(
            "literal names must be nonempty and cannot end in a wildcard underscore".to_owned(),
        );
    }
    if name
        .rsplit("::")
        .next()
        .unwrap_or(name)
        .starts_with(SYMBOL_PREFIX)
    {
        return Err("literal names cannot use the reserved math-display prefix".to_owned());
    }
    if namespace.is_empty()
        || namespace.split("::").any(|part| part.is_empty())
        || namespace
            .chars()
            .any(|c| c.is_control() || c.is_whitespace())
    {
        return Err("literal requires a nonempty canonical namespace".to_owned());
    }
    let qualified = DefaultNamespace {
        namespace: namespace.to_owned().into(),
        data: "",
        file: "".into(),
        line: 0,
    }
    .attach_namespace(name);
    let (namespace, name) = qualified
        .symbol
        .rsplit_once("::")
        .ok_or("invalid literal name")?;
    Token::check_symbol_namespace(namespace)?;
    Token::check_symbol_name(name)?;
    Ok(qualified)
}

fn budget(depth: usize, nodes: &mut usize) -> Result<(), String> {
    *nodes += 1;
    if depth > MAX_DEPTH || *nodes > MAX_NODES {
        return Err(format!(
            "math display exceeds limits of {MAX_DEPTH} levels or {MAX_NODES} nodes"
        ));
    }
    Ok(())
}
fn map_get<'a>(map: &'a [(Value, Value)], key: &str) -> Option<&'a Value> {
    map.iter()
        .find_map(|(k, v)| (k == &Value::Text(key.to_owned())).then_some(v))
}
fn ast_child<'a>(
    slots: &'a [(Value, Value)],
    args: &'a [Value],
    key: &str,
    index: usize,
) -> Result<&'a Value, String> {
    map_get(slots, key)
        .or_else(|| args.get(index))
        .ok_or_else(|| format!("math display missing {key}"))
}
fn parse_ast(value: &Value, depth: usize, nodes: &mut usize) -> Result<MathDisplay, String> {
    budget(depth, nodes)?;
    let recurse = |v: &Value, nodes: &mut usize| parse_ast(v, depth + 1, nodes);
    Ok(match value {
        Value::Null => MathDisplay::Sequence(vec![]),
        Value::Text(text) if text.trim().is_empty() => MathDisplay::Sequence(vec![]),
        Value::Text(text) if valid_number(text.trim()) => {
            MathDisplay::Number(text.trim().to_owned())
        }
        Value::Text(text) => MathDisplay::Symbol(text.trim().to_owned()),
        Value::Integer(number) => MathDisplay::Number(i128::from(*number).to_string()),
        Value::Float(number) if number.is_finite() => MathDisplay::Number(number.to_string()),
        Value::Bytes(bytes) => atom_node(bytes)?,
        Value::Array(items) => MathDisplay::Sequence(
            items
                .iter()
                .map(|v| recurse(v, nodes))
                .collect::<Result<_, _>>()?,
        ),
        Value::Map(map) => {
            let Some(Value::Text(head)) = map_get(map, "head") else {
                return Err("math display node needs a head".to_owned());
            };
            let args = match map_get(map, "args") {
                Some(Value::Array(values)) => values.as_slice(),
                None => &[],
                _ => return Err("math display args must be an array".to_owned()),
            };
            let slots = match map_get(map, "slots") {
                Some(Value::Map(values)) => values.as_slice(),
                None => &[],
                _ => return Err("math display slots must be a map".to_owned()),
            };
            let child = |name, i| ast_child(slots, args, name, i);
            match head.as_str() {
                "attach" => {
                    for (key, value) in slots {
                        if let Value::Text(key) = key {
                            if key != "base"
                                && !ATTACHMENT_SLOTS.contains(&key.as_str())
                                && !matches!(value, Value::Null)
                            {
                                return Err(format!(
                                    "unsupported math attachment parameter {key:?}"
                                ));
                            }
                        }
                    }
                    let mut parsed = std::array::from_fn(|_| None);
                    for (i, key) in ATTACHMENT_SLOTS.iter().enumerate() {
                        if let Some(value) =
                            map_get(slots, key).filter(|v| !matches!(v, Value::Null))
                        {
                            parsed[i] = Some(Box::new(recurse(value, nodes)?));
                        }
                    }
                    MathDisplay::Attach {
                        base: Box::new(recurse(child("base", 0)?, nodes)?),
                        slots: parsed,
                    }
                }
                "primes" => {
                    let value = child("count", 0)?;
                    let count = match value {
                        Value::Integer(n) => u32::try_from(i128::from(*n)).ok(),
                        Value::Text(text) => text.parse().ok(),
                        _ => None,
                    }
                    .ok_or_else(|| "prime count must be a nonnegative integer".to_owned())?;
                    MathDisplay::Primes(count)
                }
                "sequence" | "mul" => MathDisplay::Sequence(
                    args.iter()
                        .map(|v| recurse(v, nodes))
                        .collect::<Result<_, _>>()?,
                ),
                "text" => {
                    let Value::Text(text) = child("text", 0)? else {
                        return Err("display text requires a string".to_owned());
                    };
                    MathDisplay::Text(text.clone())
                }
                "semantic-metadata" => {
                    if let Some(Value::Map(payload)) = map_get(slots, "value") {
                        if map_get(payload, "protocol")
                            == Some(&Value::Text("symbolica".to_owned()))
                            && map_get(payload, "kind") == Some(&Value::Text("atom".to_owned()))
                        {
                            if map_get(payload, "version") != Some(&Value::Integer(1.into())) {
                                return Err("unsupported semantic Atom metadata version".to_owned());
                            }
                            let Some(Value::Bytes(bytes)) = map_get(payload, "atom") else {
                                return Err("semantic Atom metadata requires atom bytes".to_owned());
                            };
                            return atom_node(bytes);
                        }
                    }
                    recurse(
                        args.first()
                            .ok_or_else(|| "metadata missing visible content".to_owned())?,
                        nodes,
                    )?
                }
                "call" | "op-call" => {
                    let fn_key = if head == "call" { "fn" } else { "op" };
                    let body_key = if head == "call" { "body" } else { "args" };
                    let mut arguments = vec![recurse(child(fn_key, 0)?, nodes)?];
                    let body = recurse(child(body_key, 1)?, nodes)?;
                    match body {
                        MathDisplay::Math {
                            head,
                            arguments: items,
                        } if head == "arg" => arguments.extend(items),
                        MathDisplay::Sequence(items) if items.is_empty() => {}
                        body => arguments.push(body),
                    }
                    MathDisplay::Math {
                        head: head.clone(),
                        arguments,
                    }
                }
                "vec" | "cases" => {
                    let values = match map_get(slots, "children") {
                        Some(Value::Array(items)) => items.as_slice(),
                        _ => args,
                    };
                    MathDisplay::Math {
                        head: head.clone(),
                        arguments: values
                            .iter()
                            .map(|v| recurse(v, nodes))
                            .collect::<Result<_, _>>()?,
                    }
                }
                "mat" => MathDisplay::Math {
                    head: head.clone(),
                    arguments: match map_get(slots, "rows") {
                        Some(Value::Array(rows)) => rows.as_slice(),
                        _ => args,
                    }
                    .iter()
                    .map(|v| {
                        let items = match v {
                            Value::Array(items) => items.as_slice(),
                            _ => std::slice::from_ref(v),
                        };
                        Ok(MathDisplay::List(
                            items
                                .iter()
                                .map(|v| recurse(v, nodes))
                                .collect::<Result<_, String>>()?,
                        ))
                    })
                    .collect::<Result<_, String>>()?,
                },
                _ => {
                    let positional: &[&str] = match head.as_str() {
                        "()" | "group" => &["expr"],
                        "lr" => &["body"],
                        "pow" => &["base", "exp"],
                        "frac" => &["num", "denom"],
                        "root" => {
                            if map_get(slots, "index").is_some_and(|v| !matches!(v, Value::Null)) {
                                &["index", "radicand"]
                            } else {
                                &["radicand"]
                            }
                        }
                        "accent" => &["base", "accent"],
                        "op" => &["text"],
                        "class" => &["class", "body"],
                        "underline" | "overline" | "cancel" | "mid" | "scripts" | "limits"
                        | "stretch" | "abs" | "norm" => &["body"],
                        "underbrace" | "overbrace" | "underbracket" | "overbracket"
                        | "underparen" | "overparen" | "undershell" | "overshell" => {
                            if map_get(slots, "annotation")
                                .is_some_and(|v| !matches!(v, Value::Null))
                            {
                                &["body", "annotation"]
                            } else {
                                &["body"]
                            }
                        }
                        _ => &[],
                    };
                    let arguments: Vec<MathDisplay> = if positional.is_empty() {
                        args.iter()
                            .map(|v| recurse(v, nodes))
                            .collect::<Result<_, _>>()?
                    } else {
                        positional
                            .iter()
                            .enumerate()
                            .map(|(i, key)| recurse(child(key, i)?, nodes))
                            .collect::<Result<_, _>>()?
                    };
                    validate_math(head, &arguments)?;
                    MathDisplay::Math {
                        head: head.clone(),
                        arguments,
                    }
                }
            }
        }
        _ => return Err("unsupported value in math display".to_owned()),
    })
}

fn atom_node(bytes: &[u8]) -> Result<MathDisplay, String> {
    if bytes.len() > MAX_BYTES {
        return Err("embedded display Atom is too large".to_owned());
    }
    let payload = parse_payload(bytes).map_err(|e| e.to_string())?;
    Ok(MathDisplay::Atom {
        atom: payload.import_atom().map_err(|e| e.to_string())?,
        attachments: payload.attachment_set(),
    })
}
fn valid_number(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '.' | '+' | '-' | 'e' | 'E'))
        && text.parse::<f64>().is_ok_and(f64::is_finite)
}
fn validate_math(head: &str, arguments: &[MathDisplay]) -> Result<(), String> {
    let count = arguments.len();
    let valid = match head {
        "arg" | "add" | "sub" | "times" | "dot" | "vec" | "mat" | "cases" | "sequence" | "mul"
        | "union" | "inter" => true,
        "plus" | "neg" | "factorial" | "()" | "group" | "lr" | "op" | "underline" | "overline"
        | "cancel" | "mid" | "scripts" | "limits" | "stretch" | "abs" | "norm" => count == 1,
        "pow" | "frac" | "accent" | "class" => count == 2,
        "root" | "underbrace" | "overbrace" | "underbracket" | "overbracket" | "underparen"
        | "overparen" | "undershell" | "overshell" => (1..=2).contains(&count),
        "call" | "op-call" => count >= 1,
        _ => false,
    };
    if !valid {
        return Err(format!(
            "unsupported math display node {head:?} with {count} arguments"
        ));
    }
    let literal =
        |value: &MathDisplay| matches!(value, MathDisplay::Symbol(_) | MathDisplay::Text(_));
    if head == "op" && !literal(&arguments[0])
        || head == "accent" && !literal(&arguments[1])
        || head == "class" && !literal(&arguments[0])
    {
        return Err(format!("{head} requires a literal text parameter"));
    }
    Ok(())
}
fn validate_display(value: &MathDisplay, depth: usize, nodes: &mut usize) -> Result<(), String> {
    budget(depth, nodes)?;
    match value {
        MathDisplay::Symbol(text) | MathDisplay::Text(text) => {
            if text.len() > MAX_TEXT || text.chars().any(char::is_control) {
                return Err(
                    "math display text is too long or contains control characters".to_owned(),
                );
            }
        }
        MathDisplay::Number(text) if !valid_number(text) || text.len() > MAX_TEXT => {
            return Err("invalid display number".to_owned());
        }
        MathDisplay::Primes(count) if *count > 1024 => {
            return Err("prime count exceeds 1024".to_owned());
        }
        MathDisplay::Math { head, arguments } => {
            validate_math(head, arguments)?;
            for item in arguments {
                validate_display(item, depth + 1, nodes)?;
            }
        }
        MathDisplay::Sequence(items) | MathDisplay::List(items) => {
            for item in items {
                validate_display(item, depth + 1, nodes)?;
            }
        }
        MathDisplay::Attach { base, slots } => {
            validate_display(base, depth + 1, nodes)?;
            for item in slots.iter().flatten() {
                validate_display(item, depth + 1, nodes)?;
            }
        }
        MathDisplay::Atom { atom, attachments } => {
            if encode_display_atom(atom, attachments)
                .map_err(|e| e.to_string())?
                .len()
                > MAX_BYTES
            {
                return Err("embedded display Atom is too large".to_owned());
            }
        }
        _ => {}
    }
    Ok(())
}
fn decode_node(value: &Value, depth: usize, nodes: &mut usize) -> Result<MathDisplay, String> {
    budget(depth, nodes)?;
    let Value::Array(fields) = value else {
        return Err("math display node must be an array".to_owned());
    };
    let child = |v: &Value, nodes: &mut usize| decode_node(v, depth + 1, nodes);
    match fields.as_slice() {
        [Value::Text(kind), Value::Text(text)] if kind == "symbol" => {
            Ok(MathDisplay::Symbol(text.clone()))
        }
        [Value::Text(kind), Value::Text(text)] if kind == "text" => {
            Ok(MathDisplay::Text(text.clone()))
        }
        [Value::Text(kind), Value::Text(text)] if kind == "number" => {
            Ok(MathDisplay::Number(text.clone()))
        }
        [Value::Text(kind), Value::Array(items)] if kind == "sequence" || kind == "list" => {
            let items = items
                .iter()
                .map(|v| child(v, nodes))
                .collect::<Result<_, _>>()?;
            Ok(if kind == "sequence" {
                MathDisplay::Sequence(items)
            } else {
                MathDisplay::List(items)
            })
        }
        [Value::Text(kind), Value::Text(head), Value::Array(items)] if kind == "math" => {
            Ok(MathDisplay::Math {
                head: head.clone(),
                arguments: items
                    .iter()
                    .map(|v| child(v, nodes))
                    .collect::<Result<_, _>>()?,
            })
        }
        [Value::Text(kind), Value::Integer(count)] if kind == "primes" => Ok(MathDisplay::Primes(
            u32::try_from(i128::from(*count)).map_err(|_| "invalid prime count")?,
        )),
        [Value::Text(kind), Value::Bytes(bytes)] if kind == "atom" => atom_node(bytes),
        [Value::Text(kind), base, rest @ ..] if kind == "attach" && rest.len() == 6 => {
            let mut slots = std::array::from_fn(|_| None);
            for (i, value) in rest.iter().enumerate() {
                if !matches!(value, Value::Null) {
                    slots[i] = Some(Box::new(child(value, nodes)?));
                }
            }
            Ok(MathDisplay::Attach {
                base: Box::new(child(base, nodes)?),
                slots,
            })
        }
        _ => Err("invalid or unsupported math display node".to_owned()),
    }
}
fn encode_value(value: &Value) -> Result<Vec<u8>, String> {
    let mut bytes = vec![];
    ciborium::into_writer(value, &mut bytes).map_err(|e| e.to_string())?;
    Ok(bytes)
}
fn value_user_data(value: &Value) -> Result<UserData, String> {
    Ok(match value {
        Value::Null => UserData::None,
        Value::Text(text) => UserData::String(text.clone()),
        Value::Integer(number) => UserData::Integer(
            i64::try_from(i128::from(*number)).map_err(|_| "display number out of range")?,
        ),
        Value::Array(items) => UserData::List(
            items
                .iter()
                .map(value_user_data)
                .collect::<Result<_, _>>()?,
        ),
        _ => return Err("invalid display metadata field".to_owned()),
    })
}
fn user_data_value(value: &UserData, depth: usize, nodes: &mut usize) -> Result<Value, String> {
    budget(depth, nodes)?;
    Ok(match value {
        UserData::None => Value::Null,
        UserData::String(text) => Value::Text(text.clone()),
        UserData::Integer(number) => Value::Integer((*number).into()),
        UserData::List(items) => {
            if let [
                UserData::String(kind),
                UserData::Atom(atom),
                UserData::Serialized(sidecars),
            ] = items.as_slice()
            {
                if kind == "atom" {
                    let payload = parse_payload(sidecars).map_err(|e| e.to_string())?;
                    let bytes = encode_display_atom(atom, &payload.attachment_set())
                        .map_err(|e| e.to_string())?;
                    return Ok(Value::Array(vec![
                        Value::Text("atom".to_owned()),
                        Value::Bytes(bytes),
                    ]));
                }
            }
            Value::Array(
                items
                    .iter()
                    .map(|v| user_data_value(v, depth + 1, nodes))
                    .collect::<Result<_, _>>()?,
            )
        }
        _ => return Err("invalid math display symbol data".to_owned()),
    })
}
fn quoted(value: &str) -> String {
    let mut result = String::from("\"");
    for c in value.chars() {
        match c {
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if c.is_control() => result.push('�'),
            c => result.push(c),
        }
    }
    result.push('"');
    result
}
fn math_source(head: &str, args: &[MathDisplay], depth: usize) -> String {
    if validate_math(head, args).is_err() {
        return "upright(\"?\")".to_owned();
    }
    let sources = args
        .iter()
        .map(|item| item.source_at_depth(depth + 1))
        .collect::<Vec<_>>();
    let literal = |i: usize| match &args[i] {
        MathDisplay::Symbol(text) | MathDisplay::Text(text) => quoted(text),
        _ => "\"?\"".to_owned(),
    };
    match head {
        "arg" => sources.join(","),
        "add" | "sub" | "times" | "dot" | "union" | "inter" => sources.join(match head {
            "add" => " + ",
            "sub" => " - ",
            "times" => " times ",
            "dot" => " dot ",
            "union" => " union ",
            _ => " inter ",
        }),
        "mul" | "sequence" => sources.join(" "),
        "plus" => format!("+{}", sources[0]),
        "neg" => format!("-{}", sources[0]),
        "factorial" => format!("{}!", sources[0]),
        "()" | "group" => format!("({})", sources[0]),
        "lr" => format!("lr({})", sources[0]),
        "pow" => format!("attach({},t:{})", sources[0], sources[1]),
        "call" => format!("{}({})", sources[0], sources[1..].join(",")),
        "op-call" => format!("op({})({})", sources[0], sources[1..].join(",")),
        "op" => format!("op({})", literal(0)),
        "accent" => format!("accent({},{})", sources[0], literal(1)),
        "class" => format!("class({},{})", literal(0), sources[1]),
        "root" if sources.len() == 1 => format!("sqrt({})", sources[0]),
        "mat" => format!("mat({})", sources.join(";")),
        // `head` reached this branch only through the fixed allowlist.
        _ => format!("{head}({})", sources.join(",")),
    }
}
fn atom_source(view: AtomView<'_>, depth: usize) -> String {
    if depth > MAX_DEPTH {
        return "upright(\"?\")".to_owned();
    }
    let symbol_source = |symbol: Symbol| {
        if let Ok(Some(display)) = MathDisplay::from_symbol(symbol) {
            return display.symbol_source_at_depth(depth + 1);
        }
        if symbol == Symbol::PI {
            return "pi".to_owned();
        }
        if symbol == Symbol::E {
            return "e".to_owned();
        }
        MathDisplay::Symbol(symbol.get_stripped_name().to_owned()).to_typst_source()
    };
    match view {
        AtomView::Num(_) => view.printer(PrintOptions::typst()).to_string(),
        AtomView::Var(var) => symbol_source(var.get_symbol()),
        AtomView::Fun(fun) => format!(
            "{}({})",
            symbol_source(fun.get_symbol()),
            fun.iter()
                .map(|x| atom_source(x, depth + 1))
                .collect::<Vec<_>>()
                .join(",")
        ),
        AtomView::Pow(pow) => {
            let (base, exp) = pow.get_base_exp();
            format!(
                "attach(lr(({})),t:{})",
                atom_source(base, depth + 1),
                atom_source(exp, depth + 1)
            )
        }
        AtomView::Mul(mul) => mul
            .iter()
            .map(|x| format!("lr(({}))", atom_source(x, depth + 1)))
            .collect::<Vec<_>>()
            .join(" "),
        AtomView::Add(add) => add
            .iter()
            .map(|x| atom_source(x, depth + 1))
            .collect::<Vec<_>>()
            .join(" + "),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn node(head: &str, args: Vec<Value>, slots: Vec<(&str, Value)>) -> Value {
        Value::Map(vec![
            (Value::Text("head".into()), Value::Text(head.into())),
            (Value::Text("args".into()), Value::Array(args)),
            (
                Value::Text("slots".into()),
                Value::Map(
                    slots
                        .into_iter()
                        .map(|(k, v)| (Value::Text(k.into()), v))
                        .collect(),
                ),
            ),
        ])
    }
    fn text(s: &str) -> Value {
        Value::Text(s.into())
    }
    #[test]
    fn ordered_labels_and_all_positions_round_trip() {
        let ij = node("mul", vec![text("i"), text("j")], vec![]);
        let ji = node("mul", vec![text("j"), text("i")], vec![]);
        let first = MathDisplay::from_ast(&node(
            "attach",
            vec![],
            vec![("base", text("C")), ("b", ij)],
        ))
        .unwrap();
        let second = MathDisplay::from_ast(&node(
            "attach",
            vec![],
            vec![("base", text("C")), ("b", ji)],
        ))
        .unwrap();
        assert_ne!(
            first.register("math_display_order_test").unwrap(),
            second.register("math_display_order_test").unwrap()
        );
        let mut slots = vec![("base", text("x"))];
        for name in ATTACHMENT_SLOTS {
            slots.push((name, text(name)));
        }
        let display = MathDisplay::from_ast(&node("attach", vec![], slots)).unwrap();
        assert_eq!(
            MathDisplay::from_value(&display.to_value()).unwrap(),
            display
        );
        let symbol = display.register("math_display_slots_test").unwrap();
        assert_eq!(
            MathDisplay::from_symbol(symbol).unwrap(),
            Some(display.clone())
        );
        for name in ATTACHMENT_SLOTS {
            assert!(display.to_typst_source().contains(&format!(",{name}:")));
        }
        let atom = Atom::var(symbol);
        let bytes = crate::encode_atom(&atom).unwrap();
        assert_eq!(parse_payload(&bytes).unwrap().import_atom().unwrap(), atom);
    }
    #[test]
    fn primes_empty_slots_and_source_are_safe() {
        let prime = node("primes", vec![], vec![("count", Value::Integer(2.into()))]);
        let display = MathDisplay::from_ast(&node(
            "attach",
            vec![],
            vec![("base", text("h")), ("tr", prime), ("b", text(""))],
        ))
        .unwrap();
        assert_eq!(display.to_typst_source(), "attach(h,b:\"\",tr:primes(#2))");
        let malicious = MathDisplay::from_ast(&text("x\" ) #panic(\"oops\")")).unwrap();
        assert_eq!(
            malicious.to_typst_source(),
            "italic(\"x\\\" ) #panic(\\\"oops\\\")\")"
        );
        assert!(MathDisplay::from_ast(&node("eval", vec![text("1")], vec![])).is_err());
    }
    #[test]
    fn embedded_atoms_keep_namespaces_and_native_symbol_data() {
        let x = Symbol::parse("x", "math_display_exact_a".to_owned()).unwrap();
        let y = Symbol::parse("x", "math_display_exact_b".to_owned()).unwrap();
        let a = MathDisplay::from_ast(&Value::Bytes(crate::encode_atom(&Atom::var(x)).unwrap()))
            .unwrap();
        let b = MathDisplay::from_ast(&Value::Bytes(crate::encode_atom(&Atom::var(y)).unwrap()))
            .unwrap();
        assert_ne!(
            a.symbol_name("display").unwrap(),
            b.symbol_name("display").unwrap()
        );
        let symbol = a.register("math_display_exact").unwrap();
        assert_eq!(MathDisplay::from_symbol(symbol).unwrap(), Some(a.clone()));
        assert_eq!(a.to_typst_source(), "x");
        assert_eq!(MathDisplay::from_value(&a.to_value()).unwrap(), a);
    }
    #[test]
    fn literals_share_plain_and_decorated_identities() {
        let namespace = "display_literal_identity_test";
        let x = MathDisplay::Symbol("x".to_owned());
        let symbol = x.register_literal(namespace, None, vec![]).unwrap();
        assert_eq!(symbol, Symbol::parse("x", namespace.to_owned()).unwrap());
        assert_eq!(MathDisplay::from_symbol(symbol).unwrap(), None);
        let indexed = MathDisplay::Attach {
            base: Box::new(x),
            slots: std::array::from_fn(|i| {
                (i == 1).then(|| Box::new(MathDisplay::Number("0".to_owned())))
            }),
        };
        assert_eq!(
            indexed.register(namespace).unwrap(),
            indexed.register_literal(namespace, None, vec![]).unwrap()
        );
        for display in [
            MathDisplay::Number("2".to_owned()),
            MathDisplay::Symbol("x_".to_owned()),
            MathDisplay::Symbol("[".to_owned()),
        ] {
            let symbol = display.register_literal(namespace, None, vec![]).unwrap();
            assert_eq!(symbol.get_wildcard_level(), 0);
            assert!(matches!(Atom::var(symbol).as_view(), AtomView::Var(_)));
            assert_eq!(MathDisplay::from_symbol(symbol).unwrap(), Some(display));
        }
    }

    #[test]
    fn named_literals_retain_display_and_tags_and_reject_conflicts() {
        let display = MathDisplay::Math {
            head: "add".to_owned(),
            arguments: vec![
                MathDisplay::Symbol("x".to_owned()),
                MathDisplay::Symbol("y".to_owned()),
            ],
        };
        let namespace = "display_literal_named_test";
        let tags = vec!["model::quantity".to_owned()];
        let named = display
            .register_literal(namespace, Some("L"), tags.clone())
            .unwrap();
        assert_eq!(named.get_name(), "display_literal_named_test::L");
        assert_eq!(named.get_tags(), tags.as_slice());
        assert_eq!(
            display
                .register_literal(namespace, Some("L"), tags.clone())
                .unwrap(),
            named
        );
        assert_eq!(
            MathDisplay::from_symbol(named).unwrap(),
            Some(display.clone())
        );
        assert!(
            display
                .register_literal(namespace, Some("L"), vec!["model::different".to_owned()])
                .is_err()
        );
        assert!(
            MathDisplay::Text("other".to_owned())
                .register_literal(namespace, Some("L"), tags)
                .is_err()
        );
        assert!(
            display
                .register_literal(namespace, Some("bad_"), vec![])
                .is_err()
        );
        assert!(
            display
                .register_literal(namespace, Some("__math_display_forged"), vec![])
                .is_err()
        );
        assert!(
            display
                .register_literal(namespace, Some("tagged"), vec!["unnamespaced".to_owned()])
                .is_err()
        );
        let bytes = crate::encode_atom(&Atom::var(named)).unwrap();
        let payload = parse_payload(&bytes).unwrap();
        assert_eq!(payload.attachments().len(), 1);
        assert_eq!(
            payload.attachments()[0].identity(),
            named.get_name().as_bytes()
        );
        let imported = payload.import_atom().unwrap().get_symbol().unwrap();
        assert_eq!(MathDisplay::from_symbol(imported).unwrap(), Some(display));
        assert_eq!(imported.get_tags(), named.get_tags());
    }

    #[test]
    fn compound_literals_are_grouped_in_both_rendering_paths() {
        let display = MathDisplay::Math {
            head: "add".to_owned(),
            arguments: vec![
                MathDisplay::Symbol("x".to_owned()),
                MathDisplay::Symbol("y".to_owned()),
            ],
        };
        let symbol = display
            .register_literal("display_literal_grouping_test", None, vec![])
            .unwrap();
        assert_eq!(display.to_typst_source(), "x + y");
        assert_eq!(display.to_symbol_typst_source(), "lr((x + y))");
        let atom = Atom::var(symbol);
        let rendered = crate::typst_symbol_source(symbol).unwrap();
        assert_eq!(rendered, "lr((x + y))");
        let power = atom.pow(2);
        let source = crate::prepare_typst_display(&power)
            .unwrap()
            .printer(PrintOptions::typst())
            .to_string();
        assert!(source.contains("lr((x + y))"), "{source}");
        assert!(source.contains("^2"), "{source}");
    }

    #[test]
    fn hashed_and_named_symbol_identity_cannot_be_reassigned() {
        let display = MathDisplay::Text("label".to_owned());
        let original = display.register("display_literal_tamper_test").unwrap();
        let forged = SymbolBuilder::new(NamespacedSymbol::parse(
            "display_literal_tamper_test::wrong",
        ))
        .with_user_data(original.get_data().clone())
        .build()
        .unwrap();
        assert!(MathDisplay::from_symbol(forged).is_err());
        let named = display
            .register_literal("display_literal_tamper_test", Some("correct"), vec![])
            .unwrap();
        let renamed = SymbolBuilder::new(NamespacedSymbol::parse(
            "display_literal_tamper_test::incorrect",
        ))
        .with_user_data(named.get_data().clone())
        .build()
        .unwrap();
        assert!(MathDisplay::from_symbol(renamed).is_err());
    }

    #[test]
    fn rejects_invalid_metadata_and_depth() {
        let invalid = MathDisplay::Math {
            head: "#panic".into(),
            arguments: vec![],
        };
        assert!(invalid.register("test").is_err());
        let mut value = text("x");
        for _ in 0..70 {
            value = node("attach", vec![], vec![("base", value)]);
        }
        assert!(MathDisplay::from_ast(&value).is_err());
    }
}

// Encoding nested exact atoms must not invoke the public display collector:
// validation of a symbol's own metadata may itself contain decorated atoms.
fn encode_display_atom(
    atom: &Atom,
    attachments: &AttachmentSet,
) -> Result<Vec<u8>, crate::PayloadError> {
    crate::encode_exported_atom_from_set(&crate::export_raw_atom(atom)?, attachments)
}
