import { invoke } from "@tauri-apps/api/core";

import { mockInvoke } from "./mock";

/** Whether the page is running inside the Tauri webview. */
export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/** Whether calls should use the deterministic browser fixture backend. */
export function mockMode(): boolean {
  return import.meta.env.VITE_MOCK_IPC === "1" || !isTauri();
}

/**
 * Calls one application-service command.
 *
 * The browser fixture backend exists so the frontend and Playwright can run
 * without a native window. The desktop build always uses the Tauri IPC path.
 */
export async function call<T>(
  command: string,
  args: Record<string, unknown> = {},
): Promise<T> {
  if (mockMode()) {
    return mockInvoke<T>(command, args);
  }
  return invoke<T>(command, args);
}
