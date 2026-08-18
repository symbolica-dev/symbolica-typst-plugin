//! Native Symbolica Atom bytes exchanged by Tymbolica-compatible plugins.

use std::{fmt, io::Cursor};

use symbolica::prelude::Atom;

/// Exact Symbolica revision shared by producers and consumers in this prototype.
pub const SYMBOLICA_REVISION: &str = "9ad7ca3f59f9ed8637e3f4ae8157ead177662994";
pub const MAX_ATOM_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug)]
pub enum PayloadError {
    LimitExceeded,
    TrailingBytes,
    Export(std::io::Error),
    Import(std::io::Error),
}

impl fmt::Display for PayloadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LimitExceeded => formatter.write_str("Atom export exceeds its size limit"),
            Self::TrailingBytes => formatter.write_str("Atom export has trailing bytes"),
            Self::Export(error) => write!(formatter, "could not export Atom: {error}"),
            Self::Import(error) => write!(formatter, "could not import Atom: {error}"),
        }
    }
}

impl std::error::Error for PayloadError {}

/// Export one Atom using Symbolica's native Atom-and-state representation.
pub fn encode_atom(atom: &Atom) -> Result<Vec<u8>, PayloadError> {
    let mut output = Vec::new();
    atom.as_view()
        .export(&mut output)
        .map_err(PayloadError::Export)?;
    if output.len() > MAX_ATOM_BYTES {
        return Err(PayloadError::LimitExceeded);
    }
    Ok(output)
}

/// Import one Atom using Symbolica's native Atom-and-state representation.
pub fn decode_atom(input: &[u8]) -> Result<Atom, PayloadError> {
    if input.len() > MAX_ATOM_BYTES {
        return Err(PayloadError::LimitExceeded);
    }

    let mut cursor = Cursor::new(input);
    let atom = Atom::import(&mut cursor, None).map_err(PayloadError::Import)?;
    if cursor.position() != input.len() as u64 {
        return Err(PayloadError::TrailingBytes);
    }
    Ok(atom)
}

#[cfg(test)]
mod tests {
    use super::*;
    use symbolica::prelude::{Coefficient, Complex, Float};
    use symbolica::{function, parse, symbol};

    #[test]
    fn native_export_round_trips() {
        let atom = parse!("f(x)^2+1/3");
        let payload = encode_atom(&atom).unwrap();
        assert_eq!(decode_atom(&payload).unwrap(), atom);

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

        let mut payload = encode_atom(&parse!("x")).unwrap();
        payload.push(0);
        assert!(matches!(
            decode_atom(&payload),
            Err(PayloadError::TrailingBytes)
        ));
    }
}
