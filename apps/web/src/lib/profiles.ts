import type { ProfileSummary } from "./types";

/** The active profile and whether the server listed it. */
export interface ProfileChoice {
  active: string;
  unknown: boolean;
}

/**
 * Resolve the profile a client should use.
 *
 * A stored profile is kept even when the server no longer lists it, so the
 * server's `profile.not_found` reaches the screen instead of being hidden by a
 * silent fallback. With no stored profile, `default` is preferred; a server
 * without it uses the first listed profile.
 */
export function chooseProfile(
  items: ProfileSummary[],
  preferred: string | null,
): ProfileChoice {
  if (preferred !== null) {
    return {
      active: preferred,
      unknown: !items.some((item) => item.profile_id === preferred),
    };
  }
  const active = items.some((item) => item.profile_id === "default")
    ? "default"
    : (items[0]?.profile_id ?? "default");
  return { active, unknown: false };
}
