//! Symbolica Atom payloads exchanged by Symbolica-compatible plugins.
//!
//! The envelope validates the native Symbolica export's compatibility header,
//! while keeping the rest of that export opaque. Consumers can inspect portable
//! attachments before choosing to call [`Atom::import`] through
//! [`ParsedPayload::import_atom`].

use std::{
    collections::{BTreeMap, btree_map::Entry},
    fmt,
    io::Cursor,
    str,
};

pub mod typst_ast;

use ciborium::value::Value;
use symbolica::{
    atom::{Atom, AtomCore, AtomView, Symbol, SymbolAttribute},
    printer::PrintOptions,
};

/// Magic prefix for the versioned envelope.
pub const PAYLOAD_MAGIC: &[u8; 8] = b"SYMATOM\0";
/// Current binary-envelope version.
pub const PAYLOAD_VERSION: u16 = 2;

/// Protocol discriminator for the generic CBOR Atom render tree.
pub const RENDER_TREE_PROTOCOL: &str = "symbolica";
/// Current schema version of the generic CBOR Atom render tree.
pub const RENDER_TREE_VERSION: u16 = 1;
/// Kind discriminator for the generic CBOR Atom render tree.
pub const RENDER_TREE_KIND: &str = "atom-render-tree";

pub const MAX_ATOM_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_PAYLOAD_BYTES: usize = 10 * 1024 * 1024;
pub const MAX_ATTACHMENTS: usize = 256;
pub const MAX_ATTACHMENT_KEY_BYTES: usize = 1024;
pub const MAX_ATTACHMENT_SCHEMA_BYTES: usize = 128;
pub const MAX_ATTACHMENT_DATA_BYTES: usize = 256 * 1024;
pub const MAX_TOTAL_ATTACHMENT_BYTES: usize = 1024 * 1024;

const SYMBOLICA_MAGIC: u32 = 0x3787_1367;
const SYMBOLICA_EXPORT_FORMAT_VERSION: u16 = 5;
const SYMBOLICA_HEADER_BYTES: usize = size_of::<u32>() + size_of::<u16>();
// Permit redundant records to be merged without letting their wire count grow
// without bound. MAX_ATTACHMENTS applies to unique keys.
const MAX_ENCODED_ATTACHMENT_RECORDS: usize = 1024;
const FIXED_HEADER_BYTES: usize = PAYLOAD_MAGIC.len() + 2 + 2 + 2 + 4;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AttachmentKey {
    schema: String,
    version: u32,
    identity: Vec<u8>,
}

impl AttachmentKey {
    /// Construct a stable attachment key.
    ///
    /// `schema` identifies the attachment format, `version` identifies that
    /// schema's version, and `identity` distinguishes independent values using
    /// the same schema. Identity bytes are opaque and compared byte-for-byte.
    pub fn new(
        schema: impl Into<String>,
        version: u32,
        identity: impl Into<Vec<u8>>,
    ) -> Result<Self, PayloadError> {
        let key = Self {
            schema: schema.into(),
            version,
            identity: identity.into(),
        };
        validate_attachment_key(&key.schema, key.version, &key.identity)?;
        Ok(key)
    }

    pub fn schema(&self) -> &str {
        &self.schema
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn identity(&self) -> &[u8] {
        &self.identity
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attachment {
    key: AttachmentKey,
    data: Vec<u8>,
}

impl Attachment {
    pub fn new(key: AttachmentKey, data: impl Into<Vec<u8>>) -> Result<Self, PayloadError> {
        let data = data.into();
        validate_attachment_data(&data)?;
        Ok(Self { key, data })
    }

    pub fn key(&self) -> &AttachmentKey {
        &self.key
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

/// An owned, deterministically ordered collection of portable attachments.
///
/// Insertion and merge are idempotent for identical `(key, data)` pairs. A
/// repeated key with different data is always an error. [`Self::merge`] checks
/// all conflicts before mutating `self`, so failed merges are transactional.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AttachmentSet {
    entries: BTreeMap<AttachmentKey, Vec<u8>>,
}

impl AttachmentSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_attachments(
        attachments: impl IntoIterator<Item = Attachment>,
    ) -> Result<Self, PayloadError> {
        let mut set = Self::new();
        for attachment in attachments {
            set.insert(attachment)?;
        }
        Ok(set)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Insert an attachment, returning `true` only when a new key was added.
    pub fn insert(&mut self, attachment: Attachment) -> Result<bool, PayloadError> {
        validate_attachment_key(
            &attachment.key.schema,
            attachment.key.version,
            &attachment.key.identity,
        )?;
        validate_attachment_data(&attachment.data)?;
        let current_entry_count = self.entries.len();
        let current_attachment_bytes = self.total_attachment_bytes();
        match self.entries.entry(attachment.key) {
            Entry::Occupied(entry) if entry.get() == &attachment.data => Ok(false),
            Entry::Occupied(entry) => Err(PayloadError::ConflictingAttachment(entry.key().clone())),
            Entry::Vacant(entry) => {
                if current_entry_count >= MAX_ATTACHMENTS {
                    return Err(PayloadError::LimitExceeded);
                }
                let added_bytes = entry
                    .key()
                    .schema
                    .len()
                    .checked_add(entry.key().identity.len())
                    .and_then(|length| length.checked_add(attachment.data.len()))
                    .ok_or(PayloadError::LimitExceeded)?;
                if current_attachment_bytes
                    .checked_add(added_bytes)
                    .is_none_or(|length| length > MAX_TOTAL_ATTACHMENT_BYTES)
                {
                    return Err(PayloadError::LimitExceeded);
                }
                entry.insert(attachment.data);
                Ok(true)
            }
        }
    }

    /// Merge another set without changing either operand on conflict.
    pub fn merge(&mut self, other: &Self) -> Result<(), PayloadError> {
        for (key, data) in &other.entries {
            if let Some(existing) = self.entries.get(key)
                && existing != data
            {
                return Err(PayloadError::ConflictingAttachment(key.clone()));
            }
        }

        let new_entries = other
            .entries
            .iter()
            .filter(|(key, _)| !self.entries.contains_key(*key))
            .count();
        if self
            .entries
            .len()
            .checked_add(new_entries)
            .is_none_or(|length| length > MAX_ATTACHMENTS)
        {
            return Err(PayloadError::LimitExceeded);
        }
        let merged_bytes = self
            .total_attachment_bytes()
            .checked_add(
                other
                    .entries
                    .iter()
                    .filter(|(key, _)| !self.entries.contains_key(*key))
                    .try_fold(0usize, |total, (key, data)| {
                        total
                            .checked_add(key.schema.len())
                            .and_then(|total| total.checked_add(key.identity.len()))
                            .and_then(|total| total.checked_add(data.len()))
                            .ok_or(PayloadError::LimitExceeded)
                    })?,
            )
            .ok_or(PayloadError::LimitExceeded)?;
        if merged_bytes > MAX_TOTAL_ATTACHMENT_BYTES {
            return Err(PayloadError::LimitExceeded);
        }

        for (key, data) in &other.entries {
            self.entries
                .entry(key.clone())
                .or_insert_with(|| data.clone());
        }
        Ok(())
    }

    pub fn get(&self, key: &AttachmentKey) -> Option<&[u8]> {
        self.entries.get(key).map(Vec::as_slice)
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = AttachmentRef<'_>> {
        self.entries.iter().map(|(key, data)| AttachmentRef {
            schema: &key.schema,
            version: key.version,
            identity: &key.identity,
            data,
        })
    }

    /// Wrap already-exported native Atom bytes with this set in tests.
    #[cfg(test)]
    fn encode_exported_atom(&self, atom_bytes: &[u8]) -> Result<Vec<u8>, PayloadError> {
        encode_exported_atom_from_set(atom_bytes, self)
    }

    fn total_attachment_bytes(&self) -> usize {
        self.entries.iter().fold(0usize, |total, (key, data)| {
            total + key.schema.len() + key.identity.len() + data.len()
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttachmentRef<'a> {
    schema: &'a str,
    version: u32,
    identity: &'a [u8],
    data: &'a [u8],
}

impl<'a> AttachmentRef<'a> {
    pub fn schema(&self) -> &'a str {
        self.schema
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn identity(&self) -> &'a [u8] {
        self.identity
    }

    pub fn data(&self) -> &'a [u8] {
        self.data
    }

    pub fn to_owned_attachment(&self) -> Attachment {
        Attachment {
            key: AttachmentKey {
                schema: self.schema.to_owned(),
                version: self.version,
                identity: self.identity.to_vec(),
            },
            data: self.data.to_vec(),
        }
    }
}

/// A validated payload whose compatible native Atom bytes remain unimported.
#[derive(Debug)]
pub struct ParsedPayload<'a> {
    atom_bytes: &'a [u8],
    attachments: Vec<AttachmentRef<'a>>,
}

impl<'a> ParsedPayload<'a> {
    /// Native Symbolica Atom-and-state bytes, still unimported.
    pub fn atom_bytes(&self) -> &'a [u8] {
        self.atom_bytes
    }

    /// Attachments in deterministic key order, with identical duplicates merged.
    pub fn attachments(&self) -> &[AttachmentRef<'a>] {
        &self.attachments
    }

    pub fn attachment(&self, key: &AttachmentKey) -> Option<&'a [u8]> {
        self.attachments
            .iter()
            .find(|attachment| {
                attachment.schema == key.schema
                    && attachment.version == key.version
                    && attachment.identity == key.identity
            })
            .map(|attachment| attachment.data)
    }

    /// Clone the inspected attachment refs into a reusable owned set.
    ///
    /// Parsing has already validated uniqueness and every size invariant, so
    /// this conversion cannot fail.
    pub fn attachment_set(&self) -> AttachmentSet {
        AttachmentSet {
            entries: self
                .attachments
                .iter()
                .map(|attachment| {
                    (
                        AttachmentKey {
                            schema: attachment.schema.to_owned(),
                            version: attachment.version,
                            identity: attachment.identity.to_vec(),
                        },
                        attachment.data.to_vec(),
                    )
                })
                .collect(),
        }
    }

    /// Import the Atom only after envelope inspection has completed.
    ///
    /// # Trust boundary
    ///
    /// The native Atom export remains opaque to this crate. The current
    /// upstream importer trusts lengths embedded inside that export and merges
    /// Symbolica's global state before the complete Atom and trailing bytes are
    /// validated. Consequently, malformed native bytes may allocate excessive
    /// memory or leave partial state changes even when this method returns an
    /// error. Only import native bytes produced by a trusted compatible plugin.
    pub fn import_atom(&self) -> Result<Atom, PayloadError> {
        import_raw_atom(self.atom_bytes)
    }
}

#[derive(Debug)]
pub enum PayloadError {
    LimitExceeded,
    TrailingBytes,
    UnsupportedEnvelopeVersion(u16),
    InvalidEnvelope(&'static str),
    InvalidAttachment(&'static str),
    ConflictingAttachment(AttachmentKey),
    Cbor(String),
    Export(std::io::Error),
    Import(std::io::Error),
}

impl fmt::Display for PayloadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LimitExceeded => {
                formatter.write_str("Atom payload exceeds a size or count limit")
            }
            Self::TrailingBytes => formatter.write_str("Atom payload has trailing bytes"),
            Self::UnsupportedEnvelopeVersion(version) => {
                write!(formatter, "unsupported Atom envelope version {version}")
            }
            Self::InvalidEnvelope(reason) => write!(formatter, "invalid Atom envelope: {reason}"),
            Self::InvalidAttachment(reason) => write!(formatter, "invalid attachment: {reason}"),
            Self::ConflictingAttachment(key) => write!(
                formatter,
                "conflicting data for attachment {} version {} (identity is {} bytes)",
                key.schema,
                key.version,
                key.identity.len()
            ),
            Self::Cbor(error) => write!(formatter, "could not encode Atom render tree: {error}"),
            Self::Export(error) => write!(formatter, "could not export Atom: {error}"),
            Self::Import(error) => write!(formatter, "could not import Atom: {error}"),
        }
    }
}

impl std::error::Error for PayloadError {}

fn validate_attachment_key(
    schema: &str,
    version: u32,
    identity: &[u8],
) -> Result<(), PayloadError> {
    if schema.is_empty() {
        return Err(PayloadError::InvalidAttachment("schema cannot be empty"));
    }
    if schema.len() > MAX_ATTACHMENT_SCHEMA_BYTES {
        return Err(PayloadError::LimitExceeded);
    }
    if !schema.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
    }) {
        return Err(PayloadError::InvalidAttachment(
            "schema must use ASCII letters, digits, '-', '_', '.', ':', or '/'",
        ));
    }
    if version == 0 {
        return Err(PayloadError::InvalidAttachment(
            "schema version must be nonzero",
        ));
    }
    if identity.is_empty() {
        return Err(PayloadError::InvalidAttachment("identity cannot be empty"));
    }
    if schema
        .len()
        .checked_add(identity.len())
        .is_none_or(|length| length > MAX_ATTACHMENT_KEY_BYTES)
    {
        return Err(PayloadError::LimitExceeded);
    }
    Ok(())
}

fn validate_attachment_data(data: &[u8]) -> Result<(), PayloadError> {
    if data.len() > MAX_ATTACHMENT_DATA_BYTES {
        return Err(PayloadError::LimitExceeded);
    }
    Ok(())
}

fn preflight_raw_atom(input: &[u8]) -> Result<(), PayloadError> {
    let header = input
        .get(..SYMBOLICA_HEADER_BYTES)
        .ok_or(PayloadError::InvalidEnvelope(
            "Atom export is shorter than its Symbolica header",
        ))?;
    let magic = u32::from_le_bytes(header[..4].try_into().expect("four-byte slice"));
    if magic != SYMBOLICA_MAGIC {
        return Err(PayloadError::InvalidEnvelope(
            "Atom export has the wrong Symbolica magic",
        ));
    }
    let version = u16::from_le_bytes(header[4..].try_into().expect("two-byte slice"));
    if version != SYMBOLICA_EXPORT_FORMAT_VERSION {
        return Err(PayloadError::InvalidEnvelope(
            "Atom export uses an unsupported Symbolica format version",
        ));
    }
    Ok(())
}

fn export_raw_atom(atom: &Atom) -> Result<Vec<u8>, PayloadError> {
    let mut output = Vec::new();
    atom.as_view()
        .export(&mut output)
        .map_err(PayloadError::Export)?;
    if output.len() > MAX_ATOM_BYTES {
        return Err(PayloadError::LimitExceeded);
    }
    Ok(output)
}

fn import_raw_atom(input: &[u8]) -> Result<Atom, PayloadError> {
    if input.len() > MAX_ATOM_BYTES {
        return Err(PayloadError::LimitExceeded);
    }
    // The envelope bounds the outer byte slice, but upstream Atom::import
    // currently allocates from embedded lengths and mutates global State while
    // reading. See ParsedPayload::import_atom for the public trust contract.
    let mut cursor = Cursor::new(input);
    let atom = Atom::import(&mut cursor, None).map_err(PayloadError::Import)?;
    if cursor.position() != input.len() as u64 {
        return Err(PayloadError::TrailingBytes);
    }
    Ok(atom)
}

fn push_u16(output: &mut Vec<u8>, value: usize) -> Result<(), PayloadError> {
    output.extend_from_slice(
        &u16::try_from(value)
            .map_err(|_| PayloadError::LimitExceeded)?
            .to_be_bytes(),
    );
    Ok(())
}

fn push_u32(output: &mut Vec<u8>, value: usize) -> Result<(), PayloadError> {
    output.extend_from_slice(
        &u32::try_from(value)
            .map_err(|_| PayloadError::LimitExceeded)?
            .to_be_bytes(),
    );
    Ok(())
}

fn encode_exported_atom_from_set(
    atom_bytes: &[u8],
    attachments: &AttachmentSet,
) -> Result<Vec<u8>, PayloadError> {
    if atom_bytes.len() > MAX_ATOM_BYTES {
        return Err(PayloadError::LimitExceeded);
    }
    preflight_raw_atom(atom_bytes)?;

    let attachment_bytes = attachments.total_attachment_bytes();
    if attachment_bytes > MAX_TOTAL_ATTACHMENT_BYTES {
        return Err(PayloadError::LimitExceeded);
    }

    let entry_headers = attachments
        .len()
        .checked_mul(2 + 4 + 4 + 4)
        .ok_or(PayloadError::LimitExceeded)?;
    let total_size = FIXED_HEADER_BYTES
        .checked_add(atom_bytes.len())
        .and_then(|total| total.checked_add(entry_headers))
        .and_then(|total| total.checked_add(attachment_bytes))
        .ok_or(PayloadError::LimitExceeded)?;
    if total_size > MAX_PAYLOAD_BYTES {
        return Err(PayloadError::LimitExceeded);
    }

    let mut output = Vec::with_capacity(total_size);
    output.extend_from_slice(PAYLOAD_MAGIC);
    output.extend_from_slice(&PAYLOAD_VERSION.to_be_bytes());
    output.extend_from_slice(&0u16.to_be_bytes()); // reserved flags
    push_u16(&mut output, attachments.len())?;
    push_u32(&mut output, atom_bytes.len())?;
    output.extend_from_slice(atom_bytes);

    for (key, data) in &attachments.entries {
        push_u16(&mut output, key.schema.len())?;
        output.extend_from_slice(&key.version.to_be_bytes());
        push_u32(&mut output, key.identity.len())?;
        push_u32(&mut output, data.len())?;
        output.extend_from_slice(key.schema.as_bytes());
        output.extend_from_slice(&key.identity);
        output.extend_from_slice(data);
    }
    debug_assert_eq!(output.len(), total_size);
    Ok(output)
}

/// Wrap already-exported native Symbolica Atom bytes in the current envelope.
///
/// This function does not import or otherwise interpret `atom_bytes`.
fn encode_exported_atom(
    atom_bytes: &[u8],
    attachments: impl IntoIterator<Item = Attachment>,
) -> Result<Vec<u8>, PayloadError> {
    let attachments = AttachmentSet::from_attachments(attachments)?;
    encode_exported_atom_from_set(atom_bytes, &attachments)
}

/// Export an Atom and attach portable, schema-keyed data.
pub fn encode_atom_with_attachments(
    atom: &Atom,
    attachments: impl IntoIterator<Item = Attachment>,
) -> Result<Vec<u8>, PayloadError> {
    encode_exported_atom(&export_raw_atom(atom)?, attachments)
}

/// Export an Atom and attach a reusable owned set.
pub fn encode_atom_from_set(
    atom: &Atom,
    attachments: &AttachmentSet,
) -> Result<Vec<u8>, PayloadError> {
    encode_exported_atom_from_set(&export_raw_atom(atom)?, attachments)
}

/// Export one Atom in a versioned envelope with no attachments.
pub fn encode_atom(atom: &Atom) -> Result<Vec<u8>, PayloadError> {
    encode_atom_with_attachments(atom, std::iter::empty())
}

fn render_tree_map(entries: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(
        entries
            .into_iter()
            .map(|(key, value)| (Value::Text(key.to_owned()), value))
            .collect(),
    )
}

fn symbol_attribute_name(attribute: SymbolAttribute) -> &'static str {
    match attribute {
        SymbolAttribute::Symmetric => "symmetric",
        SymbolAttribute::Antisymmetric => "antisymmetric",
        SymbolAttribute::Cyclesymmetric => "cyclesymmetric",
        SymbolAttribute::Linear => "linear",
        SymbolAttribute::Flat => "flat",
        SymbolAttribute::Scalar => "scalar",
        SymbolAttribute::Real => "real",
        SymbolAttribute::Integer => "integer",
        SymbolAttribute::Positive => "positive",
    }
}

/// Render a symbol using Symbolica's built-in Typst symbol rules.
///
/// This deliberately does not call the symbol's custom Rust print callback.
/// The implementation mirrors `Symbol::format` with
/// [`PrintOptions::typst`] (which hides namespaces); the complete namespaced
/// identity remains available separately in the symbol descriptor.
fn builtin_typst_symbol_source(symbol: Symbol) -> String {
    if symbol == Symbol::E {
        "e".to_owned()
    } else if symbol == Symbol::PI {
        "pi".to_owned()
    } else if symbol == Symbol::COS {
        "cos".to_owned()
    } else if symbol == Symbol::SIN {
        "sin".to_owned()
    } else if symbol == Symbol::EXP {
        "exp".to_owned()
    } else if symbol == Symbol::LOG {
        "log".to_owned()
    } else {
        let name = symbol.get_stripped_name();
        if name.chars().count() == 1 {
            name.to_owned()
        } else {
            format!("\"{name}\"")
        }
    }
}

fn symbol_render_tree_value(symbol: Symbol) -> Value {
    render_tree_map([
        ("name", Value::Text(symbol.get_name().to_owned())),
        ("namespace", Value::Text(symbol.get_namespace().to_owned())),
        (
            "short-name",
            Value::Text(symbol.get_stripped_name().to_owned()),
        ),
        (
            "tags",
            Value::Array(
                symbol
                    .get_tags()
                    .iter()
                    .map(|tag| Value::Text(tag.clone()))
                    .collect(),
            ),
        ),
        (
            "attributes",
            Value::Array(
                symbol
                    .get_attributes()
                    .into_iter()
                    .map(|attribute| Value::Text(symbol_attribute_name(attribute).to_owned()))
                    .collect(),
            ),
        ),
    ])
}

fn atom_render_node_value(
    view: AtomView<'_>,
    attachments: &AttachmentSet,
) -> Result<Value, PayloadError> {
    match view {
        AtomView::Num(_) => {
            // Numeric nodes intentionally carry no exact Atom payload. They
            // stay editable as ordinary textual Typst mathematics.
            let source = view.printer(PrintOptions::typst()).to_string();
            Ok(render_tree_map([
                ("kind", Value::Text("number".to_owned())),
                ("text", Value::Text(source.clone())),
                ("source", Value::Text(source)),
            ]))
        }
        AtomView::Var(variable) => {
            let atom = view.to_owned();
            let symbol = variable.get_symbol();
            Ok(render_tree_map([
                ("kind", Value::Text("variable".to_owned())),
                ("symbol", symbol_render_tree_value(symbol)),
                ("source", Value::Text(builtin_typst_symbol_source(symbol))),
                (
                    "atom",
                    Value::Bytes(encode_atom_from_set(&atom, attachments)?),
                ),
            ]))
        }
        AtomView::Fun(function) => {
            let atom = view.to_owned();
            let symbol = function.get_symbol();
            let head = Atom::var(symbol);
            let arguments = function
                .iter()
                .map(|argument| atom_render_node_value(argument, attachments))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(render_tree_map([
                ("kind", Value::Text("function".to_owned())),
                ("symbol", symbol_render_tree_value(symbol)),
                ("source", Value::Text(builtin_typst_symbol_source(symbol))),
                (
                    "head-atom",
                    Value::Bytes(encode_atom_from_set(&head, attachments)?),
                ),
                (
                    "atom",
                    Value::Bytes(encode_atom_from_set(&atom, attachments)?),
                ),
                ("arguments", Value::Array(arguments)),
            ]))
        }
        AtomView::Pow(power) => {
            let (base, exponent) = power.get_base_exp();
            Ok(render_tree_map([
                ("kind", Value::Text("power".to_owned())),
                ("base", atom_render_node_value(base, attachments)?),
                ("exponent", atom_render_node_value(exponent, attachments)?),
            ]))
        }
        AtomView::Mul(product) => Ok(render_tree_map([
            ("kind", Value::Text("product".to_owned())),
            (
                "factors",
                Value::Array(
                    product
                        .iter()
                        .map(|factor| atom_render_node_value(factor, attachments))
                        .collect::<Result<Vec<_>, _>>()?,
                ),
            ),
        ])),
        AtomView::Add(sum) => Ok(render_tree_map([
            ("kind", Value::Text("sum".to_owned())),
            (
                "terms",
                Value::Array(
                    sum.iter()
                        .map(|term| atom_render_node_value(term, attachments))
                        .collect::<Result<Vec<_>, _>>()?,
                ),
            ),
        ])),
    }
}

/// Build the versioned, generic CBOR render tree for an Atom.
///
/// Algebraic structure and Symbolica symbol metadata are exposed without any
/// package-specific interpretation. Variable nodes carry an exact portable
/// Atom payload; function nodes carry exact payloads for both the complete call
/// and its variable head. Every exact payload receives the complete supplied
/// attachment set, including attachment schemas unknown to this crate.
pub fn atom_render_tree_value(
    atom: &Atom,
    attachments: &AttachmentSet,
) -> Result<Value, PayloadError> {
    let attachment_values = attachments
        .iter()
        .map(|attachment| {
            render_tree_map([
                ("schema", Value::Text(attachment.schema().to_owned())),
                (
                    "version",
                    Value::Integer(i64::from(attachment.version()).into()),
                ),
                ("identity", Value::Bytes(attachment.identity().to_vec())),
                ("data", Value::Bytes(attachment.data().to_vec())),
            ])
        })
        .collect();

    Ok(render_tree_map([
        ("protocol", Value::Text(RENDER_TREE_PROTOCOL.to_owned())),
        (
            "version",
            Value::Integer(i64::from(RENDER_TREE_VERSION).into()),
        ),
        ("kind", Value::Text(RENDER_TREE_KIND.to_owned())),
        ("root", atom_render_node_value(atom.as_view(), attachments)?),
        ("attachments", Value::Array(attachment_values)),
    ]))
}

/// Encode the generic Atom render tree as CBOR bytes.
pub fn encode_atom_render_tree(
    atom: &Atom,
    attachments: &AttachmentSet,
) -> Result<Vec<u8>, PayloadError> {
    let value = atom_render_tree_value(atom, attachments)?;
    let mut output = Vec::new();
    ciborium::into_writer(&value, &mut output)
        .map_err(|error| PayloadError::Cbor(error.to_string()))?;
    Ok(output)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct BorrowedAttachmentKey<'a> {
    schema: &'a str,
    version: u32,
    identity: &'a [u8],
}

struct Reader<'a> {
    input: &'a [u8],
    position: usize,
}

impl<'a> Reader<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self { input, position: 0 }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], PayloadError> {
        let end = self
            .position
            .checked_add(length)
            .ok_or(PayloadError::LimitExceeded)?;
        let value = self
            .input
            .get(self.position..end)
            .ok_or(PayloadError::InvalidEnvelope("unexpected end of input"))?;
        self.position = end;
        Ok(value)
    }

    fn u16(&mut self) -> Result<u16, PayloadError> {
        Ok(u16::from_be_bytes(
            self.take(2)?.try_into().expect("two-byte slice"),
        ))
    }

    fn u32(&mut self) -> Result<u32, PayloadError> {
        Ok(u32::from_be_bytes(
            self.take(4)?.try_into().expect("four-byte slice"),
        ))
    }

    fn is_finished(&self) -> bool {
        self.position == self.input.len()
    }
}

fn parse_envelope(input: &[u8]) -> Result<ParsedPayload<'_>, PayloadError> {
    let mut reader = Reader::new(input);
    if reader.take(PAYLOAD_MAGIC.len())? != PAYLOAD_MAGIC {
        return Err(PayloadError::InvalidEnvelope("wrong magic"));
    }
    let version = reader.u16()?;
    if version != PAYLOAD_VERSION {
        return Err(PayloadError::UnsupportedEnvelopeVersion(version));
    }
    if reader.u16()? != 0 {
        return Err(PayloadError::InvalidEnvelope("reserved flags must be zero"));
    }
    let entry_count = usize::from(reader.u16()?);
    let atom_length = usize::try_from(reader.u32()?).map_err(|_| PayloadError::LimitExceeded)?;
    if entry_count > MAX_ENCODED_ATTACHMENT_RECORDS || atom_length > MAX_ATOM_BYTES {
        return Err(PayloadError::LimitExceeded);
    }
    let atom_bytes = reader.take(atom_length)?;
    preflight_raw_atom(atom_bytes)?;

    let mut total_attachment_bytes = 0usize;
    let mut merged = BTreeMap::<BorrowedAttachmentKey<'_>, &'_ [u8]>::new();
    for _ in 0..entry_count {
        let schema_length = usize::from(reader.u16()?);
        let version = reader.u32()?;
        let identity_length =
            usize::try_from(reader.u32()?).map_err(|_| PayloadError::LimitExceeded)?;
        let data_length =
            usize::try_from(reader.u32()?).map_err(|_| PayloadError::LimitExceeded)?;
        if schema_length > MAX_ATTACHMENT_SCHEMA_BYTES
            || schema_length
                .checked_add(identity_length)
                .is_none_or(|length| length > MAX_ATTACHMENT_KEY_BYTES)
            || data_length > MAX_ATTACHMENT_DATA_BYTES
        {
            return Err(PayloadError::LimitExceeded);
        }

        let schema = str::from_utf8(reader.take(schema_length)?)
            .map_err(|_| PayloadError::InvalidAttachment("schema is not UTF-8"))?;
        let identity = reader.take(identity_length)?;
        let data = reader.take(data_length)?;
        validate_attachment_key(schema, version, identity)?;
        validate_attachment_data(data)?;
        total_attachment_bytes = total_attachment_bytes
            .checked_add(schema_length)
            .and_then(|total| total.checked_add(identity_length))
            .and_then(|total| total.checked_add(data_length))
            .ok_or(PayloadError::LimitExceeded)?;
        if total_attachment_bytes > MAX_TOTAL_ATTACHMENT_BYTES {
            return Err(PayloadError::LimitExceeded);
        }

        let key = BorrowedAttachmentKey {
            schema,
            version,
            identity,
        };
        let unique_entry_count = merged.len();
        match merged.entry(key) {
            Entry::Vacant(entry) => {
                if unique_entry_count >= MAX_ATTACHMENTS {
                    return Err(PayloadError::LimitExceeded);
                }
                entry.insert(data);
            }
            Entry::Occupied(entry) if entry.get() == &data => {}
            Entry::Occupied(entry) => {
                return Err(PayloadError::ConflictingAttachment(AttachmentKey {
                    schema: entry.key().schema.to_owned(),
                    version: entry.key().version,
                    identity: entry.key().identity.to_vec(),
                }));
            }
        }
    }
    if !reader.is_finished() {
        return Err(PayloadError::TrailingBytes);
    }

    Ok(ParsedPayload {
        atom_bytes,
        attachments: merged
            .into_iter()
            .map(|(key, data)| AttachmentRef {
                schema: key.schema,
                version: key.version,
                identity: key.identity,
                data,
            })
            .collect(),
    })
}

/// Validate a current envelope and expose its metadata without importing the Atom.
pub fn parse_payload(input: &[u8]) -> Result<ParsedPayload<'_>, PayloadError> {
    if input.len() > MAX_PAYLOAD_BYTES {
        return Err(PayloadError::LimitExceeded);
    }
    parse_envelope(input)
}

/// Import an Atom from a current envelope.
pub fn decode_atom(input: &[u8]) -> Result<Atom, PayloadError> {
    parse_payload(input)?.import_atom()
}

#[cfg(test)]
mod tests {
    use super::*;
    use symbolica::prelude::{Coefficient, Complex, Float};
    use symbolica::{function, parse, symbol};

    const OPAQUE_ATOM_EXPORT: &[u8] = &[
        0x67, 0x13, 0x87, 0x37, // Symbolica magic, little endian
        0x05, 0x00, // Symbolica export format, little endian
        b'o', b'p', b'a', b'q', b'u', b'e',
    ];

    fn key(schema: &str, version: u32, identity: &[u8]) -> AttachmentKey {
        AttachmentKey::new(schema, version, identity.to_vec()).unwrap()
    }

    fn attachment(schema: &str, version: u32, identity: &[u8], data: &[u8]) -> Attachment {
        Attachment::new(key(schema, version, identity), data.to_vec()).unwrap()
    }

    fn duplicate_last_entry(mut payload: Vec<u8>, conflicting: bool) -> Vec<u8> {
        let atom_length =
            usize::try_from(u32::from_be_bytes(payload[14..18].try_into().unwrap())).unwrap();
        let entry_start = FIXED_HEADER_BYTES + atom_length;
        let mut duplicate = payload[entry_start..].to_vec();
        if conflicting {
            *duplicate.last_mut().unwrap() ^= 1;
        }
        payload[12..14].copy_from_slice(&2u16.to_be_bytes());
        payload.extend_from_slice(&duplicate);
        payload
    }

    fn repeat_last_entry(mut payload: Vec<u8>, count: u16) -> Vec<u8> {
        assert!(count >= 1);
        let atom_length =
            usize::try_from(u32::from_be_bytes(payload[14..18].try_into().unwrap())).unwrap();
        let entry_start = FIXED_HEADER_BYTES + atom_length;
        let entry = payload[entry_start..].to_vec();
        payload[12..14].copy_from_slice(&count.to_be_bytes());
        for _ in 1..count {
            payload.extend_from_slice(&entry);
        }
        payload
    }

    fn value_field<'a>(map: &'a [(Value, Value)], key: &str) -> Option<&'a Value> {
        map.iter().find_map(|(candidate, value)| {
            matches!(candidate, Value::Text(candidate) if candidate == key).then_some(value)
        })
    }

    fn find_node<'a>(value: &'a Value, kind: &str) -> Option<&'a [(Value, Value)]> {
        match value {
            Value::Map(map) => {
                if value_field(map, "kind") == Some(&Value::Text(kind.to_owned())) {
                    return Some(map);
                }
                map.iter().find_map(|(_, child)| find_node(child, kind))
            }
            Value::Array(values) => values.iter().find_map(|child| find_node(child, kind)),
            _ => None,
        }
    }

    fn collect_node_kinds(value: &Value, kinds: &mut std::collections::BTreeSet<String>) {
        match value {
            Value::Map(map) => {
                if let Some(Value::Text(kind)) = value_field(map, "kind") {
                    kinds.insert(kind.clone());
                }
                for (_, child) in map {
                    collect_node_kinds(child, kinds);
                }
            }
            Value::Array(values) => {
                for child in values {
                    collect_node_kinds(child, kinds);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn envelope_supports_two_stage_inspection_without_import() {
        let attachment_key = key("org.symbolica.test", 1, b"namespace::f");
        let payload = encode_exported_atom(
            OPAQUE_ATOM_EXPORT,
            [Attachment::new(attachment_key.clone(), b"metadata".to_vec()).unwrap()],
        )
        .unwrap();

        let parsed = parse_payload(&payload).unwrap();
        assert_eq!(parsed.attachments().len(), 1);
        assert_eq!(
            parsed.attachment(&attachment_key),
            Some(b"metadata".as_slice())
        );
        assert_eq!(parsed.atom_bytes(), OPAQUE_ATOM_EXPORT);
    }

    #[test]
    fn encoding_is_deterministic_and_merges_identical_entries() {
        let a = attachment("org.symbolica.a", 1, b"a", b"first");
        let b = attachment("org.symbolica.b", 2, b"b", b"second");
        let forward = encode_exported_atom(OPAQUE_ATOM_EXPORT, [a.clone(), b.clone()]).unwrap();
        let reversed_with_duplicate =
            encode_exported_atom(OPAQUE_ATOM_EXPORT, [b, a.clone(), a]).unwrap();

        assert_eq!(forward, reversed_with_duplicate);
        let parsed = parse_payload(&forward).unwrap();
        assert_eq!(parsed.attachments().len(), 2);
        assert_eq!(parsed.attachments()[0].schema(), "org.symbolica.a");
        assert_eq!(parsed.attachments()[1].schema(), "org.symbolica.b");
    }

    #[test]
    fn attachment_sets_merge_associatively_and_reencode_raw_exports() {
        let a = AttachmentSet::from_attachments([attachment("org.symbolica.a", 1, b"a", b"first")])
            .unwrap();
        let b = AttachmentSet::from_attachments([
            attachment("org.symbolica.a", 1, b"a", b"first"),
            attachment("org.symbolica.b", 1, b"b", b"second"),
        ])
        .unwrap();
        let c = AttachmentSet::from_attachments([attachment("org.symbolica.c", 2, b"c", b"third")])
            .unwrap();

        let mut left = a.clone();
        left.merge(&b).unwrap();
        left.merge(&c).unwrap();
        let mut right_tail = b.clone();
        right_tail.merge(&c).unwrap();
        let mut right = a;
        right.merge(&right_tail).unwrap();
        assert_eq!(left, right);
        assert_eq!(left.len(), 3);
        assert_eq!(
            left.get(&key("org.symbolica.b", 1, b"b")),
            Some(b"second".as_slice())
        );
        assert_eq!(left.iter().count(), 3);

        let encoded = left.encode_exported_atom(OPAQUE_ATOM_EXPORT).unwrap();
        let parsed = parse_payload(&encoded).unwrap();
        assert_eq!(parsed.atom_bytes(), OPAQUE_ATOM_EXPORT);
        assert_eq!(parsed.attachment_set(), left);
    }

    #[test]
    fn attachment_limit_counts_unique_keys_after_deduplication() {
        let repeated = attachment("org.symbolica.same", 1, b"same", b"same-data");
        let set = AttachmentSet::from_attachments(std::iter::repeat_n(
            repeated.clone(),
            MAX_ATTACHMENTS + 1,
        ))
        .unwrap();
        assert_eq!(set.len(), 1);

        let encoded = encode_exported_atom(OPAQUE_ATOM_EXPORT, [repeated]).unwrap();
        let repeated_on_wire = repeat_last_entry(encoded, (MAX_ATTACHMENTS + 1) as u16);
        assert_eq!(
            parse_payload(&repeated_on_wire)
                .unwrap()
                .attachments()
                .len(),
            1
        );

        let unique = (0..=MAX_ATTACHMENTS)
            .map(|index| attachment("org.symbolica.unique", 1, &index.to_be_bytes(), b"data"))
            .collect::<Vec<_>>();
        assert!(matches!(
            AttachmentSet::from_attachments(unique),
            Err(PayloadError::LimitExceeded)
        ));
    }

    #[test]
    fn attachment_set_conflicts_are_transactional() {
        let mut target = AttachmentSet::from_attachments([
            attachment("org.symbolica.a", 1, b"a", b"original"),
            attachment("org.symbolica.b", 1, b"b", b"stable"),
        ])
        .unwrap();
        let before = target.clone();
        let conflicting = AttachmentSet::from_attachments([
            attachment("org.symbolica.c", 1, b"c", b"new"),
            attachment("org.symbolica.a", 1, b"a", b"different"),
        ])
        .unwrap();

        assert!(matches!(
            target.merge(&conflicting),
            Err(PayloadError::ConflictingAttachment(_))
        ));
        assert_eq!(target, before);
    }

    #[test]
    fn attachment_set_limit_failures_are_transactional() {
        let mut count_limited = AttachmentSet::from_attachments(
            (0..MAX_ATTACHMENTS)
                .map(|index| attachment("org.symbolica.count", 1, &index.to_be_bytes(), b"data")),
        )
        .unwrap();
        let count_before = count_limited.clone();
        let count_overflow = AttachmentSet::from_attachments([attachment(
            "org.symbolica.count",
            1,
            b"overflow",
            b"data",
        )])
        .unwrap();
        assert!(matches!(
            count_limited.merge(&count_overflow),
            Err(PayloadError::LimitExceeded)
        ));
        assert_eq!(count_limited, count_before);

        let large_data = vec![0; MAX_ATTACHMENT_DATA_BYTES];
        let mut byte_limited =
            AttachmentSet::from_attachments((0_u32..3).map(|index| {
                attachment("org.symbolica.bytes", 1, &index.to_be_bytes(), &large_data)
            }))
            .unwrap();
        let byte_before = byte_limited.clone();
        let byte_overflow = AttachmentSet::from_attachments([attachment(
            "org.symbolica.bytes",
            1,
            &3_u32.to_be_bytes(),
            &large_data,
        )])
        .unwrap();
        assert!(matches!(
            byte_limited.merge(&byte_overflow),
            Err(PayloadError::LimitExceeded)
        ));
        assert_eq!(byte_limited, byte_before);
    }

    #[test]
    fn conflicting_entries_are_rejected_during_encode_and_parse() {
        let first = attachment("org.symbolica.test", 1, b"same", b"one");
        let second = attachment("org.symbolica.test", 1, b"same", b"two");
        assert!(matches!(
            encode_exported_atom(OPAQUE_ATOM_EXPORT, [first.clone(), second]),
            Err(PayloadError::ConflictingAttachment(_))
        ));

        let encoded = encode_exported_atom(OPAQUE_ATOM_EXPORT, [first]).unwrap();
        assert_eq!(
            parse_payload(&duplicate_last_entry(encoded.clone(), false))
                .unwrap()
                .attachments()
                .len(),
            1
        );
        assert!(matches!(
            parse_payload(&duplicate_last_entry(encoded, true)),
            Err(PayloadError::ConflictingAttachment(_))
        ));
    }

    #[test]
    fn incompatible_symbolica_headers_are_rejected_during_parse() {
        let payload = encode_exported_atom(
            OPAQUE_ATOM_EXPORT,
            [attachment("org.symbolica.test", 1, b"id", b"data")],
        )
        .unwrap();

        let mut wrong_magic = payload.clone();
        wrong_magic[FIXED_HEADER_BYTES] ^= 1;
        let entry_start = FIXED_HEADER_BYTES + OPAQUE_ATOM_EXPORT.len();
        wrong_magic[entry_start..entry_start + 2].copy_from_slice(
            &u16::try_from(MAX_ATTACHMENT_SCHEMA_BYTES + 1)
                .unwrap()
                .to_be_bytes(),
        );
        assert!(matches!(
            parse_payload(&wrong_magic),
            Err(PayloadError::InvalidEnvelope(
                "Atom export has the wrong Symbolica magic"
            ))
        ));

        let mut wrong_format = payload;
        wrong_format[FIXED_HEADER_BYTES + 4..FIXED_HEADER_BYTES + 6]
            .copy_from_slice(&4_u16.to_le_bytes());
        assert!(matches!(
            parse_payload(&wrong_format),
            Err(PayloadError::InvalidEnvelope(
                "Atom export uses an unsupported Symbolica format version"
            ))
        ));
    }

    #[test]
    fn truncated_envelopes_are_always_rejected() {
        let payload = encode_exported_atom(
            OPAQUE_ATOM_EXPORT,
            [attachment("org.symbolica.test", 1, b"id", b"data")],
        )
        .unwrap();

        for length in 0..payload.len() {
            assert!(
                parse_payload(&payload[..length]).is_err(),
                "truncation at byte {length} was accepted"
            );
        }
    }

    #[test]
    fn malformed_envelopes_are_rejected() {
        let payload = encode_exported_atom(OPAQUE_ATOM_EXPORT, []).unwrap();

        let mut unsupported_version = payload.clone();
        unsupported_version[8..10].copy_from_slice(&(PAYLOAD_VERSION + 1).to_be_bytes());
        assert!(matches!(
            parse_payload(&unsupported_version),
            Err(PayloadError::UnsupportedEnvelopeVersion(_))
        ));

        let mut reserved_flags = payload.clone();
        reserved_flags[10..12].copy_from_slice(&1_u16.to_be_bytes());
        assert!(matches!(
            parse_payload(&reserved_flags),
            Err(PayloadError::InvalidEnvelope(_))
        ));

        let mut empty_atom = payload;
        empty_atom[14..18].copy_from_slice(&0_u32.to_be_bytes());
        assert!(matches!(
            parse_payload(&empty_atom),
            Err(PayloadError::InvalidEnvelope(_))
        ));

        assert!(matches!(
            parse_payload(b"not an Atom payload"),
            Err(PayloadError::InvalidEnvelope(_))
        ));
        let attachments = AttachmentSet::new();
        assert!(matches!(
            encode_exported_atom_from_set(&OPAQUE_ATOM_EXPORT[..5], &attachments),
            Err(PayloadError::InvalidEnvelope(_))
        ));
        let mut wrong_magic = OPAQUE_ATOM_EXPORT.to_vec();
        wrong_magic[0] ^= 1;
        assert!(matches!(
            encode_exported_atom_from_set(&wrong_magic, &attachments),
            Err(PayloadError::InvalidEnvelope(_))
        ));
    }

    #[test]
    fn limits_and_trailing_bytes_are_enforced() {
        assert!(matches!(
            AttachmentKey::new("org.symbolica.test", 1, vec![0; MAX_ATTACHMENT_KEY_BYTES]),
            Err(PayloadError::LimitExceeded)
        ));
        assert!(matches!(
            Attachment::new(
                key("org.symbolica.test", 1, b"id"),
                vec![0; MAX_ATTACHMENT_DATA_BYTES + 1]
            ),
            Err(PayloadError::LimitExceeded)
        ));

        let mut payload = encode_exported_atom(OPAQUE_ATOM_EXPORT, []).unwrap();
        payload.push(0);
        assert!(matches!(
            parse_payload(&payload),
            Err(PayloadError::TrailingBytes)
        ));

        let mut too_many_records = encode_exported_atom(OPAQUE_ATOM_EXPORT, []).unwrap();
        too_many_records[12..14].copy_from_slice(
            &u16::try_from(MAX_ENCODED_ATTACHMENT_RECORDS + 1)
                .unwrap()
                .to_be_bytes(),
        );
        assert!(matches!(
            parse_payload(&too_many_records),
            Err(PayloadError::LimitExceeded)
        ));
    }

    #[test]
    fn native_atoms_round_trip() {
        // A restricted Symbolica build permits one instance per process. Keep
        // all native Atom operations in one test thread; the other tests only
        // inspect the deliberately opaque exported bytes.
        let atom = parse!("f(x)^2+1/3");
        let payload = encode_atom_with_attachments(
            &atom,
            [attachment(
                "org.symbolica.test",
                1,
                b"namespace::f",
                b"metadata",
            )],
        )
        .unwrap();
        assert_eq!(
            parse_payload(&payload).unwrap().import_atom().unwrap(),
            atom
        );
        assert_eq!(decode_atom(&payload).unwrap(), atom);

        let mut trailing_atom_payload = encode_atom(&atom).unwrap();
        let atom_length = u32::from_be_bytes(trailing_atom_payload[14..18].try_into().unwrap());
        trailing_atom_payload[14..18].copy_from_slice(&(atom_length + 1).to_be_bytes());
        trailing_atom_payload.push(0);
        let parsed_trailing = parse_payload(&trailing_atom_payload).unwrap();
        assert!(matches!(
            parsed_trailing.import_atom(),
            Err(PayloadError::TrailingBytes)
        ));

        let rich = symbol!(
            "symbolica_payload_test::g";
            Symmetric, Linear, Real;
            tags = ["symbolica_test::render"]
        );
        let rich_atom = function!(
            rich,
            symbol!("symbolica_payload_test::y"),
            symbol!("symbolica_payload_test::x")
        );
        assert_eq!(
            decode_atom(&encode_atom(&rich_atom).unwrap()).unwrap(),
            rich_atom
        );

        let float_atom = Atom::num(Coefficient::Float(Complex::new(
            Float::with_val(128, 1.44496_f64),
            Float::with_val(128, 0),
        )));
        assert_eq!(
            decode_atom(&encode_atom(&float_atom).unwrap()).unwrap(),
            float_atom
        );
        let float_tree = atom_render_tree_value(&float_atom, &AttachmentSet::new()).unwrap();
        let float_number = find_node(&float_tree, "number").unwrap();
        assert_eq!(
            value_field(float_number, "source"),
            value_field(float_number, "text")
        );
        assert!(matches!(
            value_field(float_number, "source"),
            Some(Value::Text(source)) if !source.is_empty()
        ));

        let render_attachments = AttachmentSet::from_attachments([
            attachment(
                "org.symbolica.known",
                1,
                b"symbolica_payload_test::g",
                b"known declaration",
            ),
            attachment(
                "example.unknown-schema",
                19,
                b"opaque identity",
                &[0xa2, 0x01, 0x02, 0x03],
            ),
        ])
        .unwrap();
        let x = symbol!("symbolica_payload_test::render_x");
        let structured = rich_atom.clone() * Atom::var(x).pow(2) + Atom::num((1, 3));
        let render_tree = atom_render_tree_value(&structured, &render_attachments).unwrap();

        let Value::Map(envelope) = &render_tree else {
            panic!("render tree envelope must be a map");
        };
        assert_eq!(
            value_field(envelope, "protocol"),
            Some(&Value::Text(RENDER_TREE_PROTOCOL.to_owned()))
        );
        assert_eq!(
            value_field(envelope, "version"),
            Some(&Value::Integer(i64::from(RENDER_TREE_VERSION).into()))
        );
        assert_eq!(
            value_field(envelope, "kind"),
            Some(&Value::Text(RENDER_TREE_KIND.to_owned()))
        );
        let Value::Array(exposed_attachments) = value_field(envelope, "attachments").unwrap()
        else {
            panic!("render tree attachments must be an array");
        };
        assert_eq!(exposed_attachments.len(), 2);
        let Value::Map(unknown) = &exposed_attachments[0] else {
            panic!("attachment must be a map");
        };
        assert_eq!(
            value_field(unknown, "schema"),
            Some(&Value::Text("example.unknown-schema".to_owned()))
        );
        assert_eq!(
            value_field(unknown, "identity"),
            Some(&Value::Bytes(b"opaque identity".to_vec()))
        );
        assert_eq!(
            value_field(unknown, "data"),
            Some(&Value::Bytes(vec![0xa2, 0x01, 0x02, 0x03]))
        );

        let root = value_field(envelope, "root").unwrap();
        let mut kinds = std::collections::BTreeSet::new();
        collect_node_kinds(root, &mut kinds);
        assert!(
            ["sum", "product", "power", "function", "variable", "number"]
                .into_iter()
                .all(|kind| kinds.contains(kind))
        );

        let function_node = find_node(root, "function").unwrap();
        let Value::Map(function_symbol) = value_field(function_node, "symbol").unwrap() else {
            panic!("function symbol must be a map");
        };
        assert_eq!(
            value_field(function_symbol, "name"),
            Some(&Value::Text("symbolica_payload_test::g".to_owned()))
        );
        assert_eq!(
            value_field(function_symbol, "namespace"),
            Some(&Value::Text("symbolica_payload_test".to_owned()))
        );
        assert_eq!(
            value_field(function_symbol, "short-name"),
            Some(&Value::Text("g".to_owned()))
        );
        assert_eq!(
            value_field(function_symbol, "tags"),
            Some(&Value::Array(vec![Value::Text(
                "symbolica_test::render".to_owned()
            )]))
        );
        let Value::Array(attributes) = value_field(function_symbol, "attributes").unwrap() else {
            panic!("attributes must be an array");
        };
        for attribute in ["symmetric", "linear", "real"] {
            assert!(attributes.contains(&Value::Text(attribute.to_owned())));
        }
        assert_eq!(
            value_field(function_node, "source"),
            Some(&Value::Text("g".to_owned()))
        );

        let Value::Bytes(call_payload) = value_field(function_node, "atom").unwrap() else {
            panic!("function atom must be bytes");
        };
        let parsed_call = parse_payload(call_payload).unwrap();
        assert_eq!(parsed_call.attachment_set(), render_attachments);
        assert_eq!(parsed_call.import_atom().unwrap(), rich_atom);

        let Value::Bytes(head_payload) = value_field(function_node, "head-atom").unwrap() else {
            panic!("function head Atom must be bytes");
        };
        let parsed_head = parse_payload(head_payload).unwrap();
        assert_eq!(parsed_head.attachment_set(), render_attachments);
        assert_eq!(parsed_head.import_atom().unwrap(), Atom::var(rich));

        let variable_node = find_node(root, "variable").unwrap();
        let Value::Bytes(variable_payload) = value_field(variable_node, "atom").unwrap() else {
            panic!("variable atom must be bytes");
        };
        let parsed_variable = parse_payload(variable_payload).unwrap();
        assert_eq!(parsed_variable.attachment_set(), render_attachments);
        assert!(matches!(
            parsed_variable.import_atom().unwrap().as_view(),
            AtomView::Var(_)
        ));

        let number_node = find_node(root, "number").unwrap();
        assert!(value_field(number_node, "atom").is_none());
        assert_eq!(
            value_field(number_node, "source"),
            value_field(number_node, "text")
        );

        let encoded_render_tree =
            encode_atom_render_tree(&structured, &render_attachments).unwrap();
        let decoded_render_tree =
            ciborium::from_reader::<Value, _>(Cursor::new(encoded_render_tree)).unwrap();
        assert_eq!(decoded_render_tree, render_tree);

        let custom = symbol!(
            "symbolica_payload_test::custom",
            print = |_view, _options, _state| Some("callback-must-not-run".to_owned())
        );
        let custom_tree =
            atom_render_tree_value(&Atom::var(custom), &AttachmentSet::new()).unwrap();
        let custom_variable = find_node(&custom_tree, "variable").unwrap();
        assert_eq!(
            value_field(custom_variable, "source"),
            Some(&Value::Text("\"custom\"".to_owned()))
        );
    }
}
