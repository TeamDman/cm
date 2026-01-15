//! Image description tile - shows EXIF metadata of the selected image

use crate::gui::state::AppState;
use eframe::egui::ScrollArea;
use eframe::egui::{self};
use exif::In;
use exif::Tag;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// EXIF data organized by IFD (Image File Directory)
#[derive(Debug, Default)]
struct ExifData {
    /// Primary image data (IFD0)
    primary: BTreeMap<String, String>,
    /// EXIF-specific data
    exif: BTreeMap<String, String>,
    /// GPS data
    gps: BTreeMap<String, String>,
    /// Thumbnail data (IFD1)
    thumbnail: BTreeMap<String, String>,
    /// Interoperability data
    interop: BTreeMap<String, String>,
    /// Maker notes (often proprietary)
    maker_notes: BTreeMap<String, String>,
}

impl ExifData {
    fn is_empty(&self) -> bool {
        self.primary.is_empty()
            && self.exif.is_empty()
            && self.gps.is_empty()
            && self.thumbnail.is_empty()
            && self.interop.is_empty()
            && self.maker_notes.is_empty()
    }
}

/// Read EXIF data from an image file
fn read_exif_data(path: &Path) -> Result<ExifData, String> {
    let file = File::open(path).map_err(|e| format!("Failed to open file: {e}"))?;
    let mut bufreader = BufReader::new(file);

    let exif_reader = exif::Reader::new();
    let exif = exif_reader
        .read_from_container(&mut bufreader)
        .map_err(|e| format!("Failed to read EXIF: {e}"))?;

    let mut data = ExifData::default();

    for field in exif.fields() {
        let tag_name = field.tag.to_string();
        let value = field.display_value().with_unit(&exif).to_string();

        // Categorize by IFD
        match field.ifd_num {
            In::PRIMARY => {
                data.primary.insert(tag_name, value);
            }
            In::THUMBNAIL => {
                data.thumbnail.insert(tag_name, value);
            }
            _ => {
                // Check for specific tag categories
                if is_gps_tag(field.tag) {
                    data.gps.insert(tag_name, value);
                } else if is_exif_tag(field.tag) {
                    data.exif.insert(tag_name, value);
                } else if is_interop_tag(field.tag) {
                    data.interop.insert(tag_name, value);
                } else if is_maker_note_tag(field.tag) {
                    data.maker_notes.insert(tag_name, value);
                } else {
                    // Default to primary for unknown IFDs
                    data.primary.insert(tag_name, value);
                }
            }
        }
    }

    Ok(data)
}

/// Check if a tag is GPS-related
fn is_gps_tag(tag: Tag) -> bool {
    matches!(
        tag,
        Tag::GPSVersionID
            | Tag::GPSLatitudeRef
            | Tag::GPSLatitude
            | Tag::GPSLongitudeRef
            | Tag::GPSLongitude
            | Tag::GPSAltitudeRef
            | Tag::GPSAltitude
            | Tag::GPSTimeStamp
            | Tag::GPSSatellites
            | Tag::GPSStatus
            | Tag::GPSMeasureMode
            | Tag::GPSDOP
            | Tag::GPSSpeedRef
            | Tag::GPSSpeed
            | Tag::GPSTrackRef
            | Tag::GPSTrack
            | Tag::GPSImgDirectionRef
            | Tag::GPSImgDirection
            | Tag::GPSMapDatum
            | Tag::GPSDestLatitudeRef
            | Tag::GPSDestLatitude
            | Tag::GPSDestLongitudeRef
            | Tag::GPSDestLongitude
            | Tag::GPSDestBearingRef
            | Tag::GPSDestBearing
            | Tag::GPSDestDistanceRef
            | Tag::GPSDestDistance
            | Tag::GPSProcessingMethod
            | Tag::GPSAreaInformation
            | Tag::GPSDateStamp
            | Tag::GPSDifferential
    )
}

/// Check if a tag is EXIF-specific
fn is_exif_tag(tag: Tag) -> bool {
    matches!(
        tag,
        Tag::ExposureTime
            | Tag::FNumber
            | Tag::ExposureProgram
            | Tag::SpectralSensitivity
            | Tag::PhotographicSensitivity
            | Tag::SensitivityType
            | Tag::StandardOutputSensitivity
            | Tag::RecommendedExposureIndex
            | Tag::ISOSpeed
            | Tag::ISOSpeedLatitudeyyy
            | Tag::ISOSpeedLatitudezzz
            | Tag::ExifVersion
            | Tag::DateTimeOriginal
            | Tag::DateTimeDigitized
            | Tag::OffsetTime
            | Tag::OffsetTimeOriginal
            | Tag::OffsetTimeDigitized
            | Tag::ComponentsConfiguration
            | Tag::CompressedBitsPerPixel
            | Tag::ShutterSpeedValue
            | Tag::ApertureValue
            | Tag::BrightnessValue
            | Tag::ExposureBiasValue
            | Tag::MaxApertureValue
            | Tag::SubjectDistance
            | Tag::MeteringMode
            | Tag::LightSource
            | Tag::Flash
            | Tag::FocalLength
            | Tag::SubjectArea
            | Tag::MakerNote
            | Tag::UserComment
            | Tag::SubSecTime
            | Tag::SubSecTimeOriginal
            | Tag::SubSecTimeDigitized
            | Tag::FlashpixVersion
            | Tag::ColorSpace
            | Tag::PixelXDimension
            | Tag::PixelYDimension
            | Tag::RelatedSoundFile
            | Tag::FlashEnergy
            | Tag::FocalPlaneXResolution
            | Tag::FocalPlaneYResolution
            | Tag::FocalPlaneResolutionUnit
            | Tag::SubjectLocation
            | Tag::ExposureIndex
            | Tag::SensingMethod
            | Tag::FileSource
            | Tag::SceneType
            | Tag::CFAPattern
            | Tag::CustomRendered
            | Tag::ExposureMode
            | Tag::WhiteBalance
            | Tag::DigitalZoomRatio
            | Tag::FocalLengthIn35mmFilm
            | Tag::SceneCaptureType
            | Tag::GainControl
            | Tag::Contrast
            | Tag::Saturation
            | Tag::Sharpness
            | Tag::DeviceSettingDescription
            | Tag::SubjectDistanceRange
            | Tag::ImageUniqueID
            | Tag::CameraOwnerName
            | Tag::BodySerialNumber
            | Tag::LensSpecification
            | Tag::LensMake
            | Tag::LensModel
            | Tag::LensSerialNumber
            | Tag::Gamma
    )
}

/// Check if a tag is interoperability-related
fn is_interop_tag(tag: Tag) -> bool {
    matches!(
        tag,
        Tag::InteroperabilityIndex | Tag::InteroperabilityVersion
    )
}

/// Check if a tag is a maker note
fn is_maker_note_tag(tag: Tag) -> bool {
    matches!(tag, Tag::MakerNote)
}

/// Draw the image description tile UI
pub fn draw_image_description_tile(ui: &mut egui::Ui, state: &AppState) {
    // Check if we have a selected image
    let Some(ref selected_path) = state.selected_input_file else {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.label("No image selected");
            ui.add_space(8.0);
            ui.label("Select an image from the Input Images panel to view its EXIF data.");
        });
        return;
    };

    // Show the filename
    ui.horizontal(|ui| {
        ui.label("📷");
        let filename = selected_path
            .file_name().map_or_else(|| selected_path.display().to_string(), |s| s.to_string_lossy().to_string());
        let response = ui.strong(&filename);
        response.on_hover_text(selected_path.display().to_string());
    });
    ui.separator();

    // Try to read EXIF data
    match read_exif_data(selected_path) {
        Ok(exif_data) => {
            if exif_data.is_empty() {
                ui.label("No EXIF metadata found in this image.");
            } else {
                draw_exif_tree(ui, &exif_data);
            }
        }
        Err(e) => {
            ui.colored_label(egui::Color32::YELLOW, format!("⚠ {e}"));
            ui.add_space(8.0);
            ui.label("This image may not contain EXIF data, or the format is not supported.");
        }
    }
}

/// Draw the EXIF data as a collapsible tree
fn draw_exif_tree(ui: &mut egui::Ui, data: &ExifData) {
    ScrollArea::both()
        .id_salt("exif_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // Primary/Image info
            if !data.primary.is_empty() {
                draw_exif_section(ui, "📄 Image Info", &data.primary, true);
            }

            // EXIF data
            if !data.exif.is_empty() {
                draw_exif_section(ui, "📸 Camera Settings", &data.exif, true);
            }

            // GPS data
            if !data.gps.is_empty() {
                draw_exif_section(ui, "🌍 GPS Location", &data.gps, false);
            }

            // Thumbnail info
            if !data.thumbnail.is_empty() {
                draw_exif_section(ui, "🖼 Thumbnail", &data.thumbnail, false);
            }

            // Interoperability
            if !data.interop.is_empty() {
                draw_exif_section(ui, "🔗 Interoperability", &data.interop, false);
            }

            // Maker notes (usually collapsed as they can be large/binary)
            if !data.maker_notes.is_empty() {
                draw_exif_section(ui, "🏭 Maker Notes", &data.maker_notes, false);
            }
        });
}

/// Draw a collapsible section of EXIF data
fn draw_exif_section(
    ui: &mut egui::Ui,
    title: &str,
    entries: &BTreeMap<String, String>,
    default_open: bool,
) {
    egui::CollapsingHeader::new(format!("{} ({})", title, entries.len()))
        .default_open(default_open)
        .show(ui, |ui| {
            egui::Grid::new(format!("exif_grid_{title}"))
                .num_columns(2)
                .striped(true)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    for (tag, value) in entries {
                        ui.label(tag);

                        // Truncate very long values for display
                        let display_value = if value.len() > 100 {
                            format!("{}...", &value[..100])
                        } else {
                            value.clone()
                        };

                        let response = ui.label(&display_value);
                        if value.len() > 100 {
                            response.on_hover_text(value);
                        }

                        ui.end_row();
                    }
                });
        });
}
