//! Usage: Read the frozen tray provider snapshot and report mini-window hover state.

use crate::app::tray_provider_mini::TrayProviderMiniSnapshot;
use crate::resident::ResidentState;

#[tauri::command]
#[specta::specta]
pub(crate) fn tray_provider_mini_snapshot_get(
    state: tauri::State<'_, ResidentState>,
) -> Option<TrayProviderMiniSnapshot> {
    state.tray_provider_mini_snapshot()
}

#[tauri::command]
#[specta::specta]
pub(crate) fn tray_provider_mini_window_hover_set(
    app: tauri::AppHandle,
    hovered: bool,
) -> bool {
    crate::resident::set_tray_provider_mini_window_hovered(&app, hovered);
    hovered
}
