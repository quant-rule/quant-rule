/**
 * Artifact download and its integrity check.
 *
 * The run record names each artifact's size and SHA-256. The browser reads the
 * bytes, hashes them, and refuses a body that does not match, so a truncated or
 * substituted download is never presented as the run's result. The hash is a
 * transport check, not a model measurement.
 */

import { fetchBytes } from "./api";
import type { ArtifactRef, Digest } from "./types";

/** Fetch one artifact's bytes without interpreting them. */
export async function artifactBytes(runId: Digest, name: string): Promise<ArrayBuffer> {
  const path = name
    .split("/")
    .map((segment) => encodeURIComponent(segment))
    .join("/");
  return fetchBytes(`/runs/${encodeURIComponent(runId)}/artifacts/${path}`);
}

/** The lowercase `sha256:<hex>` digest of one byte body. */
export async function sha256Hex(bytes: BufferSource): Promise<Digest> {
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  const hex = [...new Uint8Array(digest)]
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
  return `sha256:${hex}`;
}

/** Fetch one artifact and verify it against the run record. */
export async function downloadArtifact(
  runId: Digest,
  artifact: ArtifactRef,
): Promise<ArrayBuffer> {
  const bytes = await artifactBytes(runId, artifact.name);
  const digest = await sha256Hex(bytes);
  if (digest !== artifact.sha256 || bytes.byteLength !== artifact.bytes) {
    throw new Error(
      `artifact ${artifact.name} does not match the run record; the download was refused`,
    );
  }
  return bytes;
}

/** Offer verified bytes to the browser as a file download. */
export function saveArtifact(bytes: ArrayBuffer, name: string): void {
  const url = URL.createObjectURL(new Blob([bytes]));
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = name.split("/").pop() ?? "artifact";
  // The anchor must be in the document for a headless browser to follow it;
  // revoking the URL or removing the anchor in the same task can cancel the
  // download before it starts.
  document.body.appendChild(anchor);
  anchor.click();
  setTimeout(() => {
    anchor.remove();
    URL.revokeObjectURL(url);
  }, 0);
}
