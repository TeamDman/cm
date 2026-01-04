use crate::cli::json_log_behaviour::JsonLogBehaviour;
use chrono::Local;
use egui_tracing::tracing::collector::EventCollector;
use eyre::Result;
use once_cell::sync::Lazy;
use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::info;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::Directive;
use tracing_subscriber::fmt::writer::BoxMakeWriter;
use tracing_subscriber::prelude::*;
use tracing_subscriber::util::SubscriberInitExt;

// Global collector used by the GUI logs widget. Cloneable cheap handle.
static EVENT_COLLECTOR: Lazy<EventCollector> = Lazy::new(EventCollector::default);

/// Return a handle to the global EventCollector for use in GUI widgets
pub fn event_collector() -> EventCollector {
    EVENT_COLLECTOR.clone()
}

/// Generate a default JSON log filename with timestamp
pub fn default_json_log_path() -> PathBuf {
    let timestamp = Local::now().format("%Y-%m-%d_%Hh%Mm%Ss").to_string();
    PathBuf::from(format!("cm_log_{}.jsonl", timestamp))
}
pub fn init_tracing(level: impl Into<Directive>, json_behaviour: JsonLogBehaviour) -> Result<()> {
    let default_directive: Directive = level.into();
    let env_filter = EnvFilter::builder()
        .with_default_directive(default_directive.clone())
        .from_env_lossy();
    let stderr_layer = tracing_subscriber::fmt::layer()
        .with_file(cfg!(debug_assertions))
        .with_target(true)
        .with_line_number(cfg!(debug_assertions))
        .with_writer(std::io::stderr)
        .pretty()
        .without_time();

    if let Some(json_log_path) = json_behaviour.get_path() {
        // Create parent directories if they don't exist
        if let Some(parent) = json_log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = File::create(json_log_path.as_ref())?;
        let file = Arc::new(Mutex::new(file));
        let json_writer = {
            let file = Arc::clone(&file);
            BoxMakeWriter::new(move || {
                file.lock()
                    .expect("failed to lock json log file")
                    .try_clone()
                    .expect("failed to clone json log file handle")
            })
        };

        let json_format = tracing_subscriber::fmt::format().json();
        let json_layer = tracing_subscriber::fmt::layer()
            .event_format(json_format)
            .with_file(true)
            .with_target(false)
            .with_line_number(true)
            .with_writer(json_writer);

        if let Err(error) = tracing_subscriber::registry()
            .with(env_filter)
            .with(stderr_layer)
            .with(json_layer)
            .with(EVENT_COLLECTOR.clone())
            .try_init()
        {
            tracing::warn!("Failed to initialize tracing subscriber: {}", error);
            // fallback: still print to stderr so CI/test runs see the message
            eprintln!("Failed to initialize tracing subscriber: {}", error);
            return Ok(());
        }

        info!(?json_log_path, "JSON log output initialized");
    } else {
        if let Err(error) = tracing_subscriber::registry()
            .with(env_filter)
            .with(stderr_layer)
            .with(EVENT_COLLECTOR.clone())
            .try_init()
        {
            tracing::warn!("Failed to initialize tracing subscriber: {}", error);
            eprintln!("Failed to initialize tracing subscriber: {}", error);
            return Ok(());
        }
    }

    Ok(())
}
