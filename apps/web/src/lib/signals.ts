/**
 * Signal artifact transport.
 *
 * The list and detail carry metadata only. The document is a verified download;
 * the browser never parses a signal row, derives an action, or computes a
 * return. The strategy service owns computation.
 */

import { call, fetchBytes } from "./api";
import { sha256Hex } from "./artifacts";
import type { Digest } from "./types";

export interface ArtifactSummary {
  artifact_id: Digest;
  kind: "signal";
  bytes: number;
  created_at: string;
}

export interface ArtifactRecord {
  summary: ArtifactSummary;
  object_key: string;
  sha256: Digest;
}

export interface SignalRequest {
  model_artifact_id: Digest;
  data_version: Digest;
  config_id: string;
  strategy_pack_id: Digest;
}

export interface SignalResponse {
  schema: string;
  artifact: ArtifactRecord;
  created: boolean;
}

/** The signal list path. */
export function signalsPath(limit?: number): string {
  return limit === undefined ? "/signals" : `/signals?limit=${limit}`;
}

/** One signal's metadata path. */
export function signalPath(signalId: Digest): string {
  return `/signals/${encodeURIComponent(signalId)}`;
}

/** List the profile's signal metadata. */
export function listSignals(limit?: number): Promise<ArtifactRecord[]> {
  return call<ArtifactRecord[]>(signalsPath(limit));
}

/** Read one signal record. */
export function fetchSignal(signalId: Digest): Promise<ArtifactRecord> {
  return call<ArtifactRecord>(signalPath(signalId));
}

/** Ask the strategy service for one signal artifact. */
export function createSignal(request: SignalRequest): Promise<SignalResponse> {
  return call<SignalResponse>("/signals", {
    method: "POST",
    body: JSON.stringify(request),
  });
}

/** Download one signal and verify it against the catalog record. */
export async function downloadSignal(record: ArtifactRecord): Promise<ArrayBuffer> {
  const bytes = await fetchBytes(`${signalPath(record.summary.artifact_id)}/bytes`);
  const digest = await sha256Hex(bytes);
  if (digest !== record.sha256 || bytes.byteLength !== record.summary.bytes) {
    throw new Error("the signal bytes do not match the catalog; the download was refused");
  }
  return bytes;
}
