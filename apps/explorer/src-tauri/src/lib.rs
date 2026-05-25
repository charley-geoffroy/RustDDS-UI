mod bench;
mod dds_service;
mod dto;
mod registry;

use std::sync::Arc;

use dds_backend::rustdds::RustDdsBackend;
use dds_backend::DdsBackend;
use dds_service::{DdsService, EmbeddedDdsService};
use dto::RegistrySnapshot;
use serde::Serialize;
use tauri::{Manager, State};

pub const DOMAIN_ID: u16 = 0;

type ServiceHandle = Arc<dyn DdsService>;

#[derive(Serialize)]
struct VersionInfo {
    app: &'static str,
    backend_name: &'static str,
    backend_version: &'static str,
}

#[tauri::command]
fn get_version() -> VersionInfo {
    VersionInfo {
        app: env!("CARGO_PKG_VERSION"),
        backend_name: RustDdsBackend::name(),
        backend_version: RustDdsBackend::version(),
    }
}

#[tauri::command]
fn list_state(service: State<'_, ServiceHandle>) -> RegistrySnapshot {
    service.snapshot()
}

#[tauri::command]
fn subscribe_topic(
    service: State<'_, ServiceHandle>,
    topic_name: String,
    type_name: String,
) -> Result<(), String> {
    service
        .subscribe(&topic_name, &type_name)
        .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
fn unsubscribe_topic(
    service: State<'_, ServiceHandle>,
    topic_name: String,
) -> Result<(), String> {
    service
        .unsubscribe(&topic_name)
        .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
fn import_bench_csv(path: String) -> Result<bench::BenchReport, String> {
    bench::import_bench_csv(std::path::Path::new(&path)).map_err(|e| format!("{e:#}"))
}

#[tauri::command]
fn parse_bench_csv(content: String) -> Result<bench::BenchReport, String> {
    bench::parse_bench_csv_str(&content).map_err(|e| format!("{e:#}"))
}

#[tauri::command]
fn parse_bench_pair(
    pub_content: String,
    sub_content: String,
) -> Result<bench::PairReport, String> {
    bench::parse_bench_pair_str(&pub_content, &sub_content).map_err(|e| format!("{e:#}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let service = EmbeddedDdsService::<RustDdsBackend>::start(
                DOMAIN_ID,
                app.handle().clone(),
            )?;
            let handle: ServiceHandle = Arc::new(service);
            app.manage(handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_version,
            list_state,
            subscribe_topic,
            unsubscribe_topic,
            import_bench_csv,
            parse_bench_csv,
            parse_bench_pair
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
