//! Engine- and transport-independent transparent video output contracts.
//!
//! The output contract is deliberately owned by `vtuber-core`: Bevy readback
//! and any later network sender can exchange frames without leaking renderer or
//! NDI types across the application boundaries.

use std::sync::Arc;

use crate::types::{FrameSeq, MonoTimeNs};

/// Fixed pixel format used by the first transparent output profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum VideoOutputPixelFormat {
    /// 8-bit BGRA in straight (non-premultiplied) alpha form.
    #[default]
    Bgra8StraightAlpha,
}

/// The fixed output profile shared by rendering and later transport layers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VideoOutputProfile {
    /// Output width in pixels.
    pub width: u32,
    /// Output height in pixels.
    pub height: u32,
    /// Nominal output frame rate.
    pub fps: u32,
    /// Output byte layout and alpha semantics.
    pub pixel_format: VideoOutputPixelFormat,
}

impl VideoOutputProfile {
    /// The initial OBS-oriented output profile.
    pub const DEFAULT: Self = Self {
        width: 1920,
        height: 1080,
        fps: 60,
        pixel_format: VideoOutputPixelFormat::Bgra8StraightAlpha,
    };

    /// Returns the number of packed bytes in one row for this profile.
    #[must_use]
    pub const fn packed_stride_bytes(self) -> usize {
        self.width as usize * 4
    }
}

impl Default for VideoOutputProfile {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// A validated, owned transparent avatar frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VideoOutputFrame {
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Number of bytes between the start of adjacent rows.
    pub stride_bytes: usize,
    /// Pixel format and alpha semantics of `data`.
    pub pixel_format: VideoOutputPixelFormat,
    /// Monotonically identifiable output sequence.
    pub frame_seq: FrameSeq,
    /// Completion timestamp from the process-local monotonic clock.
    pub captured_at: MonoTimeNs,
    /// Packed, owned frame bytes.
    pub data: Arc<[u8]>,
}

impl VideoOutputFrame {
    /// Creates a validated packed BGRA8 frame.
    pub fn new_bgra8(
        width: u32,
        height: u32,
        frame_seq: FrameSeq,
        captured_at: MonoTimeNs,
        data: Vec<u8>,
    ) -> Result<Self, VideoOutputFrameError> {
        let stride_bytes = packed_stride(width)?;
        let expected_len = checked_len(stride_bytes, height)?;
        if data.len() != expected_len {
            return Err(VideoOutputFrameError::DataLength {
                expected: expected_len,
                actual: data.len(),
            });
        }
        Ok(Self {
            width,
            height,
            stride_bytes,
            pixel_format: VideoOutputPixelFormat::Bgra8StraightAlpha,
            frame_seq,
            captured_at,
            data: data.into(),
        })
    }

    /// Converts a GPU texture readback with aligned rows into one packed frame.
    ///
    /// The readback buffer is copied row-by-row because wgpu requires the
    /// source stride to be aligned to `COPY_BYTES_PER_ROW_ALIGNMENT`. The
    /// resulting frame has no padding and is normalized to straight alpha.
    pub fn from_padded_bgra8(
        width: u32,
        height: u32,
        source_stride_bytes: usize,
        frame_seq: FrameSeq,
        captured_at: MonoTimeNs,
        readback: &[u8],
    ) -> Result<Self, VideoOutputFrameError> {
        let packed_stride = packed_stride(width)?;
        if source_stride_bytes < packed_stride {
            return Err(VideoOutputFrameError::InvalidStride {
                minimum: packed_stride,
                actual: source_stride_bytes,
            });
        }
        let expected_readback_len = checked_len(source_stride_bytes, height)?;
        if readback.len() != expected_readback_len {
            return Err(VideoOutputFrameError::DataLength {
                expected: expected_readback_len,
                actual: readback.len(),
            });
        }

        let packed_len = checked_len(packed_stride, height)?;
        let mut data = vec![0; packed_len];
        for row in 0..height as usize {
            let source_start = row * source_stride_bytes;
            let destination_start = row * packed_stride;
            data[destination_start..destination_start + packed_stride]
                .copy_from_slice(&readback[source_start..source_start + packed_stride]);
        }
        unpremultiply_bgra8_in_place(&mut data)?;
        Self::new_bgra8(width, height, frame_seq, captured_at, data)
    }
}

/// Errors raised while validating or converting an output frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VideoOutputFrameError {
    /// Width or height was zero.
    ZeroDimensions,
    /// A stride was too short to hold one packed BGRA row.
    InvalidStride {
        /// Smallest valid stride.
        minimum: usize,
        /// Supplied stride.
        actual: usize,
    },
    /// A multiplication needed to validate a buffer overflowed.
    SizeOverflow,
    /// The byte count did not match the declared dimensions and stride.
    DataLength {
        /// Required byte count.
        expected: usize,
        /// Supplied byte count.
        actual: usize,
    },
    /// A pixel buffer was not a whole number of four-byte BGRA pixels.
    PixelDataNotAligned,
}

impl std::fmt::Display for VideoOutputFrameError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroDimensions => formatter.write_str("video output dimensions must be non-zero"),
            Self::InvalidStride { minimum, actual } => write!(
                formatter,
                "video output stride {actual} is smaller than packed stride {minimum}"
            ),
            Self::SizeOverflow => formatter.write_str("video output buffer size overflowed"),
            Self::DataLength { expected, actual } => write!(
                formatter,
                "video output buffer has {actual} bytes; expected {expected}"
            ),
            Self::PixelDataNotAligned => {
                formatter.write_str("BGRA8 pixel data is not aligned to four-byte pixels")
            }
        }
    }
}

impl std::error::Error for VideoOutputFrameError {}

/// Converts premultiplied BGRA8 pixels to straight alpha in place.
///
/// `A == 0` clears RGB, `A == 255` is unchanged, and intermediate values use
/// nearest-integer rounding with a bounded result. This function is safe to
/// apply to a complete packed frame exactly once.
pub fn unpremultiply_bgra8_in_place(data: &mut [u8]) -> Result<(), VideoOutputFrameError> {
    if !data.len().is_multiple_of(4) {
        return Err(VideoOutputFrameError::PixelDataNotAligned);
    }
    for pixel in data.chunks_exact_mut(4) {
        let alpha = u16::from(pixel[3]);
        if alpha == 0 {
            pixel[0] = 0;
            pixel[1] = 0;
            pixel[2] = 0;
        } else if alpha != u16::from(u8::MAX) {
            for channel in &mut pixel[..3] {
                let value = (u16::from(*channel) * u16::from(u8::MAX) + alpha / 2) / alpha;
                *channel = value.min(u16::from(u8::MAX)) as u8;
            }
        }
    }
    Ok(())
}

fn packed_stride(width: u32) -> Result<usize, VideoOutputFrameError> {
    if width == 0 {
        return Err(VideoOutputFrameError::ZeroDimensions);
    }
    (width as usize)
        .checked_mul(4)
        .ok_or(VideoOutputFrameError::SizeOverflow)
}

fn checked_len(stride: usize, height: u32) -> Result<usize, VideoOutputFrameError> {
    if height == 0 {
        return Err(VideoOutputFrameError::ZeroDimensions);
    }
    stride
        .checked_mul(height as usize)
        .ok_or(VideoOutputFrameError::SizeOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timestamp() -> MonoTimeNs {
        MonoTimeNs(7)
    }

    #[test]
    fn default_profile_is_fixed_and_packed_bgra8() {
        let profile = VideoOutputProfile::default();
        assert_eq!(profile.width, 1920);
        assert_eq!(profile.height, 1080);
        assert_eq!(profile.fps, 60);
        assert_eq!(
            profile.pixel_format,
            VideoOutputPixelFormat::Bgra8StraightAlpha
        );
        assert_eq!(profile.packed_stride_bytes(), 7680);
    }

    #[test]
    fn alpha_boundaries_are_safe_and_stable() {
        let mut pixels = [
            50, 60, 70, 0, // transparent becomes black
            1, 2, 3, 1, // tiny alpha is bounded at 255
            63, 64, 65, 127, 64, 65, 66, 128, 254, 253, 252, 254, 1, 2, 3,
            255, // opaque is unchanged
        ];
        unpremultiply_bgra8_in_place(&mut pixels).expect("whole pixels are valid");
        assert_eq!(&pixels[..4], &[0, 0, 0, 0]);
        assert_eq!(&pixels[4..8], &[255, 255, 255, 1]);
        assert_eq!(&pixels[20..24], &[1, 2, 3, 255]);
        assert_eq!(pixels[3], 0);
    }

    #[test]
    fn padded_readback_is_packed_and_unpremultiplied_once() {
        let mut readback = vec![0_u8; 8];
        readback[..4].copy_from_slice(&[25, 50, 75, 128]);
        readback[4..].copy_from_slice(&[99, 98, 97, 96]);
        let frame =
            VideoOutputFrame::from_padded_bgra8(1, 1, 8, FrameSeq(3), timestamp(), &readback)
                .expect("padded one-pixel readback is valid");
        assert_eq!(frame.stride_bytes, 4);
        assert_eq!(&*frame.data, &[50, 100, 149, 128]);
        assert_eq!(frame.frame_seq, FrameSeq(3));
    }

    #[test]
    fn invalid_frame_shapes_are_rejected() {
        assert_eq!(
            VideoOutputFrame::new_bgra8(0, 1, FrameSeq(0), timestamp(), vec![]),
            Err(VideoOutputFrameError::ZeroDimensions)
        );
        assert_eq!(
            VideoOutputFrame::from_padded_bgra8(2, 1, 4, FrameSeq(0), timestamp(), &[0; 4]),
            Err(VideoOutputFrameError::InvalidStride {
                minimum: 8,
                actual: 4
            })
        );
        assert_eq!(
            unpremultiply_bgra8_in_place(&mut [0, 1, 2]),
            Err(VideoOutputFrameError::PixelDataNotAligned)
        );
    }
}
