// Usage: Keep the tray mini renderer separate from the main application bootstrap.

export const TRAY_PROVIDER_MINI_WINDOW_MODE = "tray-provider-mini";

export function isTrayProviderMiniWindow(search: string): boolean {
  return new URLSearchParams(search).get("window") === TRAY_PROVIDER_MINI_WINDOW_MODE;
}
