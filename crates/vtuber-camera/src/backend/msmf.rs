//! Windows MSMF camera backend using `nokhwa`.
//!
//! The native `nokhwa::Camera` object is constructed, opened, used, and
//! dropped entirely within the capture worker thread. It is never sent across
//! threads. The stream is deliberately not `Send`; it never crosses the
//! capture worker boundary.

use nokhwa::Camera;
use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{
    ApiBackend, CameraFormat as NokhwaFormat, CameraIndex, FrameFormat, Resolution,
};
use vtuber_core::{FrameSeq, MonoTimeNs, PixelFormat, StopToken, VideoFrame};

use crate::device::{
    CameraBackend, CameraDescriptor, CameraError, CameraFormat, CameraRequest, CameraStream,
};
use crate::format::{FormatCandidate, select_format};

/// Windows MSMF camera backend.
pub struct MsmfBackend;

impl MsmfBackend {
    /// Creates a new MSMF backend.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for MsmfBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl CameraBackend for MsmfBackend {
    fn enumerate(&self) -> Result<Vec<CameraDescriptor>, CameraError> {
        let devices = nokhwa::query(ApiBackend::MediaFoundation)
            .map_err(|e| CameraError::EnumFailed(format!("{e}")))?;

        Ok(devices
            .into_iter()
            .map(|info| {
                let index = match info.index() {
                    CameraIndex::Index(index) => *index,
                    CameraIndex::String(index) => index.parse::<u32>().unwrap_or(0),
                };
                // The numeric index is needed to open the current device, but
                // the descriptor also carries backend identity fields so a
                // persisted preference is never just an index.
                let id = format!("msmf:{index}:{}:{}", info.description(), info.misc());
                CameraDescriptor {
                    id,
                    label: info.human_name(),
                }
            })
            .collect())
    }

    fn open(
        &self,
        descriptor: &CameraDescriptor,
        request: &CameraRequest,
    ) -> Result<Box<dyn CameraStream>, CameraError> {
        let index = parse_msmf_index(&descriptor.id)?;
        let cam_index = CameraIndex::Index(index as u32);

        // Create camera with a default format to query capabilities.
        let default_fmt = NokhwaFormat::new(Resolution::new(640, 480), FrameFormat::MJPEG, 30);
        let mut camera = Camera::new(
            cam_index,
            nokhwa::utils::RequestedFormat::with_formats(
                nokhwa::utils::RequestedFormatType::Closest(default_fmt),
                &[FrameFormat::MJPEG, FrameFormat::YUYV],
            ),
        )
        .map_err(map_nokhwa_error)?;

        // Enumerate available formats and pick the best match.
        let candidates = enumerate_format_candidates(&mut camera)?;
        let chosen = select_format(request, &candidates)?;

        // Apply the chosen format.
        let nokhwa_fmt = to_nokhwa_format(&chosen);
        #[allow(deprecated)]
        camera
            .set_camera_format(nokhwa_fmt)
            .map_err(map_nokhwa_error)?;

        camera.open_stream().map_err(map_nokhwa_error)?;

        let source_format = camera.frame_format();

        Ok(Box::new(MsmfStream {
            camera,
            format: chosen,
            seq: 0,
            source_format,
        }))
    }
}

/// Parse the MSMF index from a descriptor id like `"msmf:0:<identity>"`.
fn parse_msmf_index(id: &str) -> Result<usize, CameraError> {
    let idx_str = id
        .strip_prefix("msmf:")
        .and_then(|value| value.split(':').next())
        .ok_or_else(|| CameraError::OpenFailed(format!("not an MSMF descriptor id: {id}")))?;
    idx_str
        .parse::<usize>()
        .map_err(|_| CameraError::OpenFailed(format!("invalid MSMF index: {idx_str}")))
}

/// Enumerate format candidates from an open camera.
fn enumerate_format_candidates(camera: &mut Camera) -> Result<Vec<FormatCandidate>, CameraError> {
    let mut candidates = Vec::new();

    // Enumerate MJPEG formats.
    if let Ok(formats) = camera.compatible_list_by_resolution(FrameFormat::MJPEG) {
        for (resolution, fps_list) in &formats {
            for &fps in fps_list {
                candidates.push(FormatCandidate {
                    width: resolution.width(),
                    height: resolution.height(),
                    fps_numerator: fps,
                    fps_denominator: 1,
                    format: PixelFormat::Rgb8,
                });
            }
        }
    }

    // Enumerate YUYV formats.
    if let Ok(formats) = camera.compatible_list_by_resolution(FrameFormat::YUYV) {
        for (resolution, fps_list) in &formats {
            for &fps in fps_list {
                candidates.push(FormatCandidate {
                    width: resolution.width(),
                    height: resolution.height(),
                    fps_numerator: fps,
                    fps_denominator: 1,
                    format: PixelFormat::Bgr8,
                });
            }
        }
    }

    if candidates.is_empty() {
        return Err(CameraError::NoSuitableFormat);
    }

    Ok(candidates)
}

/// Convert our [`CameraFormat`] to a nokhwa [`NokhwaFormat`].
fn to_nokhwa_format(format: &CameraFormat) -> NokhwaFormat {
    let frame_format = match format.format {
        PixelFormat::Rgb8 => FrameFormat::MJPEG,
        PixelFormat::Bgr8 => FrameFormat::YUYV,
        _ => FrameFormat::MJPEG,
    };
    NokhwaFormat::new(
        Resolution::new(format.width, format.height),
        frame_format,
        format.fps_numerator / format.fps_denominator.max(1),
    )
}

/// Map a nokhwa error to our typed error.
fn map_nokhwa_error(e: nokhwa::NokhwaError) -> CameraError {
    let msg = format!("{e}");
    if msg.contains("permission") || msg.contains("access") || msg.contains("denied") {
        CameraError::PermissionDenied
    } else {
        CameraError::OpenFailed(msg)
    }
}

/// An opened MSMF camera stream.
///
/// The native camera is constructed, used, and dropped on the capture worker.
pub struct MsmfStream {
    camera: Camera,
    format: CameraFormat,
    seq: u64,
    source_format: FrameFormat,
}

impl CameraStream for MsmfStream {
    fn actual_format(&self) -> CameraFormat {
        self.format
    }

    fn next_frame(&mut self, stop: &StopToken) -> Result<VideoFrame, CameraError> {
        if stop.is_stopped() {
            return Err(CameraError::Disconnected);
        }

        let buffer = self.camera.frame().map_err(|e| {
            let msg = format!("{e}");
            if msg.contains("disconnect") || msg.contains("removed") {
                CameraError::Disconnected
            } else {
                CameraError::FrameDecodeFailed(msg)
            }
        })?;

        self.seq += 1;
        let now = vtuber_core::monotonic_now().0;

        let (data, pixel_format, stride) = decode_frame(&buffer, self.source_format, &self.format)?;

        Ok(VideoFrame {
            seq: FrameSeq(self.seq),
            captured_at: MonoTimeNs(now),
            width: self.format.width,
            height: self.format.height,
            stride_bytes: stride,
            format: pixel_format,
            data: data.into(),
        })
    }

    fn stop(&mut self) -> Result<(), CameraError> {
        self.camera.stop_stream().map_err(map_nokhwa_error)
    }
}

/// Decode a nokhwa buffer into raw pixel data.
fn decode_frame(
    buffer: &nokhwa::Buffer,
    source_format: FrameFormat,
    format: &CameraFormat,
) -> Result<(Vec<u8>, PixelFormat, usize), CameraError> {
    match source_format {
        FrameFormat::MJPEG => {
            let decoded = buffer
                .decode_image::<RgbFormat>()
                .map_err(|e| CameraError::FrameDecodeFailed(format!("MJPEG decode: {e}")))?;
            let rgb = decoded.into_raw();
            let stride = format.width as usize * 3;
            Ok((rgb, PixelFormat::Rgb8, stride))
        }
        FrameFormat::YUYV => {
            let yuyv = buffer.buffer();
            let rgb = yuyv_to_rgb(yuyv, format.width, format.height);
            let stride = format.width as usize * 3;
            Ok((rgb, PixelFormat::Rgb8, stride))
        }
        FrameFormat::RAWRGB => {
            let data = buffer.buffer().to_vec();
            let stride = format.width as usize * 3;
            Ok((data, PixelFormat::Rgb8, stride))
        }
        FrameFormat::RAWBGR => {
            let data = buffer.buffer().to_vec();
            let stride = format.width as usize * 3;
            Ok((data, PixelFormat::Bgr8, stride))
        }
        FrameFormat::GRAY => {
            let data = buffer.buffer().to_vec();
            let stride = format.width as usize;
            Ok((data, PixelFormat::Gray8, stride))
        }
        FrameFormat::NV12 => Err(CameraError::FrameDecodeFailed(
            "NV12 not yet supported".into(),
        )),
    }
}

/// Convert YUYV (YUY2) packed data to interleaved RGB.
fn yuyv_to_rgb(yuyv: &[u8], width: u32, height: u32) -> Vec<u8> {
    let pixel_count = (width * height) as usize;
    let mut rgb = vec![0u8; pixel_count * 3];

    for row in 0..height as usize {
        for col in (0..width as usize).step_by(2) {
            let src = (row * width as usize + col) * 2;
            if src + 3 >= yuyv.len() {
                break;
            }
            let y0 = yuyv[src] as f32;
            let u = yuyv[src + 1] as f32 - 128.0;
            let y1 = yuyv[src + 2] as f32;
            let v = yuyv[src + 3] as f32 - 128.0;

            let dst = (row * width as usize + col) * 3;
            yuv_to_rgb_pixel(y0, u, v, &mut rgb[dst..dst + 3]);
            if col + 1 < width as usize {
                yuv_to_rgb_pixel(y1, u, v, &mut rgb[dst + 3..dst + 6]);
            }
        }
    }
    rgb
}

fn yuv_to_rgb_pixel(y: f32, u: f32, v: f32, out: &mut [u8]) {
    out[0] = (y + 1.402 * v).clamp(0.0, 255.0) as u8;
    out[1] = (y - 0.344_136 * u - 0.714_136 * v).clamp(0.0, 255.0) as u8;
    out[2] = (y + 1.772 * u).clamp(0.0, 255.0) as u8;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_msmf_index_valid() {
        assert_eq!(parse_msmf_index("msmf:0").unwrap(), 0);
        assert_eq!(parse_msmf_index("msmf:3").unwrap(), 3);
    }

    #[test]
    fn parse_msmf_index_invalid_prefix() {
        assert!(parse_msmf_index("avf:0").is_err());
    }

    #[test]
    fn yuyv_to_rgb_produces_correct_size() {
        let yuyv = vec![128, 128, 128, 128];
        let rgb = yuyv_to_rgb(&yuyv, 2, 1);
        assert_eq!(rgb.len(), 6);
    }

    #[test]
    fn yuv_to_rgb_pixel_clamps() {
        let mut out = [0u8; 3];
        // Y=0, u=0, v=0 (centered chroma) → black.
        yuv_to_rgb_pixel(0.0, 0.0, 0.0, &mut out);
        assert_eq!(out, [0, 0, 0]);

        // Y=255 with extreme chroma should not panic and produces valid output.
        yuv_to_rgb_pixel(255.0, 127.0, 127.0, &mut out);
        // Just verify the function completed without panicking.
        let _ = out;
    }

    #[test]
    fn to_nokhwa_format_rgb_maps_to_mjpeg() {
        let format = CameraFormat {
            width: 1280,
            height: 720,
            fps_numerator: 30,
            fps_denominator: 1,
            format: PixelFormat::Rgb8,
        };
        let nf = to_nokhwa_format(&format);
        assert_eq!(nf.width(), 1280);
        assert_eq!(nf.height(), 720);
        assert_eq!(nf.format(), FrameFormat::MJPEG);
    }
}
