//! Minimal `.npy` reader for the voice style tensors.
//!
//! The engine only ever reads two fixed-shape float32 arrays, so pulling in
//! `ndarray-npy` (and its `ndarray` dependency) would be a large amount of
//! surface for a 40-line parser. This reader accepts exactly what the exporter
//! writes: little-endian float32, C order, no pickles. Anything else is a
//! typed error, which is what we want — a style file from a different export
//! must not be silently reinterpreted.

use std::fs;
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

use crate::error::{Error, Result};

static DESCR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"'descr'\s*:\s*'([^']+)'").unwrap());
static ORDER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"'fortran_order'\s*:\s*(True|False)").unwrap());
static SHAPE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"'shape'\s*:\s*\(([^)]*)\)").unwrap());

const MAGIC: &[u8; 6] = b"\x93NUMPY";

/// Read a float32 `.npy` file, returning its shape and data in C order.
pub fn load_f32(path: &Path) -> Result<(Vec<usize>, Vec<f32>)> {
    let bytes = fs::read(path)
        .map_err(|e| Error::BundleCorrupt(format!("cannot read {}: {e}", path.display())))?;
    parse_f32(&bytes).map_err(|e| match e {
        Error::BundleCorrupt(message) => {
            Error::BundleCorrupt(format!("{}: {message}", path.display()))
        }
        other => other,
    })
}

/// Parse an in-memory `.npy` image.
pub fn parse_f32(bytes: &[u8]) -> Result<(Vec<usize>, Vec<f32>)> {
    if bytes.len() < 10 || &bytes[..6] != MAGIC {
        return Err(Error::BundleCorrupt("not an .npy file".into()));
    }
    let major = bytes[6];
    let (header_len, header_start): (usize, usize) = match major {
        1 => (u16::from_le_bytes([bytes[8], bytes[9]]) as usize, 10),
        2 | 3 => {
            if bytes.len() < 12 {
                return Err(Error::BundleCorrupt("truncated .npy header".into()));
            }
            (
                u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize,
                12,
            )
        }
        other => {
            return Err(Error::BundleCorrupt(format!(
                "unsupported .npy version {other}"
            )));
        }
    };
    let header_end = header_start
        .checked_add(header_len)
        .ok_or_else(|| Error::BundleCorrupt("invalid .npy header length".into()))?;
    if header_end > bytes.len() {
        return Err(Error::BundleCorrupt("truncated .npy header".into()));
    }
    let header = std::str::from_utf8(&bytes[header_start..header_end])
        .map_err(|_| Error::BundleCorrupt("non-UTF-8 .npy header".into()))?;

    let descr = DESCR_RE
        .captures(header)
        .map(|captures| captures[1].to_string())
        .ok_or_else(|| Error::BundleCorrupt("missing 'descr' in .npy header".into()))?;
    if !matches!(descr.as_str(), "<f4" | "|f4" | "=f4") {
        return Err(Error::BundleCorrupt(format!(
            "unsupported .npy dtype {descr:?} (expected little-endian float32)"
        )));
    }
    let order = ORDER_RE
        .captures(header)
        .map(|captures| captures[1].to_string())
        .ok_or_else(|| Error::BundleCorrupt("missing 'fortran_order' in .npy header".into()))?;
    if order != "False" {
        return Err(Error::BundleCorrupt(
            "Fortran-ordered .npy is not supported".into(),
        ));
    }
    let shape_text = SHAPE_RE
        .captures(header)
        .map(|captures| captures[1].to_string())
        .ok_or_else(|| Error::BundleCorrupt("missing 'shape' in .npy header".into()))?;
    let mut shape = Vec::new();
    for part in shape_text.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let dim: usize = part
            .parse()
            .map_err(|_| Error::BundleCorrupt(format!("invalid .npy shape {shape_text:?}")))?;
        shape.push(dim);
    }
    if shape.is_empty() {
        return Err(Error::BundleCorrupt("empty .npy shape".into()));
    }
    let count: usize = shape
        .iter()
        .try_fold(1usize, |acc, dim| acc.checked_mul(*dim))
        .ok_or_else(|| Error::BundleCorrupt("overflowing .npy shape".into()))?;
    let data = &bytes[header_end..];
    let expected = count
        .checked_mul(4)
        .ok_or_else(|| Error::BundleCorrupt("overflowing .npy size".into()))?;
    if data.len() != expected {
        return Err(Error::BundleCorrupt(format!(
            ".npy data is {} bytes, shape {:?} needs {expected}",
            data.len(),
            shape
        )));
    }
    let values = data
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect();
    Ok((shape, values))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn npy_bytes(header: &str, data: &[u8]) -> Vec<u8> {
        let mut header = header.to_string();
        let padding = 16 - (10 + header.len() + 1) % 16;
        header.push_str(&" ".repeat(padding));
        header.push('\n');
        let mut bytes = Vec::new();
        bytes.extend_from_slice(MAGIC);
        bytes.push(1);
        bytes.push(0);
        bytes.extend_from_slice(&(header.len() as u16).to_le_bytes());
        bytes.extend_from_slice(header.as_bytes());
        bytes.extend_from_slice(data);
        bytes
    }

    #[test]
    fn parses_a_float32_array() {
        let data: Vec<u8> = [1.0f32, -2.5]
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect();
        let bytes = npy_bytes(
            "{'descr': '<f4', 'fortran_order': False, 'shape': (1, 2), }",
            &data,
        );
        let (shape, values) = parse_f32(&bytes).unwrap();
        assert_eq!(shape, vec![1, 2]);
        assert_eq!(values, vec![1.0, -2.5]);
    }

    #[test]
    fn rejects_a_truncated_payload() {
        let bytes = npy_bytes(
            "{'descr': '<f4', 'fortran_order': False, 'shape': (1, 2), }",
            &[0u8; 4],
        );
        assert!(parse_f32(&bytes).is_err());
    }

    #[test]
    fn rejects_fortran_order_and_other_dtypes() {
        let data = [0u8; 8];
        let fortran = npy_bytes(
            "{'descr': '<f4', 'fortran_order': True, 'shape': (2,), }",
            &data,
        );
        assert!(parse_f32(&fortran).is_err());
        let f64 = npy_bytes(
            "{'descr': '<f8', 'fortran_order': False, 'shape': (1,), }",
            &data,
        );
        assert!(parse_f32(&f64).is_err());
    }

    #[test]
    fn rejects_a_bad_magic() {
        assert!(parse_f32(b"not a npy file at all").is_err());
    }
}
