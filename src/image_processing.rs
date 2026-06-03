//! Image processing utilities for the CM application

use crate::gui::state::CachedImageInfo;
use eyre::Result;
use eyre::eyre;
use image::DynamicImage;
use image::ImageFormat;
use image::Rgba;
use image::RgbaImage;
use img_parts::ImageEXIF;
use img_parts::jpeg::Jpeg;
use img_parts::png::Png;
use std::collections::HashSet;
use std::hash::BuildHasher;
use std::io::Cursor;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;

/// Maximum preview dimension (width or height)
const MAX_PREVIEW_SIZE: u32 = 1024;
/// Default preferred output size when file-size reduction is enabled.
pub const DEFAULT_MAX_FILE_SIZE_BYTES: u64 = 50 * 1024 * 1024;

/// Result of processing a single image
#[derive(Clone, Debug)]
pub struct ProcessedImage {
    /// The processed image data (in original format when possible)
    pub data: Vec<u8>,
    /// The output format used
    pub format: ImageFormat,
    /// Original width
    pub original_width: u32,
    /// Original height
    pub original_height: u32,
    /// Output width (after cropping if applied)
    pub output_width: u32,
    /// Output height (after cropping if applied)
    pub output_height: u32,
    /// Whether cropping was applied
    pub was_cropped: bool,
    /// Estimated output file size
    pub estimated_size: u64,
    /// Binarized threshold preview data (PNG encoded, downsampled for preview)
    pub threshold_preview_data: Vec<u8>,
    /// Downsampled output preview data (PNG encoded for display)
    pub output_preview_data: Vec<u8>,
    /// Crop bounds (x, y, width, height) if cropping was applied
    pub crop_bounds: Option<(u32, u32, u32, u32)>,
}

/// Binarization mode for threshold preview
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BinarizationMode {
    #[default]
    KeepWhite,
    KeepBlack,
}

/// Image processing settings
#[derive(Clone, Debug)]
pub struct ProcessingSettings {
    /// Whether image manipulation is enabled. When false, original bytes are preserved.
    pub image_manipulation_enabled: bool,
    /// Whether to crop whitespace/transparency from images
    pub crop_to_content: bool,
    /// Threshold value for crop detection (0-255)
    pub crop_threshold: u8,
    /// Binarization preview mode
    pub binarization_mode: BinarizationMode,
    /// Thickness of the red bounding box (1-10)
    pub box_thickness: u8,
    /// Whether to draw the crop-detection bounding box in the threshold preview.
    pub show_crop_bounding_box: bool,
    /// JPEG quality (1-100, default 90)
    pub jpeg_quality: u8,
    /// Optional maximum output file size in bytes
    pub max_file_size_bytes: Option<u64>,
    /// Optional description to write to image metadata
    pub description: Option<String>,
}

impl Default for ProcessingSettings {
    fn default() -> Self {
        Self {
            image_manipulation_enabled: true,
            crop_to_content: false,
            crop_threshold: 20,
            binarization_mode: BinarizationMode::default(),
            box_thickness: 10,
            show_crop_bounding_box: true,
            jpeg_quality: 90,
            max_file_size_bytes: None,
            description: None,
        }
    }
}

/// Output path planning options.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OutputPathOptions {
    /// Put all output files directly in the output root.
    pub flatten_output_hierarchy: bool,
    /// Use one shared output root for all input roots.
    pub save_all_inputs_to_same_folder: bool,
    /// Explicit shared output root selected by the user.
    pub shared_output_dir: Option<PathBuf>,
}

/// Detect the image format from the file extension
fn detect_format_from_path(path: &Path) -> ImageFormat {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map_or(ImageFormat::Png, |ext| match ext.to_lowercase().as_str() {
            "jpg" | "jpeg" => ImageFormat::Jpeg,
            "webp" => ImageFormat::WebP,
            "gif" => ImageFormat::Gif,
            "bmp" => ImageFormat::Bmp,
            "tiff" | "tif" => ImageFormat::Tiff,
            _ => ImageFormat::Png, // Default to PNG for unknown formats
        })
}

/// Downsample an image for preview while maintaining aspect ratio
#[expect(clippy::cast_possible_truncation)]
#[expect(clippy::cast_sign_loss)]
fn downsample_for_preview(img: &DynamicImage) -> DynamicImage {
    let (width, height) = (img.width(), img.height());

    // If already small enough, return clone
    if width <= MAX_PREVIEW_SIZE && height <= MAX_PREVIEW_SIZE {
        return img.clone();
    }

    // Calculate new dimensions maintaining aspect ratio
    let scale = (f64::from(MAX_PREVIEW_SIZE) / f64::from(width.max(height))).min(1.0);
    let new_width = (f64::from(width) * scale) as u32;
    let new_height = (f64::from(height) * scale) as u32;

    img.resize(new_width, new_height, image::imageops::FilterType::Triangle)
}

/// Load and process an image according to settings
/// # Errors
/// Returns an error if the image cannot be loaded or processed.
pub fn process_image(path: &Path, settings: &ProcessingSettings) -> Result<ProcessedImage> {
    // Detect original format for output
    let output_format = detect_format_from_path(path);

    // Load the image
    let img =
        image::open(path).map_err(|e| eyre!("Failed to open image {}: {}", path.display(), e))?;

    let original_width = img.width();
    let original_height = img.height();

    if !settings.image_manipulation_enabled {
        let data =
            std::fs::read(path).map_err(|e| eyre!("Failed to read {}: {}", path.display(), e))?;
        let output_preview_img = downsample_for_preview(&img);
        let mut output_preview_data = Vec::new();
        let mut preview_cursor = Cursor::new(&mut output_preview_data);
        output_preview_img
            .write_to(&mut preview_cursor, ImageFormat::Png)
            .map_err(|e| eyre!("Failed to encode output preview: {}", e))?;
        let threshold_preview_data = output_preview_data.clone();

        return Ok(ProcessedImage {
            estimated_size: data.len() as u64,
            data,
            format: output_format,
            original_width,
            original_height,
            output_width: original_width,
            output_height: original_height,
            was_cropped: false,
            threshold_preview_data,
            output_preview_data,
            crop_bounds: None,
        });
    }

    // Generate threshold preview using downsampled image for performance
    let box_thickness = if settings.box_thickness == 0 {
        10
    } else {
        settings.box_thickness
    };
    let preview_img = downsample_for_preview(&img);
    let threshold_preview_data = create_threshold_preview(
        &preview_img,
        settings.crop_threshold,
        settings.binarization_mode,
        box_thickness,
        settings.show_crop_bounding_box,
    )?;

    // Apply processing steps
    let (processed, was_cropped, crop_bounds) = if settings.crop_to_content {
        let (cropped, bounds) = crop_to_content_with_threshold(&img, settings.crop_threshold);
        let did_crop = cropped.width() != original_width || cropped.height() != original_height;
        (
            cropped,
            did_crop,
            if did_crop { Some(bounds) } else { None },
        )
    } else {
        (img, false, None)
    };

    let output_width = processed.width();
    let output_height = processed.height();

    // Create downsampled preview for GUI display (always PNG for fast decoding)
    let output_preview_img = downsample_for_preview(&processed);
    let mut output_preview_data = Vec::new();
    let mut preview_cursor = Cursor::new(&mut output_preview_data);
    output_preview_img
        .write_to(&mut preview_cursor, ImageFormat::Png)
        .map_err(|e| eyre!("Failed to encode output preview: {}", e))?;

    let exif_data = if let Some(ref description) = settings.description
        && !description.is_empty()
    {
        // Read existing EXIF from source if available
        let existing_exif = read_exif_bytes(path);
        Some(merge_description_into_exif(
            existing_exif.as_deref(),
            description,
        ))
    } else {
        None
    };

    // Encode full-resolution output using the original format
    let data = encode_image_with_size_limit(
        &processed,
        output_format,
        settings.jpeg_quality,
        settings.max_file_size_bytes,
        exif_data.as_deref(),
    )?;

    let estimated_size = data.len() as u64;

    Ok(ProcessedImage {
        data,
        format: output_format,
        original_width,
        original_height,
        output_width,
        output_height,
        was_cropped,
        estimated_size,
        threshold_preview_data,
        output_preview_data,
        crop_bounds,
    })
}

/// Encode an image to the specified format
fn encode_image(img: &DynamicImage, format: ImageFormat, jpeg_quality: u8) -> Result<Vec<u8>> {
    let mut data = Vec::new();
    let mut cursor = Cursor::new(&mut data);

    match format {
        ImageFormat::Jpeg => {
            // Use JPEG encoder with quality setting
            let quality = if jpeg_quality == 0 { 90 } else { jpeg_quality };
            let rgb = img.to_rgb8();
            let mut encoder =
                image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, quality);
            encoder
                .encode(
                    rgb.as_raw(),
                    rgb.width(),
                    rgb.height(),
                    image::ExtendedColorType::Rgb8,
                )
                .map_err(|e| eyre!("Failed to encode JPEG: {}", e))?;
        }
        ImageFormat::WebP => {
            // WebP uses quality-like encoding
            img.write_to(&mut cursor, ImageFormat::WebP)
                .map_err(|e| eyre!("Failed to encode WebP: {}", e))?;
        }
        _ => {
            // Default to PNG for other formats (lossless)
            img.write_to(&mut cursor, ImageFormat::Png)
                .map_err(|e| eyre!("Failed to encode PNG: {}", e))?;
        }
    }

    Ok(data)
}

fn encode_image_with_exif(
    img: &DynamicImage,
    format: ImageFormat,
    jpeg_quality: u8,
    exif_data: Option<&[u8]>,
) -> Result<Vec<u8>> {
    let mut data = encode_image(img, format, jpeg_quality)?;
    if let Some(exif_data) = exif_data {
        data = embed_exif(&data, format, exif_data)?;
    }
    Ok(data)
}

fn encode_image_with_size_limit(
    img: &DynamicImage,
    format: ImageFormat,
    jpeg_quality: u8,
    max_file_size_bytes: Option<u64>,
    exif_data: Option<&[u8]>,
) -> Result<Vec<u8>> {
    let initial_quality = if jpeg_quality == 0 { 90 } else { jpeg_quality };
    let mut data = encode_image_with_exif(img, format, initial_quality, exif_data)?;
    let Some(max_size) = max_file_size_bytes else {
        return Ok(data);
    };

    if data.len() as u64 <= max_size {
        return Ok(data);
    }

    if format == ImageFormat::Jpeg {
        data = encode_jpeg_under_limit(img, initial_quality, max_size, exif_data)?;
        if data.len() as u64 <= max_size {
            return Ok(data);
        }
    }

    resize_under_limit(img, format, initial_quality, max_size, exif_data, data)
}

fn encode_jpeg_under_limit(
    img: &DynamicImage,
    max_quality: u8,
    max_size: u64,
    exif_data: Option<&[u8]>,
) -> Result<Vec<u8>> {
    let mut low = 1_u8;
    let mut high = max_quality.max(1);
    let mut best_under: Option<Vec<u8>> = None;
    let mut smallest = encode_image_with_exif(img, ImageFormat::Jpeg, low, exif_data)?;

    while low <= high {
        let quality = low + (high - low) / 2;
        let candidate = encode_image_with_exif(img, ImageFormat::Jpeg, quality, exif_data)?;
        if candidate.len() < smallest.len() {
            smallest.clone_from(&candidate);
        }

        if candidate.len() as u64 <= max_size {
            best_under = Some(candidate);
            low = quality.saturating_add(1);
        } else if quality == 1 {
            break;
        } else {
            high = quality - 1;
        }
    }

    Ok(best_under.unwrap_or(smallest))
}

#[expect(clippy::cast_possible_truncation)]
#[expect(clippy::cast_sign_loss)]
#[expect(clippy::cast_precision_loss)]
fn resize_under_limit(
    img: &DynamicImage,
    format: ImageFormat,
    jpeg_quality: u8,
    max_size: u64,
    exif_data: Option<&[u8]>,
    mut best: Vec<u8>,
) -> Result<Vec<u8>> {
    let mut working = img.clone();

    for _ in 0..24 {
        if best.len() as u64 <= max_size {
            return Ok(best);
        }

        if working.width() <= 1 && working.height() <= 1 {
            return Ok(best);
        }

        let scale = ((max_size as f64 / best.len() as f64).sqrt() * 0.95).clamp(0.1, 0.95);
        let new_width = ((f64::from(working.width()) * scale).round() as u32)
            .max(1)
            .min(working.width().saturating_sub(1).max(1));
        let new_height = ((f64::from(working.height()) * scale).round() as u32)
            .max(1)
            .min(working.height().saturating_sub(1).max(1));

        working = working.resize(new_width, new_height, image::imageops::FilterType::Triangle);
        let candidate = if format == ImageFormat::Jpeg {
            encode_jpeg_under_limit(&working, jpeg_quality, max_size, exif_data)?
        } else {
            encode_image_with_exif(&working, format, jpeg_quality, exif_data)?
        };

        if candidate.len() < best.len() {
            best = candidate;
        }
    }

    Ok(best)
}

/// Read existing EXIF data from a source file
fn read_exif_bytes(path: &Path) -> Option<Vec<u8>> {
    let data = std::fs::read(path).ok()?;
    let format = detect_format_from_path(path);

    match format {
        ImageFormat::Jpeg => {
            let jpeg = Jpeg::from_bytes(data.into()).ok()?;
            jpeg.exif().map(|e| e.to_vec())
        }
        ImageFormat::Png => {
            let png = Png::from_bytes(data.into()).ok()?;
            png.exif().map(|e| e.to_vec())
        }
        _ => None,
    }
}

/// Create a minimal EXIF segment with `ImageDescription` tag
/// The EXIF format is complex; this creates a simple TIFF-based EXIF structure
#[expect(clippy::cast_possible_truncation)]
fn create_exif_with_description(description: &str) -> Vec<u8> {
    // EXIF uses TIFF format. We'll create a minimal structure:
    // - TIFF header (8 bytes)
    // - IFD0 with ImageDescription tag (0x010E)

    let desc_bytes = description.as_bytes();
    let desc_len = desc_bytes.len() as u32 + 1; // +1 for null terminator

    // Calculate offsets
    let ifd0_offset: u32 = 8; // Right after TIFF header
    let ifd0_entries: u16 = 1; // Just ImageDescription
    let ifd0_size = 2 + 12 * ifd0_entries as usize + 4; // entry count + entries + next IFD pointer
    let data_offset: u32 = ifd0_offset + ifd0_size as u32;

    let mut exif = Vec::new();

    // TIFF header (little-endian)
    exif.extend_from_slice(b"II"); // Little-endian marker
    exif.extend_from_slice(&42u16.to_le_bytes()); // TIFF magic number
    exif.extend_from_slice(&ifd0_offset.to_le_bytes()); // Offset to IFD0

    // IFD0
    exif.extend_from_slice(&ifd0_entries.to_le_bytes()); // Number of entries

    // ImageDescription tag (0x010E)
    exif.extend_from_slice(&0x010Eu16.to_le_bytes()); // Tag
    exif.extend_from_slice(&2u16.to_le_bytes()); // Type: ASCII
    exif.extend_from_slice(&desc_len.to_le_bytes()); // Count
    if desc_len <= 4 {
        // Value fits in offset field
        let mut value = [0u8; 4];
        value[..desc_bytes.len()].copy_from_slice(desc_bytes);
        exif.extend_from_slice(&value);
    } else {
        // Value stored at data_offset
        exif.extend_from_slice(&data_offset.to_le_bytes());
    }

    // Next IFD pointer (0 = no more IFDs)
    exif.extend_from_slice(&0u32.to_le_bytes());

    // Description data (if longer than 4 bytes)
    if desc_len > 4 {
        exif.extend_from_slice(desc_bytes);
        exif.push(0); // Null terminator
    }

    exif
}

/// Merge a description into existing EXIF data, or create new EXIF with just the description
fn merge_description_into_exif(existing_exif: Option<&[u8]>, description: &str) -> Vec<u8> {
    // For simplicity, we just create new EXIF with the description
    // A more sophisticated implementation would parse and modify existing EXIF
    // but that's quite complex. The description will be the main metadata we care about.
    let _ = existing_exif; // Acknowledge but don't use for now
    create_exif_with_description(description)
}

/// Embed EXIF data into image bytes
fn embed_exif(image_data: &[u8], format: ImageFormat, exif_data: &[u8]) -> Result<Vec<u8>> {
    match format {
        ImageFormat::Jpeg => {
            let mut jpeg = Jpeg::from_bytes(image_data.to_vec().into())
                .map_err(|e| eyre!("Failed to parse JPEG for EXIF embedding: {}", e))?;
            jpeg.set_exif(Some(exif_data.to_vec().into()));
            let mut output = Vec::new();
            jpeg.encoder()
                .write_to(&mut output)
                .map_err(|e| eyre!("Failed to write JPEG with EXIF: {}", e))?;
            Ok(output)
        }
        ImageFormat::Png => {
            let mut png = Png::from_bytes(image_data.to_vec().into())
                .map_err(|e| eyre!("Failed to parse PNG for EXIF embedding: {}", e))?;
            png.set_exif(Some(exif_data.to_vec().into()));
            let mut output = Vec::new();
            png.encoder()
                .write_to(&mut output)
                .map_err(|e| eyre!("Failed to write PNG with EXIF: {}", e))?;
            Ok(output)
        }
        _ => {
            // For unsupported formats, return original data
            Ok(image_data.to_vec())
        }
    }
}

/// Create a binarized threshold preview of the image
fn create_threshold_preview(
    img: &DynamicImage,
    threshold: u8,
    mode: BinarizationMode,
    box_thickness: u8,
    show_bounding_box: bool,
) -> Result<Vec<u8>> {
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    // Sample edge pixels to determine background color
    let background_color = sample_edge_color(&rgba);

    // Create binarized image
    let mut binary_img = RgbaImage::new(width, height);

    for y in 0..height {
        for x in 0..width {
            let pixel = rgba.get_pixel(x, y);
            let is_background =
                is_background_pixel_with_threshold(*pixel, background_color, threshold);

            // Set pixel color based on mode
            let output_pixel = match mode {
                BinarizationMode::KeepWhite => {
                    if is_background {
                        Rgba([0, 0, 0, 255]) // Black for background
                    } else {
                        Rgba([255, 255, 255, 255]) // White for content
                    }
                }
                BinarizationMode::KeepBlack => {
                    if is_background {
                        Rgba([255, 255, 255, 255]) // White for background
                    } else {
                        Rgba([0, 0, 0, 255]) // Black for content
                    }
                }
            };

            binary_img.put_pixel(x, y, output_pixel);
        }
    }

    // Draw red bounding box if there's content to crop
    if show_bounding_box
        && let Some((min_x, min_y, max_x, max_y)) =
            find_content_bounds(&rgba, background_color, threshold)
    {
        draw_bounding_box(
            &mut binary_img,
            min_x,
            min_y,
            max_x,
            max_y,
            u32::from(box_thickness),
        );
    }

    // Encode to PNG
    let mut data = Vec::new();
    let mut cursor = Cursor::new(&mut data);
    DynamicImage::ImageRgba8(binary_img)
        .write_to(&mut cursor, ImageFormat::Png)
        .map_err(|e| eyre!("Failed to encode threshold preview: {}", e))?;

    Ok(data)
}

/// Sample edge pixels to determine the most common background color
#[expect(clippy::cast_possible_truncation)]
fn sample_edge_color(img: &RgbaImage) -> Rgba<u8> {
    let (width, height) = img.dimensions();

    if width == 0 || height == 0 {
        return Rgba([255, 255, 255, 255]);
    }

    let mut samples = Vec::new();

    // Sample top and bottom edges
    for x in (0..width).step_by((width / 10).max(1) as usize) {
        samples.push(*img.get_pixel(x, 0));
        if height > 1 {
            samples.push(*img.get_pixel(x, height - 1));
        }
    }

    // Sample left and right edges
    for y in (0..height).step_by((height / 10).max(1) as usize) {
        samples.push(*img.get_pixel(0, y));
        if width > 1 {
            samples.push(*img.get_pixel(width - 1, y));
        }
    }

    // Return the average color (simple approach)
    if samples.is_empty() {
        return Rgba([255, 255, 255, 255]);
    }

    let mut r_sum: u64 = 0;
    let mut g_sum: u64 = 0;
    let mut b_sum: u64 = 0;
    let mut a_sum: u64 = 0;

    for pixel in &samples {
        r_sum += u64::from(pixel[0]);
        g_sum += u64::from(pixel[1]);
        b_sum += u64::from(pixel[2]);
        a_sum += u64::from(pixel[3]);
    }

    let count = samples.len() as u64;
    Rgba([
        (r_sum / count) as u8,
        (g_sum / count) as u8,
        (b_sum / count) as u8,
        (a_sum / count) as u8,
    ])
}

/// Check if a pixel is background based on threshold
fn is_background_pixel_with_threshold(
    pixel: Rgba<u8>,
    background: Rgba<u8>,
    threshold: u8,
) -> bool {
    // Transparent pixels are always background
    if pixel[3] < 10 {
        return true;
    }

    // Calculate color distance from background
    let dr = (i32::from(pixel[0]) - i32::from(background[0])).abs();
    let dg = (i32::from(pixel[1]) - i32::from(background[1])).abs();
    let db = (i32::from(pixel[2]) - i32::from(background[2])).abs();

    // Use Euclidean distance
    let distance = f64::from(dr * dr + dg * dg + db * db).sqrt();

    // Compare against threshold
    distance < f64::from(threshold)
}

/// Find content bounds using threshold - optimized edge-inward scanning
/// Instead of scanning every pixel, we scan from each edge inward until we find content.
/// This is much faster for images where content is roughly centered with padding.
fn find_content_bounds(
    img: &RgbaImage,
    background: Rgba<u8>,
    threshold: u8,
) -> Option<(u32, u32, u32, u32)> {
    let (width, height) = img.dimensions();

    if width == 0 || height == 0 {
        return None;
    }

    // Find min_y: scan from top down until we find a row with content
    let mut min_y = 0u32;
    'top: for y in 0..height {
        for x in 0..width {
            let pixel = img.get_pixel(x, y);
            if !is_background_pixel_with_threshold(*pixel, background, threshold) {
                min_y = y;
                break 'top;
            }
        }
        min_y = y + 1;
    }

    // If we scanned all rows and found nothing, no content
    if min_y >= height {
        return None;
    }

    // Find max_y: scan from bottom up until we find a row with content
    let mut max_y = height - 1;
    'bottom: for y in (min_y..height).rev() {
        for x in 0..width {
            let pixel = img.get_pixel(x, y);
            if !is_background_pixel_with_threshold(*pixel, background, threshold) {
                max_y = y;
                break 'bottom;
            }
        }
    }

    // Find min_x: scan from left to right, but only in the y range we know has content
    let mut min_x = 0u32;
    'left: for x in 0..width {
        for y in min_y..=max_y {
            let pixel = img.get_pixel(x, y);
            if !is_background_pixel_with_threshold(*pixel, background, threshold) {
                min_x = x;
                break 'left;
            }
        }
        min_x = x + 1;
    }

    // Find max_x: scan from right to left, but only in the y range we know has content
    let mut max_x = width - 1;
    'right: for x in (min_x..width).rev() {
        for y in min_y..=max_y {
            let pixel = img.get_pixel(x, y);
            if !is_background_pixel_with_threshold(*pixel, background, threshold) {
                max_x = x;
                break 'right;
            }
        }
    }

    Some((min_x, min_y, max_x, max_y))
}

/// Draw a red bounding box on an image
fn draw_bounding_box(
    img: &mut RgbaImage,
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
    thickness: u32,
) {
    let red = Rgba([255, 0, 0, 255]);

    let (width, height) = img.dimensions();

    // Draw top and bottom edges
    for x in min_x..=max_x {
        if x < width {
            for t in 0..thickness {
                if min_y + t < height {
                    img.put_pixel(x, min_y + t, red);
                }
                if max_y >= t && max_y - t < height {
                    img.put_pixel(x, max_y - t, red);
                }
            }
        }
    }

    // Draw left and right edges
    for y in min_y..=max_y {
        if y < height {
            for t in 0..thickness {
                if min_x + t < width {
                    img.put_pixel(min_x + t, y, red);
                }
                if max_x >= t && max_x - t < width {
                    img.put_pixel(max_x - t, y, red);
                }
            }
        }
    }
}

/// Crop an image to its content using threshold-based detection
#[must_use]
pub fn crop_to_content_with_threshold(
    img: &DynamicImage,
    threshold: u8,
) -> (DynamicImage, (u32, u32, u32, u32)) {
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    if width == 0 || height == 0 {
        return (img.clone(), (0, 0, width, height));
    }

    // Sample edge to determine background color
    let background_color = sample_edge_color(&rgba);

    // Find bounds of non-background content
    if let Some((min_x, min_y, max_x, max_y)) =
        find_content_bounds(&rgba, background_color, threshold)
    {
        // Crop to the content bounds
        let crop_width = max_x - min_x + 1;
        let crop_height = max_y - min_y + 1;

        (
            img.crop_imm(min_x, min_y, crop_width, crop_height),
            (min_x, min_y, crop_width, crop_height),
        )
    } else {
        // No content found, return original
        (img.clone(), (0, 0, width, height))
    }
}

/// Crop an image to its content, removing whitespace/transparent padding
#[must_use]
pub fn crop_to_content(img: &DynamicImage) -> DynamicImage {
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    if width == 0 || height == 0 {
        return img.clone();
    }

    // Find bounds of non-background content
    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0u32;
    let mut max_y = 0u32;

    for y in 0..height {
        for x in 0..width {
            let pixel = rgba.get_pixel(x, y);
            if !is_background_pixel(*pixel) {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }

    // If the whole image is background, return original
    if max_x < min_x || max_y < min_y {
        return img.clone();
    }

    // Crop to the content bounds
    let crop_width = max_x - min_x + 1;
    let crop_height = max_y - min_y + 1;

    img.crop_imm(min_x, min_y, crop_width, crop_height)
}

/// Check if a pixel is considered "background" (white or transparent)
fn is_background_pixel(pixel: image::Rgba<u8>) -> bool {
    let [r, g, b, a] = pixel.0;

    // Transparent pixels are background
    if a < 10 {
        return true;
    }

    // Near-white pixels are background (with generous tolerance for JPEG artifacts)
    // Using 240 to catch off-white pixels from compression/anti-aliasing
    let threshold = 240;
    r >= threshold && g >= threshold && b >= threshold
}

/// Get the output directory for an input path (appends -output to directory name)
#[must_use]
pub fn get_output_dir(input_path: &Path) -> PathBuf {
    if let Some(parent) = input_path.parent()
        && let Some(name) = input_path.file_name()
    {
        let output_name = format!("{}-output", name.to_string_lossy());
        return parent.join(output_name);
    }
    // Fallback
    input_path.with_file_name(format!(
        "{}-output",
        input_path
            .file_name()
            .map(|s| s.to_string_lossy())
            .unwrap_or_default()
    ))
}

/// Get the shared output directory for a set of input roots.
#[must_use]
pub fn get_shared_output_dir(input_roots: &[PathBuf]) -> Option<PathBuf> {
    get_shared_input_base(input_roots).map(|base| get_output_dir(&base))
}

/// Get the shared input base for a set of input roots.
#[must_use]
pub fn get_shared_input_base(input_roots: &[PathBuf]) -> Option<PathBuf> {
    let first = input_roots.first()?;
    let mut common = if first.is_file() {
        first.parent()?.to_path_buf()
    } else {
        first.clone()
    };

    for root in &input_roots[1..] {
        let comparable = if root.is_file() {
            root.parent()?.to_path_buf()
        } else {
            root.clone()
        };

        while !comparable.starts_with(&common) {
            if !common.pop() {
                return if first.is_file() {
                    first.parent().map(Path::to_path_buf)
                } else {
                    Some(first.clone())
                };
            }
        }
    }

    Some(common)
}

/// Get the output path for a file given its input path and the original input root
#[must_use]
pub fn get_output_path(
    file_path: &Path,
    input_root: &Path,
    renamed_filename: &str,
) -> Option<PathBuf> {
    let input_roots = vec![input_root.to_path_buf()];
    get_output_path_with_options(
        file_path,
        input_root,
        renamed_filename,
        &input_roots,
        &OutputPathOptions::default(),
    )
}

/// Get the output path for a file with configurable hierarchy behavior.
#[must_use]
pub fn get_output_path_with_options(
    file_path: &Path,
    input_root: &Path,
    renamed_filename: &str,
    input_roots: &[PathBuf],
    options: &OutputPathOptions,
) -> Option<PathBuf> {
    // Get relative path from input root or the shared parent.
    let relative_root = if options.save_all_inputs_to_same_folder {
        get_shared_input_base(input_roots)?
    } else {
        input_root.to_path_buf()
    };
    let relative = file_path.strip_prefix(relative_root).ok()?;

    // Get output root directory
    let output_root = if options.save_all_inputs_to_same_folder {
        options
            .shared_output_dir
            .clone()
            .or_else(|| get_shared_output_dir(input_roots))?
    } else {
        get_output_dir(input_root)
    };

    // Build output path: output_root + relative_dir + renamed_filename
    let mut output_path = output_root;
    if !options.flatten_output_hierarchy
        && let Some(parent) = relative.parent()
    {
        output_path = output_path.join(parent);
    }
    output_path = output_path.join(renamed_filename);

    Some(output_path)
}

/// Reserve a conflict-free output path, appending " (1)", " (2)", etc. as needed.
///
/// # Panics
/// Panics if the mutex for reserved paths cannot be locked.
#[must_use]
pub fn reserve_available_output_path<S: BuildHasher>(
    desired_path: &Path,
    reserved_paths: &Mutex<HashSet<PathBuf, S>>,
) -> PathBuf {
    let mut reserved = reserved_paths.lock().unwrap();
    let mut candidate = desired_path.to_path_buf();
    let mut suffix = 1_u32;

    while candidate.exists() || reserved.contains(&candidate) {
        candidate = numbered_path(desired_path, suffix);
        suffix += 1;
    }

    reserved.insert(candidate.clone());
    candidate
}

fn numbered_path(path: &Path, suffix: u32) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy())
        .unwrap_or_default();
    let file_name = if let Some(ext) = path.extension() {
        format!("{stem} ({suffix}).{}", ext.to_string_lossy())
    } else {
        format!("{stem} ({suffix})")
    };
    parent.join(file_name)
}
/// Process and write all images
/// # Errors
/// Returns an error if processing any image fails.
#[expect(clippy::type_complexity)]
pub fn process_all_images(
    input_files: &[PathBuf],
    renamed_files: &[PathBuf],
    input_roots: &[PathBuf],
    settings: &ProcessingSettings,
    progress_callback: Option<&dyn Fn(usize, usize, &Path)>,
) -> Result<ProcessAllResult> {
    let mut processed_count = 0;
    let skipped_count = 0;
    let mut error_count = 0;
    let mut errors: Vec<String> = Vec::new();

    let total = input_files.len();

    for (i, (input_file, renamed_file)) in input_files.iter().zip(renamed_files.iter()).enumerate()
    {
        if let Some(cb) = progress_callback {
            cb(i + 1, total, input_file);
        }

        // Find which input root this file belongs to
        let input_root = input_roots.iter().find(|r| input_file.starts_with(r));
        let Some(input_root) = input_root else {
            errors.push(format!("Could not find root for: {}", input_file.display()));
            error_count += 1;
            continue;
        };

        // Get the renamed filename
        let renamed_name = renamed_file
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        // Calculate output path
        let Some(output_path) = get_output_path(input_file, input_root, &renamed_name) else {
            errors.push(format!(
                "Could not calculate output path for: {}",
                input_file.display()
            ));
            error_count += 1;
            continue;
        };

        // Create output directory if needed
        if let Some(parent) = output_path.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            errors.push(format!(
                "Failed to create directory {}: {}",
                parent.display(),
                e
            ));
            error_count += 1;
            continue;
        }

        // Process the image
        match process_image(input_file, settings) {
            Ok(processed) => {
                // Write output file
                if let Err(e) = std::fs::write(&output_path, &processed.data) {
                    errors.push(format!("Failed to write {}: {}", output_path.display(), e));
                    error_count += 1;
                } else {
                    processed_count += 1;
                }
            }
            Err(e) => {
                errors.push(format!("Failed to process {}: {}", input_file.display(), e));
                error_count += 1;
            }
        }
    }

    Ok(ProcessAllResult {
        processed_count,
        skipped_count,
        error_count,
        errors,
    })
}

/// Result of processing all images
#[derive(Debug)]
pub struct ProcessAllResult {
    pub processed_count: usize,
    pub skipped_count: usize,
    pub error_count: usize,
    pub errors: Vec<String>,
}

/// Load image metadata and generate a thumbnail for caching
/// # Errors
/// Returns an error if the image cannot be loaded or metadata cannot be retrieved.
#[expect(clippy::cast_possible_truncation)]
#[expect(clippy::cast_sign_loss)]
pub fn load_image_metadata(path: &Path, thumbnail_size: u32) -> Result<CachedImageInfo> {
    // Get file size
    let file_size = std::fs::metadata(path)
        .map_err(|e| eyre!("Failed to get file metadata: {}", e))?
        .len();

    // Load the image
    let img =
        image::open(path).map_err(|e| eyre!("Failed to open image {}: {}", path.display(), e))?;

    let width = img.width();
    let height = img.height();

    // Generate thumbnail
    let thumbnail = if width <= thumbnail_size && height <= thumbnail_size {
        img
    } else {
        let scale = (f64::from(thumbnail_size) / f64::from(width.max(height))).min(1.0);
        let new_width = (f64::from(width) * scale) as u32;
        let new_height = (f64::from(height) * scale) as u32;
        img.resize(new_width, new_height, image::imageops::FilterType::Triangle)
    };

    // Encode thumbnail as PNG
    let mut thumbnail_data = Vec::new();
    let mut cursor = Cursor::new(&mut thumbnail_data);
    thumbnail
        .write_to(&mut cursor, ImageFormat::Png)
        .map_err(|e| eyre!("Failed to encode thumbnail: {}", e))?;

    Ok(CachedImageInfo {
        width,
        height,
        file_size,
        thumbnail_data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbImage;
    use std::fs;
    use tempfile::tempdir;

    fn write_jpeg(path: &Path, width: u32, height: u32, quality: u8) -> Result<Vec<u8>> {
        let mut img = RgbImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let r = ((x * 7 + y * 3) % 255) as u8;
                let g = ((x * 5 + y * 11) % 255) as u8;
                let b = ((x * 13 + y * 17) % 255) as u8;
                img.put_pixel(x, y, image::Rgb([r, g, b]));
            }
        }

        let mut data = Vec::new();
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut data, quality);
        encoder.encode(img.as_raw(), width, height, image::ExtendedColorType::Rgb8)?;
        fs::write(path, &data)?;
        Ok(data)
    }

    #[test]
    fn reserve_available_output_path_continues_numbered_conflicts() -> Result<()> {
        let dir = tempdir()?;
        let desired = dir.path().join("photo.jpg");
        fs::write(&desired, b"existing")?;
        fs::write(dir.path().join("photo (1).jpg"), b"existing")?;
        fs::write(dir.path().join("photo (2).jpg"), b"existing")?;

        let reserved = Mutex::new(HashSet::new());
        let path = reserve_available_output_path(&desired, &reserved);

        assert_eq!(path, dir.path().join("photo (3).jpg"));
        Ok(())
    }

    #[test]
    fn shared_output_path_preserves_source_folder_when_not_flattened() -> Result<()> {
        let dir = tempdir()?;
        let root_a = dir.path().join("a");
        let root_b = dir.path().join("b");
        fs::create_dir_all(root_a.join("nested"))?;
        fs::create_dir_all(&root_b)?;
        let file = root_a.join("nested").join("photo.jpg");
        fs::write(&file, b"data")?;
        let roots = vec![root_a.clone(), root_b];

        let output = get_output_path_with_options(
            &file,
            &root_a,
            "renamed.jpg",
            &roots,
            &OutputPathOptions {
                flatten_output_hierarchy: false,
                save_all_inputs_to_same_folder: true,
                shared_output_dir: None,
            },
        )
        .expect("output path");

        assert_eq!(
            output,
            dir.path()
                .parent()
                .unwrap()
                .join(format!(
                    "{}-output",
                    dir.path().file_name().unwrap().to_string_lossy()
                ))
                .join("a")
                .join("nested")
                .join("renamed.jpg")
        );
        Ok(())
    }

    #[test]
    fn flattened_output_path_drops_relative_folders() -> Result<()> {
        let dir = tempdir()?;
        let root = dir.path().join("input");
        fs::create_dir_all(root.join("nested"))?;
        let file = root.join("nested").join("photo.jpg");
        fs::write(&file, b"data")?;

        let output = get_output_path_with_options(
            &file,
            &root,
            "renamed.jpg",
            std::slice::from_ref(&root),
            &OutputPathOptions {
                flatten_output_hierarchy: true,
                save_all_inputs_to_same_folder: false,
                shared_output_dir: None,
            },
        )
        .expect("output path");

        assert_eq!(output, dir.path().join("input-output").join("renamed.jpg"));
        Ok(())
    }

    #[test]
    fn custom_shared_output_dir_is_used() -> Result<()> {
        let dir = tempdir()?;
        let root = dir.path().join("input");
        let custom_output = dir.path().join("Mom Renamed These");
        fs::create_dir_all(root.join("nested"))?;
        let file = root.join("nested").join("photo.jpg");
        fs::write(&file, b"data")?;

        let output = get_output_path_with_options(
            &file,
            &root,
            "renamed.jpg",
            std::slice::from_ref(&root),
            &OutputPathOptions {
                flatten_output_hierarchy: false,
                save_all_inputs_to_same_folder: true,
                shared_output_dir: Some(custom_output.clone()),
            },
        )
        .expect("output path");

        assert_eq!(output, custom_output.join("nested").join("renamed.jpg"));
        Ok(())
    }

    #[test]
    fn disabled_manipulation_preserves_original_bytes() -> Result<()> {
        let dir = tempdir()?;
        let file = dir.path().join("photo.jpg");
        let original = write_jpeg(&file, 32, 32, 80)?;

        let processed = process_image(
            &file,
            &ProcessingSettings {
                image_manipulation_enabled: false,
                max_file_size_bytes: Some(100),
                jpeg_quality: 1,
                ..ProcessingSettings::default()
            },
        )?;

        assert_eq!(processed.data, original);
        Ok(())
    }

    #[test]
    fn max_file_size_reduces_jpeg_output() -> Result<()> {
        let dir = tempdir()?;
        let file = dir.path().join("photo.jpg");
        let original = write_jpeg(&file, 512, 512, 95)?;
        let max_size = original.len() as u64 / 3;

        let processed = process_image(
            &file,
            &ProcessingSettings {
                max_file_size_bytes: Some(max_size),
                jpeg_quality: 95,
                ..ProcessingSettings::default()
            },
        )?;

        assert!(processed.data.len() < original.len());
        assert!(processed.data.len() as u64 <= max_size);
        Ok(())
    }

    #[test]
    fn threshold_preview_respects_bounding_box_visibility() -> Result<()> {
        let dir = tempdir()?;
        let file = dir.path().join("boxed.png");
        let mut img = RgbaImage::from_pixel(16, 16, Rgba([255, 255, 255, 255]));
        for y in 4..12 {
            for x in 4..12 {
                img.put_pixel(x, y, Rgba([0, 0, 0, 255]));
            }
        }
        DynamicImage::ImageRgba8(img).save(&file)?;

        let with_box = process_image(
            &file,
            &ProcessingSettings {
                crop_to_content: true,
                show_crop_bounding_box: true,
                box_thickness: 1,
                ..ProcessingSettings::default()
            },
        )?;
        let without_box = process_image(
            &file,
            &ProcessingSettings {
                crop_to_content: true,
                show_crop_bounding_box: false,
                box_thickness: 1,
                ..ProcessingSettings::default()
            },
        )?;

        assert!(red_pixel_count(&with_box.threshold_preview_data)? > 0);
        assert_eq!(red_pixel_count(&without_box.threshold_preview_data)?, 0);
        Ok(())
    }

    fn red_pixel_count(bytes: &[u8]) -> Result<usize> {
        let image = image::load_from_memory(bytes)?.to_rgba8();
        Ok(image
            .pixels()
            .filter(|pixel| pixel.0 == [255, 0, 0, 255])
            .count())
    }
}
