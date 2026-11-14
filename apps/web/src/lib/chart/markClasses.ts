/**
 * The eight keyboard mark classes.
 *
 * The keys are the capture contract: entry/exit × good/bad × long/short. The
 * oracle may also publish a neutral quality, but a neutral class is not bound
 * to a key because the human marking loop learns from contrast, not from a
 * third, ambiguous label.
 */

import type { MarkAction, MarkQuality, Side } from "./types";

/** A mark class reachable from one number key. */
export interface MarkClass {
  key: MarkClassKey;
  action: MarkAction;
  quality: Extract<MarkQuality, "good" | "bad">;
  side: Side;
  label: string;
}

/** The eight number keys that capture a mark. */
export type MarkClassKey = "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8";

/** The class table, in key order. */
export const MARK_CLASSES: readonly MarkClass[] = [
  { key: "1", action: "entry", quality: "good", side: "long", label: "Entry · good · long" },
  { key: "2", action: "entry", quality: "good", side: "short", label: "Entry · good · short" },
  { key: "3", action: "entry", quality: "bad", side: "long", label: "Entry · bad · long" },
  { key: "4", action: "entry", quality: "bad", side: "short", label: "Entry · bad · short" },
  { key: "5", action: "exit", quality: "good", side: "long", label: "Exit · good · long" },
  { key: "6", action: "exit", quality: "good", side: "short", label: "Exit · good · short" },
  { key: "7", action: "exit", quality: "bad", side: "long", label: "Exit · bad · long" },
  { key: "8", action: "exit", quality: "bad", side: "short", label: "Exit · bad · short" },
];

/** The class for a key, or null when the key is not a mark class. */
export function markClassForKey(key: string): MarkClass | null {
  return MARK_CLASSES.find((markClass) => markClass.key === key) ?? null;
}

/** The words for any mark, including an oracle's neutral quality. */
export function markClassLabel(mark: {
  action: MarkAction;
  quality: MarkQuality;
  side: Side;
}): string {
  const action = mark.action === "entry" ? "Entry" : "Exit";
  const quality = mark.quality.charAt(0).toUpperCase() + mark.quality.slice(1);
  const side = mark.side.charAt(0).toUpperCase() + mark.side.slice(1);
  return `${action} · ${quality} · ${side}`;
}
