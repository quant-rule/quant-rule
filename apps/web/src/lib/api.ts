/** The one transport seam: every screen calls through here. */
export const API_BASE = "/api/v1";
export const PROFILE_HEADER = "X-QuantRule-Profile";
export const DEFAULT_PROFILE = "default";

import {
  beginRetry,
  clearRetry,
  endRetry,
  finalMessage,
  retryDelayMs,
  retryMessage,
  retryableMethod,
  retryableStatus,
} from "./transport";
import type { DatasetRecord } from "./types";

let activeProfile = DEFAULT_PROFILE;
let activeToken: string | null = null;

/** Select the profile every call carries. */
export function setProfile(profile: string): void {
  activeProfile = profile;
}

/** The profile every call carries. */
export function currentProfile(): string {
  return activeProfile;
}

/** Select the bearer token every call carries, or clear it with null. */
export function setToken(token: string | null): void {
  const trimmed = token?.trim() ?? "";
  activeToken = trimmed === "" ? null : trimmed;
}

/** The bearer token every call carries, or null when the gateway is open. */
export function currentToken(): string | null {
  return activeToken;
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
  if (activeToken !== null) {
    headers.set("authorization", `Bearer ${activeToken}`);
  }
  return headers;
}

/** A human message that keeps a stable code when the failure has one. */
export function errorText(error: unknown): string {
  if (error instanceof ApiError) {
    return `${error.code}: ${error.message}`;
  }
  return error instanceof Error ? error.message : String(error);
}

async function failure(response: Response): Promise<ApiError> {
  let body: ErrorBody = {};
  try {
    body = (await response.json()) as ErrorBody;
  } catch {
    // A proxy or a crash may answer with no JSON; the status is still useful.
  }
  return new ApiError(
    response.status,
    body.error?.code ?? "internal",
    body.error?.message ?? response.statusText,
  );
}

async function parse<T>(response: Response): Promise<T> {
  if (!response.ok) {
    throw await failure(response);
  }
  return (await response.json()) as T;
}

function sleep(milliseconds: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

function aborted(error: unknown): boolean {
  return (
    typeof error === "object" &&
    error !== null &&
    "name" in error &&
    (error as { name?: unknown }).name === "AbortError"
  );
}

/**
 * Exchange one request, retrying a GET within the fixed budget.
 *
 * A transport failure, `408`, `429`, `502`, `503` and `504` retry; an abort, a
 * mutation, a streaming upload and every other status do not. The response is
 * returned unparsed, so an invalid body is a parse failure outside the loop and
 * is never retried.
 */
async function exchange(path: string, init: RequestInit): Promise<Response> {
  const retryable = retryableMethod(init.method);
  const headers = requestHeaders(init.headers);
  let attempt = 0;

  for (;;) {
    attempt += 1;
    let response: Response;
    try {
      response = await fetch(`${API_BASE}${path}`, { ...init, headers });
    } catch (error) {
      if (!retryable || aborted(error)) {
        throw error;
      }
      const delay = retryDelayMs(attempt);
      if (delay === null) {
        endRetry(finalMessage(attempt, null));
        throw error;
      }
      beginRetry(retryMessage(attempt, null));
      await sleep(delay);
      continue;
    }

    if (!retryable || !retryableStatus(response.status)) {
      if (retryable) {
        clearRetry();
      }
      return response;
    }

    const delay = retryDelayMs(attempt);
    if (delay === null) {
      endRetry(finalMessage(attempt, response.status));
      return response;
    }
    beginRetry(retryMessage(attempt, response.status));
    await sleep(delay);
  }
}

/**
 * Call one endpoint.
 *
 * The response is parsed as JSON; an error envelope becomes an `ApiError` with
 * its stable code, so a screen can branch on the code and print the message. A
 * GET that fails transiently is retried by [`exchange`]; the bytes a screen
 * receives are the last attempt's bytes, never a retry marker.
 */
export async function call<T>(path: string, init: RequestInit = {}): Promise<T> {
  const response = await exchange(path, init);
  return parse<T>(response);
}

/**
 * Call one endpoint that answers with no body.
 *
 * A `204` is the success shape for a delete; it has no JSON to parse, so it
 * cannot go through [`call`]. A failure still becomes an [`ApiError`] with its
 * stable code.
 */
export async function callVoid(path: string, init: RequestInit = {}): Promise<void> {
  const response = await exchange(path, init);
  if (!response.ok) {
    throw await failure(response);
  }
}

/**
 * Call one endpoint that answers with bytes.
 *
 * Artifact downloads are not JSON, so they cannot go through [`call`]. The
 * response is still checked for the shared error envelope.
 */
export async function fetchBytes(path: string, init: RequestInit = {}): Promise<ArrayBuffer> {
  const response = await exchange(path, init);
  if (!response.ok) {
    throw await failure(response);
  }
  return response.arrayBuffer();
}

/**
 * Upload a CSV as a dataset.
 *
 * The `File` is the request body, so the browser streams it and the client never
 * constructs a `data_version`. The server owns the limit and the hash.
 */
export async function uploadDataset(
  file: File,
  options: { tolerant?: boolean } = {},
): Promise<DatasetRecord> {
  const query = new URLSearchParams({ name: file.name });
  const path = options.tolerant === true ? "/catalog/datasets/tolerant" : "/catalog/datasets";
  const response = await fetch(`${API_BASE}${path}?${query.toString()}`, {
    method: "POST",
    headers: requestHeaders(undefined, "text/csv"),
    body: file,
  });
  return parse<DatasetRecord>(response);
}
