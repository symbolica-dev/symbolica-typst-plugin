use std::convert::TryFrom;
use std::ffi::OsString;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::Path;

const CHUNK_SIZE: usize = 18 * 1024 * 1024;
const CARRIER_HEADER_LEN: usize = 32;
const MAX_WEB_FILE_SIZE: usize = 20 * 1024 * 1024;
const SECTION_NAME: &[u8] = b"tymbolica.payload";

fn encode_u32_fixed(value: u32) -> [u8; 5] {
    [
        (value as u8 & 0x7f) | 0x80,
        ((value >> 7) as u8 & 0x7f) | 0x80,
        ((value >> 14) as u8 & 0x7f) | 0x80,
        ((value >> 21) as u8 & 0x7f) | 0x80,
        (value >> 28) as u8,
    ]
}

fn write_carrier(path: &Path, payload: &[u8]) -> io::Result<()> {
    let section_size = 1 + SECTION_NAME.len() + payload.len();
    let section_size = u32::try_from(section_size)
        .map_err(|_| io::Error::new(ErrorKind::InvalidInput, "payload is too large"))?;

    let mut output = Vec::with_capacity(CARRIER_HEADER_LEN + payload.len());
    output.extend_from_slice(b"\0asm\x01\0\0\0");
    output.push(0); // Custom section.
    output.extend_from_slice(&encode_u32_fixed(section_size));
    output.push(SECTION_NAME.len() as u8);
    output.extend_from_slice(SECTION_NAME);
    assert_eq!(output.len(), CARRIER_HEADER_LEN);
    output.extend_from_slice(payload);
    fs::write(path, output)
}

fn verify_carrier(path: &Path, payload: &[u8]) -> io::Result<()> {
    let carrier = fs::read(path)?;
    if carrier.len() > MAX_WEB_FILE_SIZE {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            format!("{} exceeds the 20 MiB web limit", path.display()),
        ));
    }
    if carrier.len() != CARRIER_HEADER_LEN + payload.len()
        || carrier.get(CARRIER_HEADER_LEN..) != Some(payload)
    {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            format!("{} does not contain the expected payload", path.display()),
        ));
    }
    Ok(())
}

fn required_arg(args: &mut impl Iterator<Item = OsString>, label: &str) -> io::Result<OsString> {
    args.next().ok_or_else(|| {
        io::Error::new(
            ErrorKind::InvalidInput,
            format!("missing {label}; expected INPUT PART_0 PART_1"),
        )
    })
}

fn main() -> io::Result<()> {
    let mut args = std::env::args_os();
    let _program = args.next();
    let input_path = required_arg(&mut args, "input path")?;
    let first_path = required_arg(&mut args, "first output path")?;
    let second_path = required_arg(&mut args, "second output path")?;
    if args.next().is_some() {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "too many arguments; expected INPUT PART_0 PART_1",
        ));
    }

    let input = fs::read(&input_path)?;
    if input.len() > 2 * CHUNK_SIZE {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            format!(
                "{} bytes require more than two {CHUNK_SIZE}-byte carriers",
                input.len(),
            ),
        ));
    }

    let split = input.len().min(CHUNK_SIZE);
    let first_path = Path::new(&first_path);
    let second_path = Path::new(&second_path);
    write_carrier(first_path, &input[..split])?;
    write_carrier(second_path, &input[split..])?;
    verify_carrier(first_path, &input[..split])?;
    verify_carrier(second_path, &input[split..])?;

    eprintln!(
        "packed {} bytes into {}- and {}-byte payloads",
        input.len(),
        split,
        input.len() - split,
    );
    Ok(())
}
