// SPDX-License-Identifier: MIT OR Apache-2.0
//! Thumbnail generation and caching system for the asset browser.
//!
//! Provides asynchronous thumbnail generation for various asset types with
//! in-memory and disk caching support.


use egui_wgpu::wgpu;
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::mpsc;

/// Default thumbnail size in pixels
pub const DEFAULT_THUMBNAIL_SIZE: u32 = 128;

/// Maximum number of cached thumbnails in memory
pub const MAX_CACHED_THUMBNAILS: usize = 500;

/// Thumbnail state for an asset
#[allow(dead_code)] // Intentionally kept for API completeness
#[derive(Debug, Clone)]
pub enum ThumbnailState {
    /// Not yet requested
    NotLoaded,
    /// Currently being generated
    Loading,
    /// Successfully generated with texture ID
    Ready(egui::TextureId),
    /// Failed to generate
    Failed(String),
    /// Uses default icon (no preview possible)
    UseDefault,
}

/// Thumbnail request for async generation
#[derive(Debug, Clone)]
pub struct ThumbnailRequest {
    /// Path to the asset
    pub path: PathBuf,
    /// Requested size
    pub size: u32,
    /// Cache directory for disk persistence (if enabled)
    pub cache_dir: Option<PathBuf>,
}

/// Generated thumbnail data ready for GPU upload
#[derive(Debug)]
pub struct ThumbnailData {
    /// Path to the asset
    pub path: PathBuf,
    /// RGBA pixel data
    pub pixels: Vec<u8>,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
}

/// Result of thumbnail generation
pub type ThumbnailResult = Result<ThumbnailData, ThumbnailError>;

/// Errors that can occur during thumbnail generation
#[derive(Debug, Clone, thiserror::Error)]
pub enum ThumbnailError {
    /// File not found
    #[error("File not found: {0}")]
    NotFound(String),
    /// Unsupported format
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
    /// Image decoding error
    #[error("Failed to decode image: {0}")]
    DecodeError(String),
    /// IO error
    #[error("IO error: {0}")]
    IoError(String),
}

/// Metadata for disk-cached thumbnails
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiskCacheMetadata {
    /// Original file path
    source_path: PathBuf,
    /// Modification time of source file when thumbnail was generated
    source_modified: u64,
    /// Thumbnail dimensions
    width: u32,
    height: u32,
    /// Thumbnail size setting used
    thumbnail_size: u32,
}

/// Disk cache for thumbnails
#[allow(dead_code)] // Intentionally kept for API completeness
pub struct DiskCache {
    /// Cache directory
    cache_dir: PathBuf,
}

#[allow(dead_code)] // Intentionally kept for API completeness
impl DiskCache {
    /// Create a new disk cache at the given directory
    pub fn new(cache_dir: impl Into<PathBuf>) -> std::io::Result<Self> {
        let cache_dir = cache_dir.into();
        fs::create_dir_all(&cache_dir)?;
        Ok(Self { cache_dir })
    }

    /// Compute cache path for a source file
    fn cache_path(&self, source_path: &Path, suffix: &str) -> PathBuf {
        // Use a hash of the absolute path for the cache filename
        let path_str = source_path.to_string_lossy();
        let hash = Self::hash_string(&path_str);
        self.cache_dir.join(format!("{:016x}{}", hash, suffix))
    }

    /// Simple string hash (FNV-1a)
    fn hash_string(s: &str) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for byte in s.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }

    /// Get modification time of a file as u64 (seconds since UNIX epoch)
    fn get_modified_time(path: &Path) -> Option<u64> {
        fs::metadata(path)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
    }

    /// Check if cached thumbnail is valid (exists and source hasn't changed)
    pub fn is_valid(&self, source_path: &Path, thumbnail_size: u32) -> bool {
        let meta_path = self.cache_path(source_path, ".meta");
        let data_path = self.cache_path(source_path, ".thumb");

        // Both files must exist
        if !meta_path.exists() || !data_path.exists() {
            return false;
        }

        // Read and validate metadata
        let Ok(meta_data) = fs::read(&meta_path) else {
            return false;
        };

        let Ok(metadata): Result<DiskCacheMetadata, _> = serde_json::from_slice(&meta_data) else {
            return false;
        };

        // Check thumbnail size matches
        if metadata.thumbnail_size != thumbnail_size {
            return false;
        }

        // Check source modification time
        let Some(current_mtime) = Self::get_modified_time(source_path) else {
            return false;
        };

        metadata.source_modified == current_mtime
    }

    /// Load cached thumbnail data
    pub fn load(&self, source_path: &Path) -> Option<ThumbnailData> {
        let meta_path = self.cache_path(source_path, ".meta");
        let data_path = self.cache_path(source_path, ".thumb");

        // Read metadata
        let meta_data = fs::read(&meta_path).ok()?;
        let metadata: DiskCacheMetadata = serde_json::from_slice(&meta_data).ok()?;

        // Read pixel data
        let mut file = fs::File::open(&data_path).ok()?;
        let mut pixels = Vec::new();
        file.read_to_end(&mut pixels).ok()?;

        Some(ThumbnailData {
            path: source_path.to_path_buf(),
            pixels,
            width: metadata.width,
            height: metadata.height,
        })
    }

    /// Save thumbnail to disk cache
    pub fn save(&self, data: &ThumbnailData, thumbnail_size: u32) -> std::io::Result<()> {
        let meta_path = self.cache_path(&data.path, ".meta");
        let data_path = self.cache_path(&data.path, ".thumb");

        // Get source modification time
        let source_modified = Self::get_modified_time(&data.path).unwrap_or(0);

        // Create metadata
        let metadata = DiskCacheMetadata {
            source_path: data.path.clone(),
            source_modified,
            width: data.width,
            height: data.height,
            thumbnail_size,
        };

        // Write metadata
        let meta_json = serde_json::to_vec(&metadata)
            .map_err(std::io::Error::other)?;
        fs::write(&meta_path, &meta_json)?;

        // Write pixel data
        let mut file = fs::File::create(&data_path)?;
        file.write_all(&data.pixels)?;

        Ok(())
    }

    /// Clear all cached thumbnails
    pub fn clear(&self) -> std::io::Result<()> {
        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "meta" || e == "thumb").unwrap_or(false) {
                let _ = fs::remove_file(path);
            }
        }
        Ok(())
    }

    /// Get cache statistics (file count, total size in bytes)
    pub fn stats(&self) -> (usize, u64) {
        let mut count = 0usize;
        let mut total_size = 0u64;

        if let Ok(entries) = fs::read_dir(&self.cache_dir) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    count += 1;
                    total_size += meta.len();
                }
            }
        }

        (count, total_size)
    }
}

/// Thumbnail cache entry
struct CacheEntry {
    /// Texture ID for egui rendering
    texture_id: egui::TextureId,
    /// Last access time for LRU eviction
    last_access: std::time::Instant,
    /// Size in bytes (for memory tracking)
    size_bytes: usize,
}

/// Thumbnail manager handles generation and caching of asset thumbnails
#[allow(dead_code)] // Intentionally kept for API completeness
pub struct ThumbnailManager {
    /// In-memory texture cache
    cache: Arc<RwLock<HashMap<PathBuf, CacheEntry>>>,
    /// Pending thumbnail states
    states: Arc<RwLock<HashMap<PathBuf, ThumbnailState>>>,
    /// Channel for sending thumbnail requests
    request_tx: mpsc::UnboundedSender<ThumbnailRequest>,
    /// Channel for receiving generated thumbnails
    result_rx: mpsc::UnboundedReceiver<ThumbnailResult>,
    /// Default thumbnail size
    pub thumbnail_size: u32,
    /// Cache directory for disk persistence
    cache_dir: Option<PathBuf>,
    /// Disk cache instance
    disk_cache: Option<DiskCache>,
    /// Total cached memory in bytes
    cached_bytes: Arc<RwLock<usize>>,
}

impl ThumbnailManager {
    /// Create a new thumbnail manager
    pub fn new() -> Self {
        let (request_tx, request_rx) = mpsc::unbounded_channel();
        let (result_tx, result_rx) = mpsc::unbounded_channel();

        // Spawn the thumbnail generation worker
        std::thread::spawn(move || {
            thumbnail_worker(request_rx, result_tx);
        });

        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            states: Arc::new(RwLock::new(HashMap::new())),
            request_tx,
            result_rx,
            thumbnail_size: DEFAULT_THUMBNAIL_SIZE,
            cache_dir: None,
            disk_cache: None,
            cached_bytes: Arc::new(RwLock::new(0)),
        }
    }

    /// Set the cache directory for disk persistence
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub fn with_cache_dir(mut self, path: impl Into<PathBuf>) -> Self {
        let cache_dir = path.into();
        self.cache_dir = Some(cache_dir.clone());
        // Create disk cache
        match DiskCache::new(&cache_dir) {
            Ok(dc) => {
                self.disk_cache = Some(dc);
                tracing::info!("Thumbnail disk cache enabled at: {:?}", cache_dir);
            }
            Err(e) => {
                tracing::warn!("Failed to create thumbnail disk cache: {}", e);
            }
        }
        self
    }

    /// Get disk cache statistics
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub fn disk_cache_stats(&self) -> Option<(usize, u64)> {
        self.disk_cache.as_ref().map(DiskCache::stats)
    }

    /// Clear disk cache
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub fn clear_disk_cache(&self) -> std::io::Result<()> {
        if let Some(dc) = &self.disk_cache {
            dc.clear()?;
        }
        Ok(())
    }

    /// Request a thumbnail for an asset
    pub fn request_thumbnail(&self, path: &Path) {
        let mut states = self.states.write();

        // Skip if already loaded or loading
        if let Some(ThumbnailState::Ready(_) | ThumbnailState::Loading | ThumbnailState::UseDefault) = states.get(path) {
            return;
        }

        // Check if we can generate a thumbnail for this file type
        if !Self::can_generate_thumbnail(path) {
            states.insert(path.to_path_buf(), ThumbnailState::UseDefault);
            return;
        }

        // Mark as loading and send request
        states.insert(path.to_path_buf(), ThumbnailState::Loading);

        let _ = self.request_tx.send(ThumbnailRequest {
            path: path.to_path_buf(),
            size: self.thumbnail_size,
            cache_dir: self.cache_dir.clone(),
        });
    }

    /// Check if we can generate a thumbnail for this file type
    fn can_generate_thumbnail(path: &Path) -> bool {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_lowercase);

        matches!(
            ext.as_deref(),
            Some(
                "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "tga" | "hdr" | "exr" | "webp"
                    | "ppm" | "pgm" | "pbm" | "pam"
            )
        )
    }

    /// Get the thumbnail state for an asset
    pub fn get_state(&self, path: &Path) -> ThumbnailState {
        // Check cache first
        if let Some(entry) = self.cache.read().get(path) {
            return ThumbnailState::Ready(entry.texture_id);
        }

        // Check pending states
        self.states
            .read()
            .get(path)
            .cloned()
            .unwrap_or(ThumbnailState::NotLoaded)
    }

    /// Get the texture ID if available
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub fn get_texture(&self, path: &Path) -> Option<egui::TextureId> {
        let mut cache = self.cache.write();
        if let Some(entry) = cache.get_mut(path) {
            entry.last_access = std::time::Instant::now();
            return Some(entry.texture_id);
        }
        None
    }

    /// Process completed thumbnail generations and upload to GPU
    pub fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        egui_renderer: &mut egui_wgpu::Renderer,
    ) {
        // Process all available results
        while let Ok(result) = self.result_rx.try_recv() {
            match result {
                Ok(data) => {
                    // Create egui texture
                    let texture_id = self.upload_texture(device, queue, egui_renderer, &data);

                    // Add to cache
                    let size_bytes = data.pixels.len();
                    self.cache.write().insert(
                        data.path.clone(),
                        CacheEntry {
                            texture_id,
                            last_access: std::time::Instant::now(),
                            size_bytes,
                        },
                    );

                    *self.cached_bytes.write() += size_bytes;

                    // Update state
                    self.states
                        .write()
                        .insert(data.path, ThumbnailState::Ready(texture_id));

                    // Evict old entries if needed
                    self.evict_if_needed(egui_renderer);
                }
                Err(e) => {
                    if let ThumbnailError::UnsupportedFormat(_) = &e {
                        // Mark as using default icon
                        if let Some(path) = extract_path_from_error(&e) {
                            self.states
                                .write()
                                .insert(path, ThumbnailState::UseDefault);
                        }
                    } else {
                        tracing::warn!("Thumbnail generation failed: {}", e);
                    }
                }
            }
        }
    }

    /// Upload thumbnail data to GPU
    fn upload_texture(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        egui_renderer: &mut egui_wgpu::Renderer,
        data: &ThumbnailData,
    ) -> egui::TextureId {
        let image = egui::ColorImage::from_rgba_unmultiplied(
            [data.width as usize, data.height as usize],
            &data.pixels,
        );

        // Create a wgpu texture and register it with egui
        let texture_view = create_texture(device, queue, &image);
        egui_renderer.register_native_texture(device, &texture_view, wgpu::FilterMode::Linear)
    }

    /// Evict old cache entries if we're over the limit
    fn evict_if_needed(&mut self, egui_renderer: &mut egui_wgpu::Renderer) {
        let mut cache = self.cache.write();

        while cache.len() > MAX_CACHED_THUMBNAILS {
            // Find oldest entry
            let oldest = cache
                .iter()
                .min_by_key(|(_, entry)| entry.last_access)
                .map(|(path, _)| path.clone());

            if let Some(path) = oldest {
                if let Some(entry) = cache.remove(&path) {
                    egui_renderer.free_texture(&entry.texture_id);
                    *self.cached_bytes.write() -= entry.size_bytes;
                    self.states.write().remove(&path);
                }
            } else {
                break;
            }
        }
    }

    /// Clear all cached thumbnails
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub fn clear_cache(&mut self, egui_renderer: &mut egui_wgpu::Renderer) {
        let mut cache = self.cache.write();
        for (_, entry) in cache.drain() {
            egui_renderer.free_texture(&entry.texture_id);
        }
        self.states.write().clear();
        *self.cached_bytes.write() = 0;
    }

    /// Invalidate cached thumbnails for specific paths (for hot reload)
    ///
    /// This removes the thumbnails from cache so they will be regenerated
    /// on next request.
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub fn invalidate_paths(
        &mut self,
        paths: impl IntoIterator<Item = impl AsRef<Path>>,
        egui_renderer: &mut egui_wgpu::Renderer,
    ) {
        let mut cache = self.cache.write();
        let mut states = self.states.write();
        let mut bytes = self.cached_bytes.write();

        for path in paths {
            let path = path.as_ref();

            // Remove from memory cache
            if let Some(entry) = cache.remove(path) {
                egui_renderer.free_texture(&entry.texture_id);
                *bytes -= entry.size_bytes;
            }

            // Remove from states so it can be re-requested
            states.remove(path);

            // Invalidate disk cache by removing cached files
            if self.disk_cache.is_some() {
                // The disk cache will be invalidated on next load
                // because the source file modification time changed
                tracing::debug!("Invalidated thumbnail cache for: {:?}", path);
            }
        }
    }

    /// Invalidate and immediately re-request thumbnails for the given paths
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub fn hot_reload_paths(&mut self, paths: &[PathBuf], egui_renderer: &mut egui_wgpu::Renderer) {
        // First invalidate
        self.invalidate_paths(paths.iter(), egui_renderer);

        // Then re-request (they'll be regenerated in the background)
        for path in paths {
            if Self::can_generate_thumbnail(path) {
                self.request_thumbnail(path);
            }
        }
    }

    /// Get cache statistics
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub fn cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.read();
        (cache.len(), *self.cached_bytes.read())
    }
}

impl Default for ThumbnailManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Worker thread that processes thumbnail generation requests
fn thumbnail_worker(
    mut request_rx: mpsc::UnboundedReceiver<ThumbnailRequest>,
    result_tx: mpsc::UnboundedSender<ThumbnailResult>,
) {
    // Use tokio runtime for async file operations
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");

    rt.block_on(async {
        while let Some(request) = request_rx.recv().await {
            // Try loading from disk cache first
            let disk_cache = request
                .cache_dir
                .as_ref()
                .and_then(|dir| DiskCache::new(dir).ok());

            if let Some(ref dc) = disk_cache {
                if dc.is_valid(&request.path, request.size) {
                    if let Some(data) = dc.load(&request.path) {
                        if result_tx.send(Ok(data)).is_err() {
                            break;
                        }
                        continue;
                    }
                }
            }

            // Generate thumbnail
            let result = generate_thumbnail(&request.path, request.size).await;

            // Save to disk cache if generation succeeded
            if let (Ok(ref data), Some(ref dc)) = (&result, &disk_cache) {
                if let Err(e) = dc.save(data, request.size) {
                    tracing::debug!("Failed to cache thumbnail to disk: {}", e);
                }
            }

            if result_tx.send(result).is_err() {
                break; // Channel closed
            }
        }
    });
}

/// Generate a thumbnail for an asset
async fn generate_thumbnail(path: &Path, size: u32) -> ThumbnailResult {
    // Check file exists
    if !path.exists() {
        return Err(ThumbnailError::NotFound(path.display().to_string()));
    }

    // Get file extension
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_lowercase)
        .ok_or_else(|| ThumbnailError::UnsupportedFormat("No extension".to_string()))?;

    // Load and process based on type
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "tga" | "webp" | "ppm" | "pgm" | "pbm"
        | "pam" => generate_image_thumbnail(path, size).await,
        "hdr" | "exr" => generate_hdr_thumbnail(path, size).await,
        _ => Err(ThumbnailError::UnsupportedFormat(ext)),
    }
}

/// Generate thumbnail for standard image formats
async fn generate_image_thumbnail(path: &Path, size: u32) -> ThumbnailResult {
    // Read file
    let data = fs::read(path)
        .map_err(|e| ThumbnailError::IoError(e.to_string()))?;

    // Decode image
    let img = image::load_from_memory(&data)
        .map_err(|e| ThumbnailError::DecodeError(e.to_string()))?;

    // Resize maintaining aspect ratio
    let thumbnail = resize_image(&img, size);

    Ok(ThumbnailData {
        path: path.to_path_buf(),
        pixels: thumbnail.to_rgba8().into_raw(),
        width: thumbnail.width(),
        height: thumbnail.height(),
    })
}

/// Generate thumbnail for HDR/EXR images
async fn generate_hdr_thumbnail(path: &Path, size: u32) -> ThumbnailResult {
    let data = fs::read(path)
        .map_err(|e| ThumbnailError::IoError(e.to_string()))?;

    let img =
        image::load_from_memory(&data).map_err(|e| ThumbnailError::DecodeError(e.to_string()))?;

    // Tonemap HDR to SDR
    let tonemapped = tonemap_hdr(&img);

    // Resize
    let thumbnail = resize_image(&tonemapped, size);

    Ok(ThumbnailData {
        path: path.to_path_buf(),
        pixels: thumbnail.to_rgba8().into_raw(),
        width: thumbnail.width(),
        height: thumbnail.height(),
    })
}

/// Resize image maintaining aspect ratio
fn resize_image(img: &DynamicImage, max_size: u32) -> DynamicImage {
    let (width, height) = img.dimensions();

    if width <= max_size && height <= max_size {
        return img.clone();
    }

    let ratio = width as f32 / height as f32;
    let (new_width, new_height) = if ratio > 1.0 {
        (max_size, (max_size as f32 / ratio) as u32)
    } else {
        ((max_size as f32 * ratio) as u32, max_size)
    };

    img.resize(new_width, new_height, image::imageops::FilterType::Lanczos3)
}

/// Simple Reinhard tonemapping for HDR images
fn tonemap_hdr(img: &DynamicImage) -> DynamicImage {
    let rgba = img.to_rgba32f();
    let (width, height) = rgba.dimensions();

    let mut output: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(width, height);

    for (x, y, pixel) in rgba.enumerate_pixels() {
        // Reinhard tonemapping
        let r = pixel[0] / (1.0 + pixel[0]);
        let g = pixel[1] / (1.0 + pixel[1]);
        let b = pixel[2] / (1.0 + pixel[2]);

        // Gamma correction
        let gamma = 1.0 / 2.2;
        let r = (r.powf(gamma) * 255.0).clamp(0.0, 255.0) as u8;
        let g = (g.powf(gamma) * 255.0).clamp(0.0, 255.0) as u8;
        let b = (b.powf(gamma) * 255.0).clamp(0.0, 255.0) as u8;
        let a = (pixel[3] * 255.0).clamp(0.0, 255.0) as u8;

        output.put_pixel(x, y, Rgba([r, g, b, a]));
    }

    DynamicImage::ImageRgba8(output)
}

/// Extract path from error (for updating state)
fn extract_path_from_error(_error: &ThumbnailError) -> Option<PathBuf> {
    // This is a simplified version - in practice we'd track this differently
    None
}

/// Create a wgpu texture from egui `ColorImage`
fn create_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    image: &egui::ColorImage,
) -> wgpu::TextureView {
    let size = wgpu::Extent3d {
        width: image.width() as u32,
        height: image.height() as u32,
        depth_or_array_layers: 1,
    };

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("thumbnail_texture"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    // Convert pixels to bytes
    let pixels: Vec<u8> = image
        .pixels
        .iter()
        .flat_map(egui::Color32::to_array)
        .collect();

    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &pixels,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(4 * image.width() as u32),
            rows_per_image: Some(image.height() as u32),
        },
        size,
    );

    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

/// Thumbnail display helper for egui
#[allow(dead_code)] // Intentionally kept for API completeness
pub struct ThumbnailWidget<'a> {
    path: &'a Path,
    size: egui::Vec2,
    manager: &'a ThumbnailManager,
}

#[allow(dead_code)] // Intentionally kept for API completeness
impl<'a> ThumbnailWidget<'a> {
    /// Create a new thumbnail widget
    pub fn new(path: &'a Path, size: impl Into<egui::Vec2>, manager: &'a ThumbnailManager) -> Self {
        Self {
            path,
            size: size.into(),
            manager,
        }
    }
}

impl egui::Widget for ThumbnailWidget<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(self.size, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            // Request thumbnail if not already loaded
            self.manager.request_thumbnail(self.path);

            match self.manager.get_state(self.path) {
                ThumbnailState::Ready(texture_id) => {
                    // Draw the thumbnail
                    ui.painter().image(
                        texture_id,
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
                ThumbnailState::Loading => {
                    // Draw loading indicator
                    let center = rect.center();
                    let time = ui.input(|i| i.time);
                    let angle = time as f32 * 3.0;
                    let radius = rect.width().min(rect.height()) * 0.3;

                    ui.painter().circle_stroke(
                        center,
                        radius,
                        egui::Stroke::new(2.0, egui::Color32::from_gray(60)),
                    );

                    // Spinning arc
                    use core::f32::consts::PI;
                    let start_angle = angle;
                    let end_angle = angle + PI * 0.75;
                    let points: Vec<egui::Pos2> = (0..20)
                        .map(|i| {
                            let t = i as f32 / 19.0;
                            let a = start_angle + (end_angle - start_angle) * t;
                            egui::pos2(center.x + radius * a.cos(), center.y + radius * a.sin())
                        })
                        .collect();

                    ui.painter().add(egui::Shape::line(
                        points,
                        egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 150, 255)),
                    ));

                    // Request repaint for animation
                    ui.ctx().request_repaint();
                }
                ThumbnailState::Failed(_) | ThumbnailState::UseDefault | ThumbnailState::NotLoaded => {
                    // Draw placeholder
                    ui.painter()
                        .rect_filled(rect, 4.0, egui::Color32::from_gray(45));
                    ui.painter().rect_stroke(
                        rect,
                        4.0,
                        egui::Stroke::new(1.0, egui::Color32::from_gray(60)),
                    );

                    // Draw icon placeholder
                    let icon = get_file_icon(self.path);
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        icon,
                        egui::FontId::proportional(self.size.x * 0.4),
                        egui::Color32::from_gray(120),
                    );
                }
            }
        }

        response
    }
}

/// Get icon character for a file type
fn get_file_icon(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_lowercase);

    match ext.as_deref() {
        // Images
        Some("png" | "jpg" | "jpeg" | "gif" | "bmp" | "tga" | "ico" | "webp") => "\u{f03e}",
        Some("hdr" | "exr") => "\u{f185}", // sun icon for HDR
        // 3D
        Some("glb" | "gltf" | "obj" | "fbx" | "dae") => "\u{f1b2}",
        // Audio
        Some("wav" | "mp3" | "ogg" | "flac") => "\u{f001}",
        // Video
        Some("mp4" | "avi" | "mkv" | "webm") => "\u{f03d}",
        // Code/Scripts
        Some("rs" | "lua" | "wasm") => "\u{f121}",
        // Shaders
        Some("wgsl" | "glsl" | "hlsl") => "\u{f0eb}",
        // Fonts
        Some("ttf" | "otf" | "woff" | "woff2") => "\u{f031}",
        // Materials
        Some("mat" | "material") => "\u{f5aa}",
        // Scenes
        Some("scene" | "ron") => "\u{f0c5}",
        // Default
        _ => "\u{f15b}",
    }
}
