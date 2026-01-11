use crate::cli::command::search::search_command::SearchArgs;
use crate::gui::state::AppState;
use eframe::egui::Button;
use eframe::egui::RichText;
use eframe::egui::ScrollArea;
use eframe::egui::TextEdit;
use eframe::egui::{self};
use facet_pretty::PrettyPrinter;

#[derive(Debug)]
pub enum SearchResultDisplay {
    None,
    SomeResults(String),
    NoResults,
}
impl core::fmt::Display for SearchResultDisplay {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SearchResultDisplay::None => write!(f, ""),
            SearchResultDisplay::SomeResults(s) => write!(f, "{}", s),
            SearchResultDisplay::NoResults => write!(f, "No results found."),
        }
    }
}

pub fn draw_product_search_tile(ui: &mut egui::Ui, state: &mut AppState) {
    let text = state.product_search_result_display.to_string();
    ui.vertical(|ui| {
        ui.label("Query:");
        ui.add(TextEdit::singleline(&mut state.product_search_query).desired_width(200.0));
        ui.label("SKU:");
        ui.add(TextEdit::singleline(&mut state.product_search_sku).desired_width(120.0));
        if ui.add(Button::new("Submit")).clicked() {
            // Perform search in background: spawn tokio task
            let query = state.product_search_query.clone();
            let sku = if state.product_search_sku.is_empty() {
                None
            } else {
                Some(state.product_search_sku.clone())
            };
            let tx = state.background_sender.clone();

            // Use tokio::spawn to run async search and then send result back via background channel
            let tx = tx.clone();
            tokio::spawn(async move {
                let args = SearchArgs {
                    query: if query.is_empty() { None } else { Some(query) },
                    sku,
                    no_cache: true,
                    output: crate::cli::command::search::search_command::OutputFormat::Json,
                };
                match args.search().await {
                    Ok(res) => {
                        match res.results {
                            None => {
                                let _ = tx.send(
                                    crate::gui::state::BackgroundMessage::ProductSearchResult {
                                        result_display: SearchResultDisplay::NoResults,
                                        error: None,
                                    },
                                );
                            }
                            Some(results) if results.is_empty() => {
                                let _ = tx.send(
                                    crate::gui::state::BackgroundMessage::ProductSearchResult {
                                        result_display: SearchResultDisplay::NoResults,
                                        error: None,
                                    },
                                );
                            }
                            Some(results) => {
                                // Use Facet pretty printing for readable output
                                let pretty = facet_json::to_string_pretty(&results).unwrap_or_else(
                                    |_error| {
                                        PrettyPrinter::new().with_colors(false).format(&results)
                                    },
                                );
                                let _ = tx.send(
                                    crate::gui::state::BackgroundMessage::ProductSearchResult {
                                        result_display: SearchResultDisplay::SomeResults(
                                            pretty.to_string(),
                                        ),
                                        error: None,
                                    },
                                );
                            }
                        }
                    }
                    Err(e) => {
                        let _ =
                            tx.send(crate::gui::state::BackgroundMessage::ProductSearchResult {
                                result_display: SearchResultDisplay::None,
                                error: Some(format!("Search failed: {}", e)),
                            });
                    }
                }
            });
        }
        ui.add_space(6.0);

        if ui.button("Copy").clicked() {
            ui.ctx().copy_text(text.clone());
        }

        ui.label(RichText::new("Result:").strong());

        // Height left in this column:
        let remaining = ui.available_size_before_wrap().y;

        // Make a child with exactly the remaining height
        egui::Frame::default().show(ui, |ui| {
            ui.set_min_height(remaining);
            ui.set_max_height(remaining);

            ScrollArea::vertical().show(ui, |ui| {
                ui.add(
                    TextEdit::multiline(&mut text.as_str())
                        .code_editor()
                        .desired_rows(0) // let height be driven by the container
                        .desired_width(f32::INFINITY),
                );
            });
        });
    });
}
