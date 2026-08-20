//! Versioned Symbolica Atom payloads exchanged by Tymbolica-compatible plugins.
//!
//! The envelope deliberately keeps the native Symbolica export opaque. Consumers
//! can validate and inspect portable attachments before choosing to call
//! [`Atom::import`] through [`ParsedPayload::import_atom`].

use std::{
    collections::{BTreeMap, btree_map::Entry},
    fmt,
    io::Cursor,
    str,
};

use symbolica::prelude::Atom;

/// Exact Symbolica revision shared by producers and consumers.
///
/// The build script derives this from the pinned `symbolica-upstream`
/// dependency in `vendor/symbolica-wasm/Cargo.toml`.
pub const SYMBOLICA_REVISION: &str = env!("TYMBOLICA_SYMBOLICA_REVISION");

/// Magic prefix for the versioned envelope.
pub const PAYLOAD_MAGIC: &[u8; 8] = b"TYMATOM\0";
/// Current binary-envelope version.
pub const PAYLOAD_VERSION: u16 = 1;

pub const MAX_ATOM_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_PAYLOAD_BYTES: usize = 10 * 1024 * 1024;
pub const MAX_ATTACHMENTS: usize = 256;
pub const MAX_ATTACHMENT_KEY_BYTES: usize = 1024;
pub const MAX_ATTACHMENT_SCHEMA_BYTES: usize = 128;
pub const MAX_ATTACHMENT_DATA_BYTES: usize = 256 * 1024;
pub const MAX_TOTAL_ATTACHMENT_BYTES: usize = 1024 * 1024;

const REVISION_BYTES: usize = 40;
const LEGACY_SYMBOLICA_MAGIC: u32 = 0x3787_1367;
const LEGACY_SYMBOLICA_EXPORT_VERSION: u16 = 4;
const LEGACY_SYMBOLICA_HEADER_BYTES: usize =
    std::mem::size_of::<u32>() + std::mem::size_of::<u16>();
// Permit redundant records to be merged without letting their wire count grow
// without bound. MAX_ATTACHMENTS applies to unique keys.
const MAX_ENCODED_ATTACHMENT_RECORDS: usize = 1024;
const FIXED_HEADER_BYTES: usize = PAYLOAD_MAGIC.len() + 2 + 2 + 2 + 2 + 4;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PayloadFormat {
    /// A pre-envelope native Symbolica export. It has no recorded revision or
    /// attachments, but remains importable for backwards compatibility.
    LegacyRawAtom,
    EnvelopeV1,
}

/// A validated payload whose native Atom bytes have not yet been imported.
#[derive(Debug)]
pub struct ParsedPayload<'a> {
    format: PayloadFormat,
    symbolica_revision: Option<&'a str>,
    atom_bytes: &'a [u8],
    attachments: Vec<AttachmentRef<'a>>,
}

impl<'a> ParsedPayload<'a> {
    pub fn format(&self) -> PayloadFormat {
        self.format
    }

    /// Revision recorded by an envelope, or `None` for a legacy raw export.
    pub fn symbolica_revision(&self) -> Option<&'a str> {
        self.symbolica_revision
    }

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

    /// Check whether this payload can be imported by the compiled Symbolica.
    ///
    /// Legacy raw exports contain no exact revision marker. Their native magic
    /// and export-format version are checked during parsing, but compatibility
    /// beyond that remains best-effort.
    pub fn ensure_import_compatible(&self) -> Result<(), PayloadError> {
        if let Some(revision) = self.symbolica_revision
            && revision != SYMBOLICA_REVISION
        {
            return Err(PayloadError::RevisionMismatch(revision.to_owned()));
        }
        Ok(())
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
        self.ensure_import_compatible()?;
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
    RevisionMismatch(String),
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
            Self::RevisionMismatch(found) => write!(
                formatter,
                "Atom payload uses Symbolica revision {found}, expected {SYMBOLICA_REVISION}"
            ),
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

fn validate_revision(revision: &str) -> Result<(), PayloadError> {
    if revision.len() != REVISION_BYTES
        || !revision
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(PayloadError::InvalidEnvelope(
            "Symbolica revision must be a 40-character lowercase hexadecimal Git ID",
        ));
    }
    Ok(())
}

fn validate_legacy_header(input: &[u8]) -> Result<(), PayloadError> {
    let header =
        input
            .get(..LEGACY_SYMBOLICA_HEADER_BYTES)
            .ok_or(PayloadError::InvalidEnvelope(
                "input is neither a current envelope nor a complete legacy Symbolica header",
            ))?;
    let magic = u32::from_le_bytes(header[..4].try_into().expect("four-byte slice"));
    if magic != LEGACY_SYMBOLICA_MAGIC {
        return Err(PayloadError::InvalidEnvelope(
            "input is neither a current envelope nor a legacy Symbolica export",
        ));
    }
    let version = u16::from_le_bytes(header[4..].try_into().expect("two-byte slice"));
    if version != LEGACY_SYMBOLICA_EXPORT_VERSION {
        return Err(PayloadError::InvalidEnvelope(
            "legacy Symbolica export format is not supported",
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

fn encode_exported_atom_from_set_with_revision(
    atom_bytes: &[u8],
    revision: &str,
    attachments: &AttachmentSet,
) -> Result<Vec<u8>, PayloadError> {
    if atom_bytes.is_empty() {
        return Err(PayloadError::InvalidEnvelope("Atom export cannot be empty"));
    }
    if atom_bytes.len() > MAX_ATOM_BYTES {
        return Err(PayloadError::LimitExceeded);
    }
    validate_revision(revision)?;

    let attachment_bytes = attachments.total_attachment_bytes();
    if attachment_bytes > MAX_TOTAL_ATTACHMENT_BYTES {
        return Err(PayloadError::LimitExceeded);
    }

    let entry_headers = attachments
        .len()
        .checked_mul(2 + 4 + 4 + 4)
        .ok_or(PayloadError::LimitExceeded)?;
    let total_size = FIXED_HEADER_BYTES
        .checked_add(revision.len())
        .and_then(|total| total.checked_add(atom_bytes.len()))
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
    push_u16(&mut output, revision.len())?;
    push_u16(&mut output, attachments.len())?;
    push_u32(&mut output, atom_bytes.len())?;
    output.extend_from_slice(revision.as_bytes());
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

/// Wrap already-exported native Symbolica Atom bytes with a reusable set.
fn encode_exported_atom_from_set(
    atom_bytes: &[u8],
    attachments: &AttachmentSet,
) -> Result<Vec<u8>, PayloadError> {
    encode_exported_atom_from_set_with_revision(atom_bytes, SYMBOLICA_REVISION, attachments)
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
    let revision_length = usize::from(reader.u16()?);
    let entry_count = usize::from(reader.u16()?);
    let atom_length = usize::try_from(reader.u32()?).map_err(|_| PayloadError::LimitExceeded)?;
    if revision_length != REVISION_BYTES {
        return Err(PayloadError::InvalidEnvelope(
            "Symbolica revision must be a 40-character lowercase hexadecimal Git ID",
        ));
    }
    if entry_count > MAX_ENCODED_ATTACHMENT_RECORDS || atom_length > MAX_ATOM_BYTES {
        return Err(PayloadError::LimitExceeded);
    }
    if atom_length == 0 {
        return Err(PayloadError::InvalidEnvelope("Atom export cannot be empty"));
    }

    let revision = str::from_utf8(reader.take(revision_length)?)
        .map_err(|_| PayloadError::InvalidEnvelope("Symbolica revision is not UTF-8"))?;
    validate_revision(revision)?;
    let atom_bytes = reader.take(atom_length)?;

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
        format: PayloadFormat::EnvelopeV1,
        symbolica_revision: Some(revision),
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

/// Validate an envelope and expose its metadata without importing the Atom.
///
/// Inputs without [`PAYLOAD_MAGIC`] must begin with the native Symbolica magic
/// and supported export-format version. Such legacy exports expose an empty
/// attachment list and have no exact revision marker.
pub fn parse_payload(input: &[u8]) -> Result<ParsedPayload<'_>, PayloadError> {
    if input.len() > MAX_PAYLOAD_BYTES {
        return Err(PayloadError::LimitExceeded);
    }
    if input.starts_with(PAYLOAD_MAGIC) {
        parse_envelope(input)
    } else {
        if input.len() > MAX_ATOM_BYTES {
            return Err(PayloadError::LimitExceeded);
        }
        validate_legacy_header(input)?;
        Ok(ParsedPayload {
            format: PayloadFormat::LegacyRawAtom,
            symbolica_revision: None,
            atom_bytes: input,
            attachments: Vec::new(),
        })
    }
}

/// Import either a current envelope or a legacy raw Symbolica export.
pub fn decode_atom(input: &[u8]) -> Result<Atom, PayloadError> {
    parse_payload(input)?.import_atom()
}

#[cfg(test)]
mod tests {
    use super::*;
    use symbolica::prelude::{Coefficient, Complex, Float};
    use symbolica::{function, parse, symbol};

    const OPAQUE_ATOM_EXPORT: &[u8] = b"opaque native Atom export";

    fn key(schema: &str, version: u32, identity: &[u8]) -> AttachmentKey {
        AttachmentKey::new(schema, version, identity.to_vec()).unwrap()
    }

    fn attachment(schema: &str, version: u32, identity: &[u8], data: &[u8]) -> Attachment {
        Attachment::new(key(schema, version, identity), data.to_vec()).unwrap()
    }

    fn legacy_export(atom: &Atom) -> Vec<u8> {
        export_raw_atom(atom).unwrap()
    }

    fn duplicate_last_entry(mut payload: Vec<u8>, conflicting: bool) -> Vec<u8> {
        let revision_length = usize::from(u16::from_be_bytes(payload[12..14].try_into().unwrap()));
        let atom_length =
            usize::try_from(u32::from_be_bytes(payload[16..20].try_into().unwrap())).unwrap();
        let entry_start = FIXED_HEADER_BYTES + revision_length + atom_length;
        let mut duplicate = payload[entry_start..].to_vec();
        if conflicting {
            *duplicate.last_mut().unwrap() ^= 1;
        }
        payload[14..16].copy_from_slice(&2u16.to_be_bytes());
        payload.extend_from_slice(&duplicate);
        payload
    }

    fn repeat_last_entry(mut payload: Vec<u8>, count: u16) -> Vec<u8> {
        assert!(count >= 1);
        let revision_length = usize::from(u16::from_be_bytes(payload[12..14].try_into().unwrap()));
        let atom_length =
            usize::try_from(u32::from_be_bytes(payload[16..20].try_into().unwrap())).unwrap();
        let entry_start = FIXED_HEADER_BYTES + revision_length + atom_length;
        let entry = payload[entry_start..].to_vec();
        payload[14..16].copy_from_slice(&count.to_be_bytes());
        for _ in 1..count {
            payload.extend_from_slice(&entry);
        }
        payload
    }

    #[test]
    fn envelope_supports_two_stage_inspection_without_import() {
        let attachment_key = key("org.tymbolica.test", 1, b"namespace::f");
        let payload = encode_exported_atom(
            OPAQUE_ATOM_EXPORT,
            [Attachment::new(attachment_key.clone(), b"metadata".to_vec()).unwrap()],
        )
        .unwrap();

        let parsed = parse_payload(&payload).unwrap();
        assert_eq!(parsed.format(), PayloadFormat::EnvelopeV1);
        assert_eq!(parsed.symbolica_revision(), Some(SYMBOLICA_REVISION));
        assert_eq!(parsed.attachments().len(), 1);
        assert_eq!(
            parsed.attachment(&attachment_key),
            Some(b"metadata".as_slice())
        );
        assert_eq!(parsed.atom_bytes(), OPAQUE_ATOM_EXPORT);
    }

    #[test]
    fn encoding_is_deterministic_and_merges_identical_entries() {
        let a = attachment("org.tymbolica.a", 1, b"a", b"first");
        let b = attachment("org.tymbolica.b", 2, b"b", b"second");
        let forward = encode_exported_atom(OPAQUE_ATOM_EXPORT, [a.clone(), b.clone()]).unwrap();
        let reversed_with_duplicate =
            encode_exported_atom(OPAQUE_ATOM_EXPORT, [b, a.clone(), a]).unwrap();

        assert_eq!(forward, reversed_with_duplicate);
        let parsed = parse_payload(&forward).unwrap();
        assert_eq!(parsed.attachments().len(), 2);
        assert_eq!(parsed.attachments()[0].schema(), "org.tymbolica.a");
        assert_eq!(parsed.attachments()[1].schema(), "org.tymbolica.b");
    }

    #[test]
    fn attachment_sets_merge_associatively_and_reencode_raw_exports() {
        let a = AttachmentSet::from_attachments([attachment("org.tymbolica.a", 1, b"a", b"first")])
            .unwrap();
        let b = AttachmentSet::from_attachments([
            attachment("org.tymbolica.a", 1, b"a", b"first"),
            attachment("org.tymbolica.b", 1, b"b", b"second"),
        ])
        .unwrap();
        let c = AttachmentSet::from_attachments([attachment("org.tymbolica.c", 2, b"c", b"third")])
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
            left.get(&key("org.tymbolica.b", 1, b"b")),
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
        let repeated = attachment("org.tymbolica.same", 1, b"same", b"same-data");
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
            .map(|index| attachment("org.tymbolica.unique", 1, &index.to_be_bytes(), b"data"))
            .collect::<Vec<_>>();
        assert!(matches!(
            AttachmentSet::from_attachments(unique),
            Err(PayloadError::LimitExceeded)
        ));
    }

    #[test]
    fn attachment_set_conflicts_are_transactional() {
        let mut target = AttachmentSet::from_attachments([
            attachment("org.tymbolica.a", 1, b"a", b"original"),
            attachment("org.tymbolica.b", 1, b"b", b"stable"),
        ])
        .unwrap();
        let before = target.clone();
        let conflicting = AttachmentSet::from_attachments([
            attachment("org.tymbolica.c", 1, b"c", b"new"),
            attachment("org.tymbolica.a", 1, b"a", b"different"),
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
                .map(|index| attachment("org.tymbolica.count", 1, &index.to_be_bytes(), b"data")),
        )
        .unwrap();
        let count_before = count_limited.clone();
        let count_overflow = AttachmentSet::from_attachments([attachment(
            "org.tymbolica.count",
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
                attachment("org.tymbolica.bytes", 1, &index.to_be_bytes(), &large_data)
            }))
            .unwrap();
        let byte_before = byte_limited.clone();
        let byte_overflow = AttachmentSet::from_attachments([attachment(
            "org.tymbolica.bytes",
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
        let first = attachment("org.tymbolica.test", 1, b"same", b"one");
        let second = attachment("org.tymbolica.test", 1, b"same", b"two");
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
    fn revision_can_be_inspected_before_import_is_rejected() {
        let mut payload = encode_exported_atom(
            OPAQUE_ATOM_EXPORT,
            [attachment("org.tymbolica.test", 1, b"id", b"data")],
        )
        .unwrap();
        payload[FIXED_HEADER_BYTES] = if payload[FIXED_HEADER_BYTES] == b'0' {
            b'1'
        } else {
            b'0'
        };

        let parsed = parse_payload(&payload).unwrap();
        assert_ne!(parsed.symbolica_revision(), Some(SYMBOLICA_REVISION));
        assert_eq!(parsed.attachments()[0].data(), b"data");
        assert!(matches!(
            parsed.ensure_import_compatible(),
            Err(PayloadError::RevisionMismatch(_))
        ));
        assert!(matches!(
            parsed.import_atom(),
            Err(PayloadError::RevisionMismatch(_))
        ));
    }

    #[test]
    fn truncated_envelopes_are_always_rejected() {
        let payload = encode_exported_atom(
            OPAQUE_ATOM_EXPORT,
            [attachment("org.tymbolica.test", 1, b"id", b"data")],
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
    fn malformed_envelope_and_legacy_headers_are_rejected() {
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

        let mut wrong_revision_length = payload.clone();
        wrong_revision_length[12..14].copy_from_slice(&39_u16.to_be_bytes());
        assert!(matches!(
            parse_payload(&wrong_revision_length),
            Err(PayloadError::InvalidEnvelope(_))
        ));

        let mut malformed_revision = payload.clone();
        malformed_revision[FIXED_HEADER_BYTES] = b'G';
        assert!(matches!(
            parse_payload(&malformed_revision),
            Err(PayloadError::InvalidEnvelope(_))
        ));

        let mut empty_atom = payload;
        empty_atom[16..20].copy_from_slice(&0_u32.to_be_bytes());
        assert!(matches!(
            parse_payload(&empty_atom),
            Err(PayloadError::InvalidEnvelope(_))
        ));

        assert!(matches!(
            parse_payload(b"not an Atom payload"),
            Err(PayloadError::InvalidEnvelope(_))
        ));
        let mut unsupported_legacy = Vec::new();
        unsupported_legacy.extend_from_slice(&LEGACY_SYMBOLICA_MAGIC.to_le_bytes());
        unsupported_legacy.extend_from_slice(&(LEGACY_SYMBOLICA_EXPORT_VERSION + 1).to_le_bytes());
        assert!(matches!(
            parse_payload(&unsupported_legacy),
            Err(PayloadError::InvalidEnvelope(_))
        ));

        let attachments = AttachmentSet::new();
        assert!(matches!(
            encode_exported_atom_from_set_with_revision(
                OPAQUE_ATOM_EXPORT,
                &"0".repeat(39),
                &attachments
            ),
            Err(PayloadError::InvalidEnvelope(_))
        ));
        assert!(matches!(
            encode_exported_atom_from_set_with_revision(
                OPAQUE_ATOM_EXPORT,
                &"A".repeat(REVISION_BYTES),
                &attachments
            ),
            Err(PayloadError::InvalidEnvelope(_))
        ));
    }

    #[test]
    fn limits_and_trailing_bytes_are_enforced() {
        assert!(matches!(
            AttachmentKey::new("org.tymbolica.test", 1, vec![0; MAX_ATTACHMENT_KEY_BYTES]),
            Err(PayloadError::LimitExceeded)
        ));
        assert!(matches!(
            Attachment::new(
                key("org.tymbolica.test", 1, b"id"),
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
        too_many_records[14..16].copy_from_slice(
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
    fn native_atoms_round_trip_and_legacy_remains_compatible() {
        // A restricted Symbolica build permits one instance per process. Keep
        // all native Atom operations in one test thread; the other tests only
        // inspect the deliberately opaque exported bytes.
        let atom = parse!("f(x)^2+1/3");
        let payload = encode_atom_with_attachments(
            &atom,
            [attachment(
                "org.tymbolica.test",
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

        let legacy = legacy_export(&atom);
        let parsed_legacy = parse_payload(&legacy).unwrap();
        assert_eq!(parsed_legacy.format(), PayloadFormat::LegacyRawAtom);
        assert_eq!(parsed_legacy.symbolica_revision(), None);
        assert!(parsed_legacy.attachments().is_empty());
        assert_eq!(parsed_legacy.import_atom().unwrap(), atom);
        assert_eq!(decode_atom(&legacy).unwrap(), atom);

        let rich = symbol!("tymbolica_payload_test::g"; Symmetric, Linear, Real);
        let rich_atom = function!(
            rich,
            symbol!("tymbolica_payload_test::y"),
            symbol!("tymbolica_payload_test::x")
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
    }
}
