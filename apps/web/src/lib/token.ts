/**
 * The browser's bearer-token store.
 *
 * A remote gateway can require one shared secret. The value lives in
 * `localStorage` so a reload keeps the connection, and it never enters the URL,
 * the build, a cookie, or a rendered label. This module owns only storage; the
 * transport header is set through `setToken` in `api.ts`.
 */
const STORAGE_KEY = "quantrule.token";

/** Read the stored token, or null when none is stored or storage is blocked. */
export function storedToken(): string | null {
  try {
    const value = localStorage.getItem(STORAGE_KEY);
    return value === null || value.trim() === "" ? null : value.trim();
  } catch {
    // A browser in a restricted mode has no storage; the gateway still works
    // for this session when the operator enters the token again.
    return null;
  }
}

/**
 * Store a token, or remove it with null or a blank value.
 *
 * Returns the value that was kept, normalized the same way [`storedToken`]
 * reads it, so a caller can hand it to `setToken` without a second trim.
 */
export function rememberToken(token: string | null): string | null {
  const value = token?.trim() ?? "";
  try {
    if (value === "") {
      localStorage.removeItem(STORAGE_KEY);
    } else {
      localStorage.setItem(STORAGE_KEY, value);
    }
  } catch {
    // Persistence is a convenience; the session token still works.
  }
  return value === "" ? null : value;
}
