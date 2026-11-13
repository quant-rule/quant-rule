import { call, errorText, setProfile } from "./api";
import { chooseProfile } from "./profiles";
import type { ProfileSummary } from "./types";

const STORAGE_KEY = "quantrule.profile";

function stored(): string | null {
  try {
    return localStorage.getItem(STORAGE_KEY);
  } catch {
    // A browser in a restricted mode has no storage; the default still works.
    return null;
  }
}

function remember(profile: string): void {
  try {
    localStorage.setItem(STORAGE_KEY, profile);
  } catch {
    // Persistence is a convenience, not a requirement.
  }
}

const saved = stored();
if (saved !== null) {
  setProfile(saved);
}

/** What the profile switcher knows. */
export interface Profiles {
  items: ProfileSummary[];
  active: string;
  loading: boolean;
  error: string | null;
}

let current = $state<Profiles>({
  items: [],
  active: saved ?? "default",
  loading: false,
  error: null,
});

/** Read the current profiles state. */
export function profiles(): Profiles {
  return current;
}

/** Load the profile list and resolve the active id against it. */
export async function load(): Promise<void> {
  current = { ...current, loading: true, error: null };
  try {
    const items = await call<ProfileSummary[]>("/profiles");
    const choice = chooseProfile(items, current.active);
    setProfile(choice.active);
    remember(choice.active);
    current = {
      items,
      active: choice.active,
      loading: false,
      error: choice.unknown
        ? `profile.not_found: profile ${choice.active} is unknown`
        : null,
    };
  } catch (error) {
    current = { ...current, items: [], loading: false, error: errorText(error) };
  }
}

/** Select a profile for every later call. */
export async function select(profile: string): Promise<void> {
  setProfile(profile);
  remember(profile);
  current = { ...current, active: profile, error: null };
}

/** Create a profile and select it. Returns whether the server accepted it. */
export async function create(profileId: string, name: string): Promise<boolean> {
  try {
    await call<ProfileSummary>("/profiles", {
      method: "POST",
      body: JSON.stringify({ profile_id: profileId, name }),
    });
    await select(profileId);
    await load();
    return true;
  } catch (error) {
    current = { ...current, error: errorText(error) };
    return false;
  }
}
