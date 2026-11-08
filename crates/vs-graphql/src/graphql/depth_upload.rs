#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use async_graphql::{Error, Result};

use super::DepthEncodingGql;
use crate::service::DepthFrame;

pub(super) fn depth_frame_from_bytes(
    width: u32,
    height: u32,
    encoding: DepthEncodingGql,
    bigendian: bool,
    bytes: &[u8],
) -> Result<DepthFrame> {
    let pixel_count = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .ok_or_else(|| Error::new("Depth dimensions overflow"))?;
    if pixel_count > 64_000_000 {
        return Err(Error::new(
            "Depth image exceeds the 64-million-pixel safety limit",
        ));
    }
    let sample_bytes = match encoding {
        DepthEncodingGql::Uint16Mm => 2,
        DepthEncodingGql::Float32M => 4,
    };
    let expected_bytes = pixel_count
        .checked_mul(sample_bytes)
        .ok_or_else(|| Error::new("Depth dimensions overflow"))?;
    if bytes.len() != expected_bytes {
        return Err(Error::new(format!(
            "Depth upload has {} bytes, expected {expected_bytes}",
            bytes.len()
        )));
    }

    let metres = match encoding {
        DepthEncodingGql::Uint16Mm => bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| {
                let raw = if bigendian {
                    u16::from_be_bytes([pair[0], pair[1]])
                } else {
                    u16::from_le_bytes([pair[0], pair[1]])
                };
                if raw == 0 {
                    f32::NAN
                } else {
                    f32::from(raw) / 1000.0
                }
            })
            .collect(),
        DepthEncodingGql::Float32M => bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|sample| {
                let bytes = [sample[0], sample[1], sample[2], sample[3]];
                f32::from_bits(if bigendian {
                    u32::from_be_bytes(bytes)
                } else {
                    u32::from_le_bytes(bytes)
                })
            })
            .collect(),
    };
    Ok(DepthFrame {
        width,
        height,
        metres,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_rep118_depth_units_and_endianness() {
        let depth =
            depth_frame_from_bytes(2, 1, DepthEncodingGql::Uint16Mm, true, &[0x03, 0xe8, 0, 0])
                .expect("big-endian 16UC1 should decode");
        assert_eq!(depth.metres[0], 1.0);
        assert!(depth.metres[1].is_nan());

        let metres = 1.25_f32.to_be_bytes();
        let depth = depth_frame_from_bytes(1, 1, DepthEncodingGql::Float32M, true, &metres)
            .expect("big-endian 32FC1 should decode");
        assert_eq!(depth.metres, vec![1.25]);
    }

    #[test]
    fn rejects_depth_payload_size_mismatch() {
        let error = depth_frame_from_bytes(2, 2, DepthEncodingGql::Uint16Mm, false, &[0; 7])
            .expect_err("wrong-sized depth should be rejected");
        assert!(error.message.contains("expected 8"));
    }
}
