/**
 * Pack registry transport.
 *
 * A pack is a definition, not evidence. The list and detail carry the catalog
 * record; the download carries the canonical bytes and verifies them against
 * the record before the browser offers the file.
 */

import { call, fetchBytes } from "./api";
import { sha256Hex } from "./artifacts";
import type { Digest } from "./types";

export type PackKind = "strategy" | "portfolio" | "execution";
export type PackState = "draft" | "validated" | "frozen" | "retired";

export interface PackSummary {
  pack_id: Digest;
  kind: PackKind;
  name: string;
  version: string;
  owner: string;
  state: PackState;
  bytes: number;
  created_at: string;
  validated_at?: string;
  frozen_at?: string;
  retired_at?: string;
}

export interface PackValidationRecord {
  validator: string;
  valid: boolean;
  message?: string;
  created_at: string;
}

export interface PackLifecycleEvent {
  from?: PackState;
  to: PackState;
  action: string;
  reason?: string;
  created_at: string;
}

export interface PackRecord {
  summary: PackSummary;
  object_key: string;
  sha256: Digest;
  validations: PackValidationRecord[];
  events: PackLifecycleEvent[];
}

export interface PackFilter {
  kind?: PackKind;
  state?: PackState;
}

/** One pack's catalog path. */
export function packPath(packId: Digest): string {
  return `/catalog/packs/${encodeURIComponent(packId)}`;
}

/** The pack list path for one filter. */
export function packsPath(filter: PackFilter = {}): string {
  const query = new URLSearchParams();
  if (filter.kind !== undefined) {
    query.set("kind", filter.kind);
  }
  if (filter.state !== undefined) {
    query.set("state", filter.state);
  }
  const rendered = query.toString();
  return rendered === "" ? "/catalog/packs" : `/catalog/packs?${rendered}`;
}

/** List the profile's packs. */
export function listPacks(filter: PackFilter = {}): Promise<PackSummary[]> {
  return call<PackSummary[]>(packsPath(filter));
}

/** Read one pack record. */
export function fetchPack(packId: Digest): Promise<PackRecord> {
  return call<PackRecord>(packPath(packId));
}

/** Register one local pack document. The file is the request body. */
export function createPack(file: File): Promise<PackRecord> {
  return call<PackRecord>("/catalog/packs", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: file,
  });
}

/** Re-validate one draft pack. */
export function validatePack(packId: Digest): Promise<PackRecord> {
  return call<PackRecord>(`${packPath(packId)}/validate`, { method: "POST" });
}

/** Freeze one validated pack. */
export function freezePack(packId: Digest): Promise<PackRecord> {
  return call<PackRecord>(`${packPath(packId)}/freeze`, { method: "POST" });
}

/** Retire one frozen pack. */
export function retirePack(packId: Digest): Promise<PackRecord> {
  return call<PackRecord>(`${packPath(packId)}/retire`, { method: "POST" });
}

/** Download one pack and verify it against the catalog record. */
export async function downloadPack(record: PackRecord): Promise<ArrayBuffer> {
  const bytes = await fetchBytes(`${packPath(record.summary.pack_id)}/bytes`);
  const digest = await sha256Hex(bytes);
  if (digest !== record.sha256 || bytes.byteLength !== record.summary.bytes) {
    throw new Error("the pack bytes do not match the catalog; the download was refused");
  }
  return bytes;
}
