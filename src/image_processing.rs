//! Image processing utilities for the CM application

use eyre::{Result, eyre};
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
use std::io::Cursor;
use std::path::Path;

/// Result of processing a single image
#[derive(Clone, Debug)]
pub struct ProcessedImage {
    /// The processed image data (PNG encoded)
    pub data: Vec<u8>,
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
    /// Binarized threshold preview data (PNG encoded)
    pub threshold_preview_data: Vec<u8>,
    /// Crop bounds (x, y, width, height) if cropping was applied
    pub crop_bounds: Option<(u32, u32, u32, u32)>,
}

/// Binarization mode for threshold preview
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinarizationMode {
    KeepWhite,
    KeepBlack,
}

impl Default for BinarizationMode {
    fn default() -> Self {
        BinarizationMode::KeepWhite
    }
}

/// Image processing settings
#[derive(Clone, Debug, Default)]
pub struct ProcessingSettings {
    /// Whether to crop whitespace/transparency from images
    pub crop_to_content: bool,
    /// Threshold value for crop detection (0-255)
    pub crop_threshold: u8,
    /// Binarization preview mode
    pub binarization_mode: BinarizationMode,
}

/// Load and process an image according to settings
pub fn process_image(path: &Path, settings: &ProcessingSettings) -> Result<ProcessedImage> {
    // Load the image
    let img = image::open(path)
        .map_err(|e| eyre!("Failed to open image {}: {}", path.display(), e))?;
    
    let original_width = img.width();
    let original_height = img.height();
    
    // Generate threshold preview
    let threshold_preview_data = create_threshold_preview(&img, settings.crop_threshold, settings.binarization_mode)?;
    
    // Apply processing steps
    let (processed, was_cropped, crop_bounds) = if settings.crop_to_content {
        let (cropped, bounds) = crop_to_content_with_threshold(&img, settings.crop_threshold);
        let did_crop = cropped.width() != original_width || cropped.height() != original_height;
        (cropped, did_crop, if did_crop { Some(bounds) } else { None })
    } else {
        (img, false, None)
    };
    
    let output_width = processed.width();
    let output_height = processed.height();
    
    // Encode to PNG in memory
    let mut data = Vec::new();
    let mut cursor = Cursor::new(&mut data);
    processed.write_to(&mut cursor, ImageFormat::Png)
        .map_err(|e| eyre!("Failed to encode image: {}", e))?;
    
    let estimated_size = data.len() as u64;
    
    Ok(ProcessedImage {
        data,
        original_width,
        original_height,
        output_width,
        output_height,
        was_cropped,
        estimated_size,
        threshold_preview_data,
        crop_bounds,
    })
}

/// Create a binarized threshold preview of the image
fn create_threshold_preview(
    img: &DynamicImage,
    threshold: u8,
    mode: BinarizationMode,
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
            let is_background = is_background_pixel_with_threshold(pixel, &background_color, threshold);
            
            // Set pixel color based on mode
            let output_pixel = match mode {
                BinarizationMode::KeepWhite => {
                    if is_background {
                        Rgba([255, 255, 255, 255]) // White for background
                    } else {
                        Rgba([0, 0, 0, 255]) // Black for content
                    }
                }
                BinarizationMode::KeepBlack => {
                    if is_background {
                        Rgba([0, 0, 0, 255]) // Black for background
                    } else {
                        Rgba([255, 255, 255, 255]) // White for content
                    }
                }
            };
            
            binary_img.put_pixel(x, y, output_pixel);
        }
    }
    
    // Draw red bounding box if there's content to crop
    let bounds = find_content_bounds(&rgba, &background_color, threshold);
    if let Some((min_x, min_y, max_x, max_y)) = bounds {
        draw_bounding_box(&mut binary_img, min_x, min_y, max_x, max_y);
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
        r_sum += pixel[0] as u64;
        g_sum += pixel[1] as u64;
        b_sum += pixel[2] as u64;
        a_sum += pixel[3] as u64;
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
    pixel: &Rgba<u8>,
    background: &Rgba<u8>,
    threshold: u8,
) -> bool {
    // Transparent pixels are always background
    if pixel[3] < 10 {
        return true;
    }
    
    // Calculate color distance from background
    let dr = (pixel[0] as i32 - background[0] as i32).abs();
    let dg = (pixel[1] as i32 - background[1] as i32).abs();
    let db = (pixel[2] as i32 - background[2] as i32).abs();
    
    // Use Euclidean distance
    let distance = ((dr * dr + dg * dg + db * db) as f64).sqrt();
    
    // Compare against threshold
    distance < threshold as f64
}

/// Find content bounds using threshold
fn find_content_bounds(
    img: &RgbaImage,
    background: &Rgba<u8>,
    threshold: u8,
) -> Option<(u32, u32, u32, u32)> {
    let (width, height) = img.dimensions();
    
    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0u32;
    let mut max_y = 0u32;
    let mut found_content = false;
    
    for y in 0..height {
        for x in 0..width {
            let pixel = img.get_pixel(x, y);
            if !is_background_pixel_with_threshold(pixel, background, threshold) {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
                found_content = true;
            }
        }
    }
    
    if found_content {
        Some((min_x, min_y, max_x, max_y))
    } else {
        None
    }
}

/// Draw a red bounding box on an image
fn draw_bounding_box(img: &mut RgbaImage, min_x: u32, min_y: u32, max_x: u32, max_y: u32) {
    let red = Rgba([255, 0, 0, 255]);
    let thickness = 2;
    
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
    if let Some((min_x, min_y, max_x, max_y)) = find_content_bounds(&rgba, &background_color, threshold) {
        // Crop to the content bounds
        let crop_width = max_x - min_x + 1;
        let crop_height = max_y - min_y + 1;
        
        (img.crop_imm(min_x, min_y, crop_width, crop_height), (min_x, min_y, crop_width, crop_height))
    } else {
        // No content found, return original
        (img.clone(), (0, 0, width, height))
    }
}

/// Crop an image to its content, removing whitespace/transparent padding
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
            if !is_background_pixel(pixel) {
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
fn is_background_pixel(pixel: &image::Rgba<u8>) -> bool {
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
pub fn get_output_dir(input_path: &Path) -> std::path::PathBuf {
    if let Some(parent) = input_path.parent() {
        if let Some(name) = input_path.file_name() {
            let output_name = format!("{}-output", name.to_string_lossy());
            return parent.join(output_name);
        }
    }
    // Fallback
    input_path.with_file_name(format!(
        "{}-output",
        input_path.file_name().map(|s| s.to_string_lossy()).unwrap_or_default()
    ))
}

/// Get the output path for a file given its input path and the original input root
pub fn get_output_path(
    file_path: &Path,
    input_root: &Path,
    renamed_filename: &str,
) -> Option<std::path::PathBuf> {
    // Get relative path from input root
    let relative = file_path.strip_prefix(input_root).ok()?;
    
    // Get output root directory
    let output_root = get_output_dir(input_root);
    
    // Build output path: output_root + relative_dir + renamed_filename
    let mut output_path = output_root;
    if let Some(parent) = relative.parent() {
        output_path = output_path.join(parent);
    }
    output_path = output_path.join(renamed_filename);
    
    Some(output_path)
}

/// Process and write all images
pub fn process_all_images(
    input_files: &[std::path::PathBuf],
    renamed_files: &[std::path::PathBuf],
    input_roots: &[std::path::PathBuf],
    settings: &ProcessingSettings,
    progress_callback: Option<&dyn Fn(usize, usize, &Path)>,
) -> Result<ProcessAllResult> {
    let mut processed_count = 0;
    let skipped_count = 0;
    let mut error_count = 0;
    let mut errors: Vec<String> = Vec::new();
    
    let total = input_files.len();
    
    for (i, (input_file, renamed_file)) in input_files.iter().zip(renamed_files.iter()).enumerate() {
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
            errors.push(format!("Could not calculate output path for: {}", input_file.display()));
            error_count += 1;
            continue;
        };
        
        // Create output directory if needed
        if let Some(parent) = output_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                errors.push(format!("Failed to create directory {}: {}", parent.display(), e));
                error_count += 1;
                continue;
            }
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
