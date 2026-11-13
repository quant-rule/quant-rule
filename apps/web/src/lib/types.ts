/** A `sha256:<64 hex>` content digest. */
export type Digest = string;

export interface ServiceHealth {
  name: string;
  state: "healthy" | "degraded" | "down";
  detail: string | null;
}

export interface HealthReport {
  v: string;
  state: "healthy" | "degraded" | "down";
  services: ServiceHealth[];
  queue_depth: number;
  workers: number;
  image_digest: Digest | null;
}

export interface VersionInfo {
  product: string;
  api: string;
  image_digest: Digest | null;
  contracts: [string, string][];
}
