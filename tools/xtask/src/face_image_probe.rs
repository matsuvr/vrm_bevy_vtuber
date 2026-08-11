//! Still-image probe for the production UltraFace detector.

use std::path::Path;
use std::sync::Arc;

use image::ImageReader;
use vtuber_core::types::{FrameSeq, MonoTimeNs, PixelFormat, VideoFrame};
use vtuber_inference::detector::{
    DetectorDecodeOutcome, UltraFaceDetector, UltraFacePreprocessBuffers, decode_detections,
};

/// Runs the detector against one decoded still image.
pub fn run(args: &[String]) -> Result<(), String> {
    let Some(path) = args.first() else {
        print_help();
        return Err("face-image-probe requires an image path".into());
    };
    if args.len() > 1 {
        return Err(format!("unexpected argument `{}`", args[1]));
    }

    let path = Path::new(path);
    let project_root =
        std::env::current_dir().map_err(|error| format!("cannot resolve project root: {error}"))?;
    let manifest = project_root
        .join("assets")
        .join("models")
        .join("manifest.toml");
    let pipeline = vtuber_app::model_catalog::verify_pipeline_artifacts(&manifest)
        .map_err(|error| format!("model verification failed: {error}"))?;
    let artifact_root = project_root.join("assets").join("models");
    let detector_path = artifact_root.join(&pipeline.detector.file);
    let detector = UltraFaceDetector::from_path(&detector_path)
        .map_err(|error| format!("detector load failed: {error}"))?;

    let decoded = ImageReader::open(path)
        .map_err(|error| format!("image read failed for {}: {error}", path.display()))?
        .decode()
        .map_err(|error| format!("image decode failed for {}: {error}", path.display()))?
        .into_rgba8();
    let (width, height) = decoded.dimensions();
    let data = decoded.into_raw();
    let expected_len = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "decoded image dimensions overflow the RGBA8 buffer length".to_string())?;
    if data.len() != expected_len {
        return Err(format!(
            "decoded image pixel format is not RGBA8: width={width} height={height} bytes={}",
            data.len()
        ));
    }

    let frame = VideoFrame {
        seq: FrameSeq(1),
        captured_at: MonoTimeNs(1),
        width,
        height,
        stride_bytes: usize::try_from(width)
            .ok()
            .and_then(|width| width.checked_mul(4))
            .ok_or_else(|| "image stride overflows usize".to_string())?,
        format: PixelFormat::Rgba8,
        data: Arc::<[u8]>::from(data),
    };
    let mut buffers = UltraFacePreprocessBuffers::new();
    let raw = detector
        .infer(&mut buffers, &frame)
        .map_err(|error| format!("detector inference failed: {error}"))?;
    let outcome = decode_detections(&raw, pipeline.detector_postprocess)
        .map_err(|error| format!("detector decode failed: {error}"))?;

    println!("Face image probe");
    println!("  image: {}", path.display());
    println!("  dimensions: {width}x{height}");
    println!("  pipeline: {}", pipeline.id);
    match outcome {
        DetectorDecodeOutcome::NoFace => {
            println!("  result: NO_FACE");
        }
        DetectorDecodeOutcome::Detections(detections) => {
            println!("  result: FACE_DETECTED");
            println!("  detections: {}", detections.len());
            for (index, detection) in detections.iter().enumerate() {
                println!(
                    "  detection[{index}]: confidence={:.4} rect=({:.4},{:.4},{:.4},{:.4}) anchor={}",
                    detection.confidence,
                    detection.rect.x,
                    detection.rect.y,
                    detection.rect.width,
                    detection.rect.height,
                    detection.anchor_index,
                );
            }
        }
    }
    Ok(())
}

/// Prints command usage.
pub fn print_help() {
    println!("face-image-probe - run the production UltraFace detector on one image");
    println!();
    println!("USAGE:");
    println!("  cargo run -p xtask -- face-image-probe <image-path>");
}
