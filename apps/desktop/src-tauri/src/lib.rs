//! The QuantRule desktop application library.
//!
//! The Tauri host registers one command per application-service operation. It
//! holds no path resolution, configuration parsing, model computation, or
//! database rule; those belong to `quantrule-app`.

mod commands;

/// Starts the desktop application.
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::open_workspace,
            commands::dataset_summary,
            commands::check_run,
            commands::run_model,
            commands::export_run_summary,
            commands::memory_status,
            commands::memory_workspaces,
            commands::memory_runs,
            commands::memory_metrics,
            commands::memory_claims,
            commands::memory_notes,
            commands::record_run,
            commands::memory_export,
            commands::memory_import,
            commands::memory_backup,
            commands::memory_restore,
        ])
        .run(tauri::generate_context!())
        .expect("the QuantRule desktop window failed to start");
}
