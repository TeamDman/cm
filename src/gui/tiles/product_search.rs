use crate::cli::command::search::search_command::OutputFormat;
use crate::cli::command::search::search_command::SearchArgs;
use crate::gui::state::AppState;
use crate::gui::state::BackgroundMessage;
use eframe::egui::Button;
use eframe::egui::RichText;
use eframe::egui::ScrollArea;
use eframe::egui::TextEdit;
use eframe::egui::{self};
use facet_pretty::PrettyPrinter;
use regex::Regex;
use std::path::Path;
use tokio::sync::mpsc::UnboundedSender;

/// Suggest search args given a filename.
/// If a six-digit SKU is found (\b(\d{6})\b) suggest a SKU search, otherwise
/// suggest a query formed by replacing hyphens with spaces and stripping numbers.
fn suggest_search(filename: &str) -> SearchArgs {
    let re_sku = Regex::new(r"\b(\d{6})\b").unwrap();
    let re_digits = Regex::new(r"\d+").unwrap();

    // Use file stem (strip extension) when possible
    let stem = Path::new(filename)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| filename.to_string());

    if let Some(cap) = re_sku.captures(&stem) {
        let sku = cap.get(1).unwrap().as_str().to_string();
        return SearchArgs {
            query: None,
            sku: Some(sku),
            no_cache: true,
            output: OutputFormat::Json,
        };
    }

    let with_spaces = stem.replace('-', " ").replace("_", " ");
    let stripped = re_digits.replace_all(&with_spaces, "").to_string();
    // Collapse whitespace and trim
    let suggestion = stripped
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();

    SearchArgs {
        query: if suggestion.is_empty() {
            Some(stem)
        } else {
            Some(suggestion)
        },
        sku: None,
        no_cache: true,
        output: OutputFormat::Json,
    }
}

// Spawn a tokio task to perform a product search and forward the result to the background channel.
fn spawn_product_search(tx: UnboundedSender<BackgroundMessage>, args: SearchArgs) {
    tokio::spawn(async move {
        match args.search().await {
            Ok(res) => {
                // Prettify once on the background thread and send both the parsed struct and the prettified string
                // Format as json first, fallback to facet_pretty if that fails
                let pretty = facet_json::to_string_pretty(&res.results)
                    .unwrap_or(PrettyPrinter::new().with_colors(false).format(&res.results));
                let _ = tx.send(BackgroundMessage::ProductSearchResult {
                    result: Some(res),
                    pretty: Some(pretty),
                    error: None,
                });
            }
            Err(e) => {
                let _ = tx.send(BackgroundMessage::ProductSearchResult {
                    result: None,
                    pretty: None,
                    error: Some(format!("Search failed: {}", e)),
                });
            }
        }
    });
}

pub fn draw_product_search_tile(ui: &mut egui::Ui, state: &mut AppState) {
    // Keep a cloned copy of the prettified JSON for read-only display
    let pretty_text = state.product_search_result_pretty.clone();

    ui.vertical(|ui| {
        ui.label("Query:");
        let query_resp =
            ui.add(TextEdit::singleline(&mut state.product_search_query).desired_width(200.0));
        // Typing while suggestion is active disables the suggestion
        if query_resp.changed() && state.product_search_use_suggestion {
            state.product_search_use_suggestion = false;
        }
        // Submit on Enter
        if query_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            let query = state.product_search_query.clone();
            let sku = if state.product_search_sku.is_empty() {
                None
            } else {
                Some(state.product_search_sku.clone())
            };
            let tx = state.background_sender.clone();
            let args = SearchArgs {
                query: if query.is_empty() { None } else { Some(query) },
                sku,
                no_cache: true,
                output: OutputFormat::Json,
            };
            spawn_product_search(tx, args);
        }

        ui.label("SKU:");
        let sku_resp =
            ui.add(TextEdit::singleline(&mut state.product_search_sku).desired_width(120.0));
        if sku_resp.changed() && state.product_search_use_suggestion {
            state.product_search_use_suggestion = false;
        }
        if sku_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            let query = state.product_search_query.clone();
            let sku = if state.product_search_sku.is_empty() {
                None
            } else {
                Some(state.product_search_sku.clone())
            };
            let tx = state.background_sender.clone();
            let args = SearchArgs {
                query: if query.is_empty() { None } else { Some(query) },
                sku,
                no_cache: true,
                output: OutputFormat::Json,
            };
            spawn_product_search(tx, args);
        }

        // Show suggested query for the selected item, if any
        if let Some(ref selected_path) = state.selected_input_file {
            if let Some(filename) = selected_path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
            {
                let suggestion = suggest_search(&filename);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Suggested:").strong());
                    if let Some(sku) = &suggestion.sku {
                        ui.label(format!("SKU: {}", sku));
                    } else if let Some(q) = &suggestion.query {
                        ui.label(q);
                    }

                    // Checkbox to enable/disable using the suggested values; when checked, populate fields
                    if ui
                        .checkbox(&mut state.product_search_use_suggestion, "Use suggested")
                        .changed()
                    {
                        if state.product_search_use_suggestion {
                            if let Some(s) = &suggestion.sku {
                                state.product_search_sku = s.clone();
                            }
                            if let Some(q) = &suggestion.query {
                                state.product_search_query = q.clone();
                            }
                        }
                    }
                });
            }
        }

        if ui.add(Button::new("Submit")).clicked() {
            // Perform search in background: spawn tokio task
            let query = state.product_search_query.clone();
            let sku = if state.product_search_sku.is_empty() {
                None
            } else {
                Some(state.product_search_sku.clone())
            };
            let tx = state.background_sender.clone();

            let args = SearchArgs {
                query: if query.is_empty() { None } else { Some(query) },
                sku,
                no_cache: true,
                output: OutputFormat::Json,
            };
            spawn_product_search(tx, args);
        }

        ui.add_space(6.0);

        if ui.button("Copy").clicked() {
            ui.ctx().copy_text(pretty_text.clone());
        }

        ui.label(RichText::new("Pretty results:").strong());

        // Height left in this column:
        let remaining = ui.available_size_before_wrap().y;

        // Make a child with exactly the remaining height and show both a pretty listing and an expandable raw text area
        egui::Frame::default().show(ui, |ui| {
            ui.set_min_height(remaining);
            ui.set_max_height(remaining);

            ScrollArea::vertical().show(ui, |ui| {
                // Pretty listing: name and price per item
                if let Some(ref raw) = state.product_search_result_raw {
                    if let Some(results) = &raw.results {
                        for item in results {
                            let name = item.name.as_deref().unwrap_or("<no name>");
                            let price =
                                item.price.as_ref().map(|p| p.0.clone()).unwrap_or_default();
                            ui.horizontal(|ui| {
                                ui.label(name);
                                ui.add_space(6.0);
                                ui.label(RichText::new(price).monospace());
                            });
                        }
                    } else {
                        ui.label("No results");
                    }
                } else {
                    ui.label("No results");
                }

                // Raw prettified JSON in an expando
                egui::CollapsingHeader::new("Raw response").show(ui, |ui| {
                    let text = pretty_text.clone();
                    ui.add(
                        TextEdit::multiline(&mut text.as_str())
                            .code_editor()
                            .desired_rows(10)
                            .desired_width(f32::INFINITY),
                    );
                });
            });
        });
    });
}
