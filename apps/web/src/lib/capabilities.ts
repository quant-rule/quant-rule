/**
 * The browser's command coverage.
 *
 * The shell registry is the command surface; this table names the screen and
 * control that reaches each command's use case in the browser. The parity test
 * compares it with the shared command fixture, so a command added to the shell
 * without a browser control fails the suite instead of drifting silently.
 *
 * `help` is the one presentation-only command; the browser renders the
 * reference from this same table.
 */

/** One screen in the browser client. */
export const SECTIONS = [
  { id: "connection", label: "Connection", hint: "Gateway and services" },
  { id: "profiles", label: "Profiles", hint: "Research lines" },
  { id: "datasets", label: "Datasets", hint: "Candles by content" },
  { id: "configs", label: "Configs", hint: "Hypotheses and runs" },
  { id: "runs", label: "Runs", hint: "Queued and finished" },
  { id: "inspect", label: "Inspect", hint: "Candles, ladders, events" },
  { id: "simulation", label: "Simulation", hint: "Chart-first research" },
  { id: "help", label: "Help", hint: "Command reference" },
] as const;

/** The id of one browser screen. */
export type Section = (typeof SECTIONS)[number]["id"];

/** One command and the browser control that reaches it. */
export interface CommandCapability {
  /** The registered shell command name. */
  command: string;
  /** The screen that owns the use case. */
  section: Section;
  /** The control a human uses. */
  control: string;
}

/** Every registered command, in the order the shell lists it. */
export const COMMANDS: CommandCapability[] = [  { command: "health", section: "connection", control: "Health report and service table" },
  { command: "dataset list", section: "datasets", control: "Dataset table" },
  { command: "dataset show", section: "datasets", control: "Dataset record detail" },
  { command: "dataset upload", section: "datasets", control: "Upload form" },
  { command: "config list", section: "configs", control: "Config table" },
  { command: "config show", section: "configs", control: "Config record detail" },
  { command: "run submit", section: "configs", control: "Submit action" },
  { command: "run status", section: "runs", control: "Run detail view" },
  { command: "run list", section: "runs", control: "Runs table" },
  { command: "run fetch", section: "runs", control: "Verified artifact download" },
  { command: "run cancel", section: "runs", control: "Cancel action" },
  { command: "inspect candles", section: "inspect", control: "Candles inspection" },
  { command: "inspect timeframes", section: "inspect", control: "Timeframe ladder inspection" },
  { command: "inspect events", section: "inspect", control: "Event batch inspection" },
  { command: "verify", section: "runs", control: "Verify action" },
  { command: "profile list", section: "profiles", control: "Profile table" },
  { command: "profile create", section: "profiles", control: "Create profile form" },
  { command: "profile use", section: "profiles", control: "Profile switcher" },
  { command: "simulation list", section: "simulation", control: "Simulation table" },
  { command: "simulation create", section: "simulation", control: "Create simulation form" },
  { command: "simulation windows", section: "simulation", control: "Window table" },
  { command: "simulation mark", section: "simulation", control: "Mark form" },
  { command: "simulation marks", section: "simulation", control: "Marks table" },
  { command: "simulation oracle", section: "simulation", control: "Oracle result" },
  { command: "simulation check", section: "simulation", control: "Check action" },
  { command: "simulation decide", section: "simulation", control: "Decision action" },
  { command: "help", section: "help", control: "Command reference table" },
];

/**
 * The commands the shell registers that the browser does not reach.
 *
 * The parity test calls this with the fixture's command list. An empty result
 * is the parity condition; a name here is a capability gap, not a warning.
 */
export function missingCommands(
  browser: readonly string[],
  shell: readonly string[],
): string[] {
  return shell.filter((name) => !browser.includes(name));
}
