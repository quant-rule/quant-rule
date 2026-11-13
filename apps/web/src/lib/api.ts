/** The one transport seam: every screen calls through here. */
export const API_BASE = "/api/v1";

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

/**
 * Call one endpoint.
 *
 * The response is parsed as JSON; an error envelope becomes an `ApiError` with
 * its stable code, so a screen can branch on the code and print the message.
 */
export async function call<T>(path: string, init: RequestInit = {}): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`, {
    ...init,
    headers: { "content-type": "application/json", ...(init.headers ?? {}) },
  });

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
