# convx Phase 8: ML Data Cleanup Pipeline

**Version:** 0.1.0  
**Purpose:** Transform convx from a converter into a data preprocessing powerhouse for ML workflows

---

## The Problem

> "Garbage in, garbage out"

ML teams spend **80% of their time** on data prep:
- Removing backgrounds from product images
- Upscaling low-res training data
- Denoising audio recordings
- Extracting text from images
- Normalizing inconsistent formats
- Cleaning up old/damaged media

**convx solves this** with a local-first, pipeline-able CLI.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      convx Pipeline                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Input → [Transform 1] → [Transform 2] → [...] → Output    │
│                                                             │
│  Transforms:                                                │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐           │
│  │  Convert    │ │  ML Plugin  │ │  ML Plugin  │           │
│  │  (built-in) │ │  (ONNX)     │ │  (ONNX)     │           │
│  └─────────────┘ └─────────────┘ └─────────────┘           │
│                                                             │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
              ┌───────────────────────────────┐
              │       ONNX Runtime            │
              │  (CPU / CUDA / CoreML / ROCm) │
              └───────────────────────────────┘
                              │
                              ▼
              ┌───────────────────────────────┐
              │       Model Registry          │
              │  ~/.convx/models/             │
              │  - realesrgan-x4.onnx         │
              │  - rmbg-1.4.onnx              │
              │  - whisper-base.onnx          │
              └───────────────────────────────┘
```

---

## ML Plugin System

### Plugin Trait

```rust
// src/plugins/mod.rs

use crate::ConvxError;
use std::path::Path;

/// Input/output types for plugins
#[derive(Debug, Clone)]
pub enum PluginData {
    /// Raw image bytes (decoded)
    Image {
        data: Vec<u8>,
        width: u32,
        height: u32,
        channels: u8,  // 3 = RGB, 4 = RGBA
    },
    /// Raw audio samples
    Audio {
        samples: Vec<f32>,
        sample_rate: u32,
        channels: u8,
    },
    /// Text output
    Text(String),
    /// Structured data (JSON)
    Json(serde_json::Value),
    /// Raw bytes (passthrough)
    Bytes(Vec<u8>),
}

/// Plugin capability flags
#[derive(Debug, Clone, Copy)]
pub struct PluginCapabilities {
    pub accepts_image: bool,
    pub accepts_audio: bool,
    pub accepts_video: bool,
    pub produces_image: bool,
    pub produces_audio: bool,
    pub produces_text: bool,
    pub gpu_accelerated: bool,
    pub batch_supported: bool,
}

/// Configuration for a plugin operation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Plugin-specific options (e.g., scale factor, threshold)
    pub options: HashMap<String, serde_json::Value>,
    
    /// Use GPU if available
    pub use_gpu: bool,
    
    /// Model variant to use
    pub model_variant: Option<String>,
}

/// Core plugin trait
pub trait MlPlugin: Send + Sync {
    /// Plugin identifier
    fn name(&self) -> &str;
    
    /// Human-readable description
    fn description(&self) -> &str;
    
    /// What this plugin can do
    fn capabilities(&self) -> PluginCapabilities;
    
    /// Required model files
    fn required_models(&self) -> Vec<ModelInfo>;
    
    /// Initialize the plugin (load models)
    fn init(&mut self, model_dir: &Path, use_gpu: bool) -> Result<(), ConvxError>;
    
    /// Process data
    fn process(&self, input: PluginData, config: &PluginConfig) -> Result<PluginData, ConvxError>;
    
    /// Process a batch (default: process one at a time)
    fn process_batch(&self, inputs: Vec<PluginData>, config: &PluginConfig) -> Result<Vec<PluginData>, ConvxError> {
        inputs.into_iter()
            .map(|input| self.process(input, config))
            .collect()
    }
    
    /// Estimate VRAM usage for a given input size
    fn estimate_vram(&self, width: u32, height: u32) -> u64 {
        0 // Default: unknown
    }
}

/// Model metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub filename: String,
    pub url: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub variant: Option<String>,  // e.g., "base", "large", "turbo"
}
```

### Model Manager

```rust
// src/plugins/model_manager.rs

use std::path::{Path, PathBuf};
use std::fs;
use std::io::Write;
use sha2::{Sha256, Digest};
use indicatif::{ProgressBar, ProgressStyle};

pub struct ModelManager {
    model_dir: PathBuf,
    registry: ModelRegistry,
}

impl ModelManager {
    pub fn new() -> Result<Self, ConvxError> {
        let model_dir = dirs::home_dir()
            .ok_or(ConvxError::ConfigError { reason: "Cannot find home directory".into() })?
            .join(".convx")
            .join("models");
        
        fs::create_dir_all(&model_dir)?;
        
        Ok(Self {
            model_dir,
            registry: ModelRegistry::default(),
        })
    }
    
    /// Check if a model is downloaded
    pub fn is_model_available(&self, model: &ModelInfo) -> bool {
        let path = self.model_dir.join(&model.filename);
        if !path.exists() {
            return false;
        }
        
        // Verify checksum
        self.verify_checksum(&path, &model.sha256).unwrap_or(false)
    }
    
    /// Download a model if not present
    pub async fn ensure_model(&self, model: &ModelInfo) -> Result<PathBuf, ConvxError> {
        let path = self.model_dir.join(&model.filename);
        
        if self.is_model_available(model) {
            return Ok(path);
        }
        
        println!("Downloading model: {} ({:.1} MB)", model.name, model.size_bytes as f64 / 1_000_000.0);
        
        self.download_model(model, &path).await?;
        
        // Verify
        if !self.verify_checksum(&path, &model.sha256)? {
            fs::remove_file(&path)?;
            return Err(ConvxError::ModelError {
                reason: "Checksum mismatch after download".into(),
            });
        }
        
        Ok(path)
    }
    
    async fn download_model(&self, model: &ModelInfo, path: &Path) -> Result<(), ConvxError> {
        let response = reqwest::get(&model.url).await
            .map_err(|e| ConvxError::NetworkError { reason: e.to_string() })?;
        
        let total_size = response.content_length().unwrap_or(model.size_bytes);
        
        let pb = ProgressBar::new(total_size);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#>-"));
        
        let mut file = fs::File::create(path)?;
        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();
        
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| ConvxError::NetworkError { reason: e.to_string() })?;
            file.write_all(&chunk)?;
            downloaded += chunk.len() as u64;
            pb.set_position(downloaded);
        }
        
        pb.finish_with_message("Download complete");
        Ok(())
    }
    
    fn verify_checksum(&self, path: &Path, expected: &str) -> Result<bool, ConvxError> {
        let mut file = fs::File::open(path)?;
        let mut hasher = Sha256::new();
        std::io::copy(&mut file, &mut hasher)?;
        let result = format!("{:x}", hasher.finalize());
        Ok(result == expected)
    }
    
    /// Get model path
    pub fn model_path(&self, filename: &str) -> PathBuf {
        self.model_dir.join(filename)
    }
    
    /// List downloaded models
    pub fn list_models(&self) -> Result<Vec<String>, ConvxError> {
        let mut models = Vec::new();
        for entry in fs::read_dir(&self.model_dir)? {
            let entry = entry?;
            if entry.path().extension().map(|e| e == "onnx").unwrap_or(false) {
                models.push(entry.file_name().to_string_lossy().to_string());
            }
        }
        Ok(models)
    }
    
    /// Delete a model
    pub fn delete_model(&self, filename: &str) -> Result<(), ConvxError> {
        let path = self.model_dir.join(filename);
        fs::remove_file(path)?;
        Ok(())
    }
    
    /// Total size of all models
    pub fn total_size(&self) -> Result<u64, ConvxError> {
        let mut total = 0;
        for entry in fs::read_dir(&self.model_dir)? {
            let entry = entry?;
            total += entry.metadata()?.len();
        }
        Ok(total)
    }
}
```

### ONNX Runtime Integration

```rust
// src/plugins/onnx_runtime.rs

use ort::{Environment, Session, SessionBuilder, Value, GraphOptimizationLevel};
use ndarray::{Array, Array4, ArrayView4};
use std::path::Path;

pub struct OnnxModel {
    session: Session,
    input_name: String,
    output_name: String,
}

impl OnnxModel {
    pub fn load(model_path: &Path, use_gpu: bool) -> Result<Self, ConvxError> {
        let environment = Environment::builder()
            .with_name("convx")
            .with_execution_providers([
                #[cfg(feature = "cuda")]
                ort::CUDAExecutionProvider::default().build(),
                #[cfg(target_os = "macos")]
                ort::CoreMLExecutionProvider::default().build(),
                ort::CPUExecutionProvider::default().build(),
            ])
            .build()?;
        
        let session = SessionBuilder::new(&environment)?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(num_cpus::get())?
            .with_model_from_file(model_path)?;
        
        let input_name = session.inputs[0].name.clone();
        let output_name = session.outputs[0].name.clone();
        
        Ok(Self {
            session,
            input_name,
            output_name,
        })
    }
    
    pub fn run_image(&self, input: ArrayView4<f32>) -> Result<Array4<f32>, ConvxError> {
        let input_tensor = Value::from_array(input)?;
        
        let outputs = self.session.run(vec![input_tensor])?;
        
        let output = outputs[0]
            .try_extract::<f32>()?
            .view()
            .to_owned()
            .into_dimensionality::<ndarray::Ix4>()?;
        
        Ok(output)
    }
}

/// Preprocess image for model input
pub fn preprocess_image(
    data: &[u8],
    width: u32,
    height: u32,
    channels: u8,
    normalize: bool,
) -> Array4<f32> {
    let mut array = Array4::<f32>::zeros((1, channels as usize, height as usize, width as usize));
    
    for y in 0..height as usize {
        for x in 0..width as usize {
            for c in 0..channels as usize {
                let idx = (y * width as usize + x) * channels as usize + c;
                let value = data[idx] as f32;
                array[[0, c, y, x]] = if normalize { value / 255.0 } else { value };
            }
        }
    }
    
    array
}

/// Postprocess model output to image
pub fn postprocess_image(
    output: Array4<f32>,
    denormalize: bool,
) -> (Vec<u8>, u32, u32, u8) {
    let shape = output.shape();
    let channels = shape[1] as u8;
    let height = shape[2] as u32;
    let width = shape[3] as u32;
    
    let mut data = vec![0u8; (width * height * channels as u32) as usize];
    
    for y in 0..height as usize {
        for x in 0..width as usize {
            for c in 0..channels as usize {
                let idx = (y * width as usize + x) * channels as usize + c;
                let value = output[[0, c, y, x]];
                let value = if denormalize { value * 255.0 } else { value };
                data[idx] = value.clamp(0.0, 255.0) as u8;
            }
        }
    }
    
    (data, width, height, channels)
}
```

---

## Built-in ML Plugins

### 1. Background Removal (RMBG)

```rust
// src/plugins/remove_bg.rs

use super::*;

pub struct RemoveBgPlugin {
    model: Option<OnnxModel>,
}

impl RemoveBgPlugin {
    pub fn new() -> Self {
        Self { model: None }
    }
}

impl MlPlugin for RemoveBgPlugin {
    fn name(&self) -> &str { "remove-bg" }
    
    fn description(&self) -> &str {
        "Remove background from images using RMBG-1.4"
    }
    
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            accepts_image: true,
            accepts_audio: false,
            accepts_video: false,
            produces_image: true,
            produces_audio: false,
            produces_text: false,
            gpu_accelerated: true,
            batch_supported: true,
        }
    }
    
    fn required_models(&self) -> Vec<ModelInfo> {
        vec![ModelInfo {
            name: "RMBG-1.4".into(),
            filename: "rmbg-1.4.onnx".into(),
            url: "https://huggingface.co/briaai/RMBG-1.4/resolve/main/onnx/model.onnx".into(),
            sha256: "abc123...".into(),
            size_bytes: 176_000_000,
            variant: None,
        }]
    }
    
    fn init(&mut self, model_dir: &Path, use_gpu: bool) -> Result<(), ConvxError> {
        let model_path = model_dir.join("rmbg-1.4.onnx");
        self.model = Some(OnnxModel::load(&model_path, use_gpu)?);
        Ok(())
    }
    
    fn process(&self, input: PluginData, config: &PluginConfig) -> Result<PluginData, ConvxError> {
        let PluginData::Image { data, width, height, channels } = input else {
            return Err(ConvxError::PluginError {
                plugin: self.name().into(),
                reason: "Expected image input".into(),
            });
        };
        
        let model = self.model.as_ref().ok_or(ConvxError::PluginError {
            plugin: self.name().into(),
            reason: "Model not initialized".into(),
        })?;
        
        // Preprocess: resize to 1024x1024, normalize
        let input_tensor = preprocess_image(&data, width, height, channels, true);
        
        // Run model
        let mask = model.run_image(input_tensor.view())?;
        
        // Apply mask to original image
        let (output_data, _, _, _) = apply_alpha_mask(&data, width, height, channels, &mask);
        
        Ok(PluginData::Image {
            data: output_data,
            width,
            height,
            channels: 4,  // Now has alpha
        })
    }
    
    fn estimate_vram(&self, width: u32, height: u32) -> u64 {
        // ~2GB for 1024x1024
        (width as u64 * height as u64 * 4 * 4) + 500_000_000
    }
}

fn apply_alpha_mask(
    rgb: &[u8],
    width: u32,
    height: u32,
    channels: u8,
    mask: &Array4<f32>,
) -> (Vec<u8>, u32, u32, u8) {
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    
    for y in 0..height as usize {
        for x in 0..width as usize {
            let src_idx = (y * width as usize + x) * channels as usize;
            let dst_idx = (y * width as usize + x) * 4;
            
            // Copy RGB
            rgba[dst_idx] = rgb[src_idx];
            rgba[dst_idx + 1] = rgb[src_idx + 1];
            rgba[dst_idx + 2] = rgb[src_idx + 2];
            
            // Set alpha from mask
            let alpha = (mask[[0, 0, y, x]] * 255.0).clamp(0.0, 255.0) as u8;
            rgba[dst_idx + 3] = alpha;
        }
    }
    
    (rgba, width, height, 4)
}
```

### 2. Image Upscaling (Real-ESRGAN)

```rust
// src/plugins/upscale.rs

pub struct UpscalePlugin {
    model_2x: Option<OnnxModel>,
    model_4x: Option<OnnxModel>,
}

impl MlPlugin for UpscalePlugin {
    fn name(&self) -> &str { "upscale" }
    
    fn description(&self) -> &str {
        "Upscale images 2x or 4x using Real-ESRGAN"
    }
    
    fn required_models(&self) -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                name: "Real-ESRGAN x2".into(),
                filename: "realesrgan-x2.onnx".into(),
                url: "https://github.com/xinntao/Real-ESRGAN/releases/download/v0.2.5.0/realesrgan-x2.onnx".into(),
                sha256: "...".into(),
                size_bytes: 64_000_000,
                variant: Some("2x".into()),
            },
            ModelInfo {
                name: "Real-ESRGAN x4".into(),
                filename: "realesrgan-x4.onnx".into(),
                url: "https://github.com/xinntao/Real-ESRGAN/releases/download/v0.2.5.0/realesrgan-x4.onnx".into(),
                sha256: "...".into(),
                size_bytes: 64_000_000,
                variant: Some("4x".into()),
            },
        ]
    }
    
    fn process(&self, input: PluginData, config: &PluginConfig) -> Result<PluginData, ConvxError> {
        let scale = config.options
            .get("scale")
            .and_then(|v| v.as_u64())
            .unwrap_or(2) as u32;
        
        let model = match scale {
            2 => self.model_2x.as_ref(),
            4 => self.model_4x.as_ref(),
            _ => return Err(ConvxError::PluginError {
                plugin: self.name().into(),
                reason: format!("Unsupported scale: {}x. Use 2x or 4x.", scale),
            }),
        };
        
        let model = model.ok_or(ConvxError::PluginError {
            plugin: self.name().into(),
            reason: format!("{}x model not loaded", scale),
        })?;
        
        let PluginData::Image { data, width, height, channels } = input else {
            return Err(ConvxError::PluginError {
                plugin: self.name().into(),
                reason: "Expected image input".into(),
            });
        };
        
        // Process in tiles for large images
        let output = if width > 512 || height > 512 {
            self.process_tiled(&data, width, height, channels, model, scale)?
        } else {
            self.process_whole(&data, width, height, channels, model)?
        };
        
        Ok(PluginData::Image {
            data: output,
            width: width * scale,
            height: height * scale,
            channels,
        })
    }
}
```

### 3. Image Denoising (NAFNet)

```rust
// src/plugins/denoise_image.rs

pub struct DenoiseImagePlugin {
    model: Option<OnnxModel>,
}

impl MlPlugin for DenoiseImagePlugin {
    fn name(&self) -> &str { "denoise-image" }
    
    fn description(&self) -> &str {
        "Remove noise from images using NAFNet"
    }
    
    fn required_models(&self) -> Vec<ModelInfo> {
        vec![ModelInfo {
            name: "NAFNet Denoise".into(),
            filename: "nafnet-denoise.onnx".into(),
            url: "https://huggingface.co/datasets/convx/models/resolve/main/nafnet-denoise.onnx".into(),
            sha256: "...".into(),
            size_bytes: 67_000_000,
            variant: None,
        }]
    }
    
    fn process(&self, input: PluginData, config: &PluginConfig) -> Result<PluginData, ConvxError> {
        let strength = config.options
            .get("strength")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;
        
        // ... denoise processing
        
        Ok(input)  // Placeholder
    }
}
```

### 4. Face Restoration (CodeFormer)

```rust
// src/plugins/restore_faces.rs

pub struct RestoreFacesPlugin {
    detector: Option<OnnxModel>,  // Face detection
    restorer: Option<OnnxModel>,  // CodeFormer
}

impl MlPlugin for RestoreFacesPlugin {
    fn name(&self) -> &str { "restore-faces" }
    
    fn description(&self) -> &str {
        "Restore and enhance faces using CodeFormer"
    }
    
    fn required_models(&self) -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                name: "RetinaFace".into(),
                filename: "retinaface.onnx".into(),
                url: "...".into(),
                sha256: "...".into(),
                size_bytes: 2_000_000,
                variant: None,
            },
            ModelInfo {
                name: "CodeFormer".into(),
                filename: "codeformer.onnx".into(),
                url: "...".into(),
                sha256: "...".into(),
                size_bytes: 400_000_000,
                variant: None,
            },
        ]
    }
    
    fn process(&self, input: PluginData, config: &PluginConfig) -> Result<PluginData, ConvxError> {
        let fidelity = config.options
            .get("fidelity")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5) as f32;
        
        // 1. Detect faces
        // 2. Crop each face
        // 3. Run CodeFormer on each face
        // 4. Blend back into original
        
        Ok(input)  // Placeholder
    }
}
```

### 5. Audio Denoising (Demucs/RNNoise)

```rust
// src/plugins/denoise_audio.rs

pub struct DenoiseAudioPlugin {
    model: Option<OnnxModel>,
}

impl MlPlugin for DenoiseAudioPlugin {
    fn name(&self) -> &str { "denoise-audio" }
    
    fn description(&self) -> &str {
        "Remove background noise from audio"
    }
    
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            accepts_image: false,
            accepts_audio: true,
            accepts_video: false,
            produces_image: false,
            produces_audio: true,
            produces_text: false,
            gpu_accelerated: true,
            batch_supported: false,
        }
    }
    
    fn required_models(&self) -> Vec<ModelInfo> {
        vec![ModelInfo {
            name: "RNNoise".into(),
            filename: "rnnoise.onnx".into(),
            url: "...".into(),
            sha256: "...".into(),
            size_bytes: 2_000_000,
            variant: None,
        }]
    }
    
    fn process(&self, input: PluginData, config: &PluginConfig) -> Result<PluginData, ConvxError> {
        let PluginData::Audio { samples, sample_rate, channels } = input else {
            return Err(ConvxError::PluginError {
                plugin: self.name().into(),
                reason: "Expected audio input".into(),
            });
        };
        
        // Process in chunks
        let chunk_size = 480;  // 10ms at 48kHz
        let mut output = Vec::with_capacity(samples.len());
        
        for chunk in samples.chunks(chunk_size) {
            let denoised = self.process_chunk(chunk)?;
            output.extend(denoised);
        }
        
        Ok(PluginData::Audio {
            samples: output,
            sample_rate,
            channels,
        })
    }
}
```

### 6. Speech Transcription (Whisper)

```rust
// src/plugins/transcribe.rs

pub struct TranscribePlugin {
    model: Option<OnnxModel>,
}

impl MlPlugin for TranscribePlugin {
    fn name(&self) -> &str { "transcribe" }
    
    fn description(&self) -> &str {
        "Transcribe speech to text using Whisper"
    }
    
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            accepts_image: false,
            accepts_audio: true,
            accepts_video: false,
            produces_image: false,
            produces_audio: false,
            produces_text: true,
            gpu_accelerated: true,
            batch_supported: false,
        }
    }
    
    fn required_models(&self) -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                name: "Whisper Base".into(),
                filename: "whisper-base.onnx".into(),
                url: "...".into(),
                sha256: "...".into(),
                size_bytes: 140_000_000,
                variant: Some("base".into()),
            },
            ModelInfo {
                name: "Whisper Small".into(),
                filename: "whisper-small.onnx".into(),
                url: "...".into(),
                sha256: "...".into(),
                size_bytes: 460_000_000,
                variant: Some("small".into()),
            },
        ]
    }
    
    fn process(&self, input: PluginData, config: &PluginConfig) -> Result<PluginData, ConvxError> {
        let language = config.options
            .get("language")
            .and_then(|v| v.as_str())
            .unwrap_or("en");
        
        let timestamps = config.options
            .get("timestamps")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        // ... transcription processing
        
        Ok(PluginData::Text("transcribed text".into()))
    }
}
```

### 7. OCR (PaddleOCR)

```rust
// src/plugins/ocr.rs

pub struct OcrPlugin {
    detector: Option<OnnxModel>,
    recognizer: Option<OnnxModel>,
}

impl MlPlugin for OcrPlugin {
    fn name(&self) -> &str { "ocr" }
    
    fn description(&self) -> &str {
        "Extract text from images using PaddleOCR"
    }
    
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            accepts_image: true,
            accepts_audio: false,
            accepts_video: false,
            produces_image: false,
            produces_audio: false,
            produces_text: true,
            gpu_accelerated: true,
            batch_supported: true,
        }
    }
    
    fn required_models(&self) -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                name: "PaddleOCR Detector".into(),
                filename: "paddleocr-det.onnx".into(),
                url: "...".into(),
                sha256: "...".into(),
                size_bytes: 3_000_000,
                variant: None,
            },
            ModelInfo {
                name: "PaddleOCR Recognizer".into(),
                filename: "paddleocr-rec.onnx".into(),
                url: "...".into(),
                sha256: "...".into(),
                size_bytes: 10_000_000,
                variant: None,
            },
        ]
    }
    
    fn process(&self, input: PluginData, config: &PluginConfig) -> Result<PluginData, ConvxError> {
        let output_format = config.options
            .get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("text");
        
        // 1. Detect text regions
        // 2. Recognize text in each region
        // 3. Format output
        
        match output_format {
            "json" => Ok(PluginData::Json(serde_json::json!({
                "text": "extracted text",
                "boxes": [[10, 10, 100, 30]],
                "confidence": [0.95]
            }))),
            _ => Ok(PluginData::Text("extracted text".into()))
        }
    }
}
```

---

## CLI Interface

### New Commands and Flags

```bash
# Model management
convx models list                    # List downloaded models
convx models download remove-bg      # Download a specific model
convx models download all            # Download all models
convx models delete remove-bg        # Delete a model
convx models info                    # Show total size, VRAM requirements

# Image ML operations
convx convert image.jpg --to png --remove-bg
convx convert image.jpg --to png --upscale 2x
convx convert image.jpg --to png --upscale 4x --model real-esrgan
convx convert image.jpg --to png --denoise
convx convert image.jpg --to png --restore-faces
convx convert image.jpg --to png --restore-faces --fidelity 0.7

# Chain operations (processed in order)
convx convert old_photo.jpg --to png \
    --denoise \
    --upscale 2x \
    --restore-faces

# Audio ML operations  
convx convert audio.wav --to mp3 --denoise
convx convert interview.wav --to wav --denoise --normalize

# Transcription
convx transcribe audio.mp3                      # Output: audio.txt
convx transcribe audio.mp3 --to srt             # With timestamps
convx transcribe audio.mp3 --to json            # Structured output
convx transcribe audio.mp3 --language es        # Spanish
convx transcribe video.mp4 --to srt             # Extract & transcribe

# OCR
convx ocr document.jpg                          # Output: document.txt
convx ocr document.jpg --to json                # With bounding boxes
convx ocr receipt.png --to json                 # Structured extraction

# Batch with ML
convx batch ./photos --remove-bg --to png --output ./cutouts
convx batch ./dataset --upscale 2x --to png --output ./upscaled --jobs 4
convx batch ./audio --denoise --to wav --output ./clean

# GPU control
convx convert image.jpg --to png --remove-bg --gpu         # Force GPU
convx convert image.jpg --to png --remove-bg --no-gpu      # Force CPU
convx convert image.jpg --to png --remove-bg --gpu cuda    # Specific GPU
convx convert image.jpg --to png --remove-bg --gpu mps     # Apple Silicon
```

### CLI Argument Additions

```rust
// src/main.rs additions

#[derive(Subcommand)]
enum Commands {
    Convert {
        // ... existing args ...
        
        /// Remove background
        #[arg(long)]
        remove_bg: bool,
        
        /// Upscale (2x or 4x)
        #[arg(long)]
        upscale: Option<String>,
        
        /// Denoise image
        #[arg(long)]
        denoise: bool,
        
        /// Restore faces
        #[arg(long)]
        restore_faces: bool,
        
        /// Face restoration fidelity (0.0-1.0)
        #[arg(long, default_value = "0.5")]
        fidelity: f32,
        
        /// Force GPU usage
        #[arg(long)]
        gpu: bool,
        
        /// Disable GPU
        #[arg(long)]
        no_gpu: bool,
    },
    
    /// Transcribe audio/video to text
    Transcribe {
        /// Input file
        input: PathBuf,
        
        /// Output format (txt, srt, vtt, json)
        #[arg(short, long, default_value = "txt")]
        to: String,
        
        /// Language code
        #[arg(short, long, default_value = "en")]
        language: String,
        
        /// Model size (base, small, medium, large)
        #[arg(long, default_value = "base")]
        model: String,
    },
    
    /// Extract text from images
    Ocr {
        /// Input file
        input: PathBuf,
        
        /// Output format (txt, json)
        #[arg(short, long, default_value = "txt")]
        to: String,
    },
    
    /// Manage ML models
    Models {
        #[command(subcommand)]
        command: ModelCommands,
    },
}

#[derive(Subcommand)]
enum ModelCommands {
    /// List downloaded models
    List,
    
    /// Download a model
    Download {
        /// Model name (remove-bg, upscale, denoise, whisper, ocr, all)
        model: String,
    },
    
    /// Delete a model
    Delete {
        /// Model name
        model: String,
    },
    
    /// Show model info
    Info,
}
```

---

## Pipeline Processor

```rust
// src/pipeline.rs

pub struct Pipeline {
    steps: Vec<PipelineStep>,
    engine: Arc<ConvxEngine>,
    plugin_manager: Arc<PluginManager>,
}

pub enum PipelineStep {
    Convert { output_format: Format, options: ConversionOptions },
    Plugin { name: String, config: PluginConfig },
}

impl Pipeline {
    pub fn new(engine: Arc<ConvxEngine>, plugin_manager: Arc<PluginManager>) -> Self {
        Self {
            steps: Vec::new(),
            engine,
            plugin_manager,
        }
    }
    
    pub fn add_convert(mut self, format: Format, options: ConversionOptions) -> Self {
        self.steps.push(PipelineStep::Convert {
            output_format: format,
            options,
        });
        self
    }
    
    pub fn add_plugin(mut self, name: &str, config: PluginConfig) -> Self {
        self.steps.push(PipelineStep::Plugin {
            name: name.to_string(),
            config,
        });
        self
    }
    
    pub fn execute(&self, input: &Path, output: &Path) -> Result<PipelineResult, ConvxError> {
        let mut current_data = self.load_input(input)?;
        let start = std::time::Instant::now();
        
        for (i, step) in self.steps.iter().enumerate() {
            println!("  Step {}/{}: {:?}", i + 1, self.steps.len(), step);
            
            current_data = match step {
                PipelineStep::Convert { output_format, options } => {
                    self.execute_convert(current_data, *output_format, options)?
                }
                PipelineStep::Plugin { name, config } => {
                    self.execute_plugin(current_data, name, config)?
                }
            };
        }
        
        self.save_output(current_data, output)?;
        
        Ok(PipelineResult {
            duration_ms: start.elapsed().as_millis() as u64,
            steps_completed: self.steps.len(),
        })
    }
    
    fn execute_plugin(
        &self,
        data: PluginData,
        name: &str,
        config: &PluginConfig,
    ) -> Result<PluginData, ConvxError> {
        let plugin = self.plugin_manager.get_plugin(name)?;
        plugin.process(data, config)
    }
}

// Builder pattern for CLI
pub fn build_pipeline_from_args(args: &ConvertArgs) -> Pipeline {
    let mut pipeline = Pipeline::new(engine, plugins);
    
    // ML operations first (order matters!)
    if args.denoise {
        pipeline = pipeline.add_plugin("denoise-image", PluginConfig::default());
    }
    
    if let Some(scale) = &args.upscale {
        let scale_factor: u64 = scale.trim_end_matches('x').parse().unwrap_or(2);
        pipeline = pipeline.add_plugin("upscale", PluginConfig {
            options: [("scale".into(), serde_json::json!(scale_factor))].into(),
            ..Default::default()
        });
    }
    
    if args.restore_faces {
        pipeline = pipeline.add_plugin("restore-faces", PluginConfig {
            options: [("fidelity".into(), serde_json::json!(args.fidelity))].into(),
            ..Default::default()
        });
    }
    
    if args.remove_bg {
        pipeline = pipeline.add_plugin("remove-bg", PluginConfig::default());
    }
    
    // Format conversion last
    pipeline = pipeline.add_convert(args.output_format, args.conversion_options());
    
    pipeline
}
```

---

## Cargo.toml Additions

```toml
[dependencies]
# ... existing deps ...

# ONNX Runtime
ort = { version = "2", features = ["cuda", "coreml"] }

# Numerical processing
ndarray = "0.15"
image = "0.25"

# Audio processing
rubato = "0.14"  # Resampling
hound = "3"      # WAV reading

# Async downloads
reqwest = { version = "0.11", features = ["stream"] }
tokio-stream = "0.1"

# Checksums
sha2 = "0.10"

# System info
num_cpus = "1"
dirs = "5"

[features]
default = ["ml"]
ml = ["ort", "ndarray"]
cuda = ["ort/cuda"]
coreml = ["ort/coreml"]
```

---

## Tests for ML Plugins

```rust
// tests/ml_integration.rs

#[test]
fn test_remove_bg() {
    let engine = ConvxEngine::new().unwrap();
    let plugins = PluginManager::new().unwrap();
    
    // Ensure model is downloaded
    let model_manager = ModelManager::new().unwrap();
    tokio_test::block_on(async {
        model_manager.ensure_model(&plugins.get_plugin("remove-bg").unwrap().required_models()[0]).await.unwrap();
    });
    
    let input = Path::new("tests/fixtures/person.jpg");
    let output = tempfile::NamedTempFile::new().unwrap();
    
    let pipeline = Pipeline::new(Arc::new(engine), Arc::new(plugins))
        .add_plugin("remove-bg", PluginConfig::default())
        .add_convert(Format::Png, ConversionOptions::default());
    
    let result = pipeline.execute(input, output.path()).unwrap();
    
    assert!(output.path().exists());
    
    // Verify output has alpha channel
    let img = image::open(output.path()).unwrap();
    assert!(img.color().has_alpha());
}

#[test]
fn test_upscale_2x() {
    let engine = ConvxEngine::new().unwrap();
    let plugins = PluginManager::new().unwrap();
    
    let input = Path::new("tests/fixtures/small.jpg");  // 100x100
    let output = tempfile::NamedTempFile::new().unwrap();
    
    let pipeline = Pipeline::new(Arc::new(engine), Arc::new(plugins))
        .add_plugin("upscale", PluginConfig {
            options: [("scale".into(), serde_json::json!(2))].into(),
            ..Default::default()
        })
        .add_convert(Format::Png, ConversionOptions::default());
    
    pipeline.execute(input, output.path()).unwrap();
    
    // Verify output is 2x size
    let img = image::open(output.path()).unwrap();
    assert_eq!(img.width(), 200);
    assert_eq!(img.height(), 200);
}

#[test]
fn test_transcribe() {
    let plugins = PluginManager::new().unwrap();
    
    let input = Path::new("tests/fixtures/speech.wav");
    let output = tempfile::NamedTempFile::new().unwrap();
    
    let plugin = plugins.get_plugin("transcribe").unwrap();
    
    let audio = load_audio(input).unwrap();
    let result = plugin.process(audio, &PluginConfig::default()).unwrap();
    
    if let PluginData::Text(text) = result {
        assert!(!text.is_empty());
    } else {
        panic!("Expected text output");
    }
}

#[test]
fn test_pipeline_chain() {
    // Test: denoise -> upscale -> remove-bg -> convert
    let engine = ConvxEngine::new().unwrap();
    let plugins = PluginManager::new().unwrap();
    
    let input = Path::new("tests/fixtures/noisy_small_person.jpg");
    let output = tempfile::NamedTempFile::new().unwrap();
    
    let pipeline = Pipeline::new(Arc::new(engine), Arc::new(plugins))
        .add_plugin("denoise-image", PluginConfig::default())
        .add_plugin("upscale", PluginConfig {
            options: [("scale".into(), serde_json::json!(2))].into(),
            ..Default::default()
        })
        .add_plugin("remove-bg", PluginConfig::default())
        .add_convert(Format::Png, ConversionOptions::default());
    
    let result = pipeline.execute(input, output.path()).unwrap();
    
    assert_eq!(result.steps_completed, 4);
    assert!(output.path().exists());
}
```

---

## Model Registry

```rust
// src/plugins/registry.rs

pub static MODEL_REGISTRY: &[ModelInfo] = &[
    // Background removal
    ModelInfo {
        name: "RMBG-1.4",
        filename: "rmbg-1.4.onnx",
        url: "https://huggingface.co/briaai/RMBG-1.4/resolve/main/onnx/model.onnx",
        sha256: "...",
        size_bytes: 176_000_000,
        variant: None,
    },
    
    // Upscaling
    ModelInfo {
        name: "Real-ESRGAN x2",
        filename: "realesrgan-x2.onnx",
        url: "...",
        sha256: "...",
        size_bytes: 64_000_000,
        variant: Some("2x"),
    },
    ModelInfo {
        name: "Real-ESRGAN x4",
        filename: "realesrgan-x4.onnx",
        url: "...",
        sha256: "...",
        size_bytes: 64_000_000,
        variant: Some("4x"),
    },
    
    // Whisper
    ModelInfo {
        name: "Whisper Base",
        filename: "whisper-base.onnx",
        url: "...",
        sha256: "...",
        size_bytes: 140_000_000,
        variant: Some("base"),
    },
    ModelInfo {
        name: "Whisper Small",
        filename: "whisper-small.onnx",
        url: "...",
        sha256: "...",
        size_bytes: 460_000_000,
        variant: Some("small"),
    },
    
    // ... more models
];
```

---

## Summary

**Phase 8 adds:**

| Feature | CLI Flag | Model Size |
|---------|----------|------------|
| Background removal | `--remove-bg` | 176 MB |
| Upscale 2x | `--upscale 2x` | 64 MB |
| Upscale 4x | `--upscale 4x` | 64 MB |
| Denoise image | `--denoise` | 67 MB |
| Restore faces | `--restore-faces` | 400 MB |
| Denoise audio | `--denoise` (audio) | 2 MB |
| Transcribe | `convx transcribe` | 140-460 MB |
| OCR | `convx ocr` | 13 MB |

**Total model storage:** ~1 GB if all downloaded

**New commands:**
- `convx models list/download/delete/info`
- `convx transcribe`
- `convx ocr`

**Pipeline chaining:**
```bash
convx convert photo.jpg --to png --denoise --upscale 2x --remove-bg
```

---

## Definition of Done (Phase 8)

- [ ] `convx models list` works
- [ ] `convx models download remove-bg` downloads model
- [ ] `convx convert image.jpg --to png --remove-bg` removes background
- [ ] `convx convert image.jpg --to png --upscale 2x` upscales
- [ ] `convx transcribe audio.mp3` produces transcript
- [ ] Pipeline chaining works: `--denoise --upscale 2x --remove-bg`
- [ ] GPU acceleration works on CUDA/CoreML
- [ ] All tests pass
