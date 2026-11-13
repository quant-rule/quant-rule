/** The one transport seam: every screen calls through here. */
export const API_BASE = "/api/v1";
export const PROFILE_HEADER = "X-QuantRule-Profile";
export const DEFAULT_PROFILE = "default";

import type { DatasetRecord } from "./types";

let activeProfile = DEFAULT_PROFILE;

/** Select the profile every call carries. */
export function setProfile(profile: string): void {
  activeProfile = profile;
}

/** The profile every call carries. */
export function currentProfile(): string {
  return activeProfile;
}

/** The error envelope every service returns. */
interface ErrorBody {
  error?: { code?: string; message?: string };
}

/** A failed call, carrying the stable code. */
export class ApiError extends Error {
  readonly status: number;
  readonly code: string;

  constructor(status: number, code: string, message: string) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.code = code;
  }
}

function requestHeaders(init?: HeadersInit, contentType?: string): Headers {
  const headers = new Headers(init);
  if (contentType !== undefined) {
    headers.set("content-type", contentType);
  } else if (!headers.has("content-type")) {
    headers.set("content-type", "application/json");
  }
  headers.set(PROFILE_HEADER, activeProfile);
  return headers;
}

async function parse<T>(response: Response): Promise<T> {
  if (!response.ok) {
    let body: ErrorBody = {};
    try {
      body = (await response.json()) as ErrorBody;
    } catch {
      // A proxy or a crash may answer with no JSON; the status is still useful.
    }
    throw new ApiError(
      response.status,
      body.error?.code ?? "internal",
      body.error?.message ?? response.statusText,
    );
  }
  return (await response.json()) as T;
}

/**
 * Call one endpoint.
 *
 * The response is parsed as JSON; an error envelope becomes an `ApiError` with
 * its stable code, so a screen can branch on the code and print the message.
 */
export async function call<T>(path: string, init: RequestInit = {}): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`, {
    ...init,
    headers: requestHeaders(init.headers),
  });
  return parse<T>(response);
}

/**
 * Upload a CSV as a dataset.
 *
 * The `File` is the request body, so the browser streams it and the client never
 * constructs a `data_version`. The server owns the limit and the hash.
 */
export async function uploadDataset(file: File): Promise<DatasetRecord> {
  const query = new URLSearchParams({ name: file.name });
  const response = await fetch(`${API_BASE}/catalog/datasets?${query.toString()}`, {
    method: "POST",
    headers: requestHeaders(undefined, "text/csv"),
    body: file,
  });
  return parse<DatasetRecord>(response);
}
