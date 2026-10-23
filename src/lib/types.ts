// Mirrors the Rust types in crates/core (serde snake_case).

export type Launcher = "riot" | "garena";
export type Game = "league" | "valorant" | "delta_force";

export interface Account {
  id: string;
  launcher: Launcher;
  label: string;
  username: string;
  note: string;
  default_game: Game | null;
  last_used: number | null;
  has_password: boolean;
}

export interface AccountInput {
  id?: string | null;
  launcher: Launcher;
  label: string;
  username: string;
  password: string;
  note: string;
  default_game: Game | null;
}

export interface RelPoint {
  x: number;
  y: number;
}

export interface RiotSettings {
  client_path: string;
  username_point: RelPoint | null;
  ready_delay_ms: number;
}

export interface GarenaSettings {
  launcher_path: string;
  login_tab_point: RelPoint | null;
  username_point: RelPoint | null;
  launch_button_point: RelPoint | null;
  ready_delay_ms: number;
  launch_delay_ms: number;
  session_paths: string[];
  game_processes: string[];
}

export interface Settings {
  auto_submit: boolean;
  ask_game_each_time: boolean;
  close_to_tray: boolean;
  start_minimized: boolean;
  run_at_startup: boolean;
  hotkeys_enabled: boolean;
  riot: RiotSettings;
  garena: GarenaSettings;
}

export interface ActiveState {
  riot: string | null;
  garena: string | null;
}

export interface Bootstrap {
  accounts: Account[];
  settings: Settings;
  active: ActiveState;
  load_error: string | null;
  supported: boolean;
  version: string;
  data_dir: string;
  /** Set once after importing data from the old Riot Account Manager v3. */
  import_notice: {
    report: { accounts_added: number; riot_path_imported: boolean } | null;
    error: string | null;
  } | null;
}

export interface LauncherStatus {
  configured: boolean;
  client_running: boolean;
  game_running: boolean;
}

export interface StatusView {
  riot: LauncherStatus;
  garena: LauncherStatus;
  active: ActiveState;
  busy: boolean;
}

export type Step =
  | "closing"
  | "launching"
  | "waiting_window"
  | "signing_in"
  | "starting_game"
  | "done";

export interface Progress {
  step: Step;
  message: string;
}

export interface Outcome {
  signed_in: boolean;
  game_started: boolean;
  notes: string[];
}

export interface ImportResult {
  added: number;
  accounts: Account[];
}

export const GAMES: Record<Game, { name: string; short: string; launcher: Launcher }> = {
  league: { name: "League of Legends", short: "LoL", launcher: "riot" },
  valorant: { name: "VALORANT", short: "VAL", launcher: "riot" },
  delta_force: { name: "Delta Force", short: "DF", launcher: "garena" },
};

export const LAUNCHERS: Record<Launcher, { name: string; tab: string }> = {
  riot: { name: "Riot Client", tab: "Riot" },
  garena: { name: "Garena", tab: "Delta Force" },
};

/** Riot first, then Garena, in list order — matches hotkeys Ctrl+Alt+1..9. */
export function hotkeyNumber(accounts: Account[], id: string): number | null {
  const order = [
    ...accounts.filter((a) => a.launcher === "riot"),
    ...accounts.filter((a) => a.launcher === "garena"),
  ];
  const i = order.findIndex((a) => a.id === id);
  return i >= 0 && i < 9 ? i + 1 : null;
}

export function displayName(a: Account): string {
  return a.label.trim() || a.username;
}
