//! The QuantRule desktop application library.
//!
//! The Tauri host registers one command per application-service operation. It
//! holds no path resolution, configuration parsing, model computation, or
//! database rule; those belong to `quantrule-app`.

mod commands;

/// Starts the desktop application.
///
/// # Errors
///
/// Returns the Tauri runtime error when the window or the event loop cannot
/// start. The binary entry point renders an actionable message; this library
/// function does not panic.
pub fn run() -> Result<(), tauri::Error> {
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
            commands::memory_notes_for_run,
            commands::memory_metric_set,
            commands::memory_metric_delete,
            commands::memory_claim_add,
            commands::memory_claim_delete,
            commands::memory_note_add,
            commands::memory_note_update,
            commands::memory_note_delete,
            commands::record_run,
            commands::memory_export,
            commands::memory_import,
            commands::memory_backup,
            commands::memory_restore,
        ])
        .run(tauri::generate_context!())
}
