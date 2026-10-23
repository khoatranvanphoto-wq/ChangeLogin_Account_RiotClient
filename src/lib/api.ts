import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import { mock } from "./mock";
import type {
  Account,
  AccountInput,
  Bootstrap,
  Game,
  ImportResult,
  Launcher,
  Outcome,
  Progress,
  RelPoint,
  Settings,
  StatusView,
} from "./types";

/** True inside the Tauri webview; false in a plain browser (UI preview). */
export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const call = <T>(cmd: string, args?: Record<string, unknown>): Promise<T> =>
  isTauri ? invoke<T>(cmd, args) : mock.invoke<T>(cmd, args);

export const api = {
  bootstrap: () => call<Bootstrap>("bootstrap"),
  saveAccount: (input: AccountInput) => call<Account[]>("save_account", { input }),
  deleteAccount: (id: string) => call<Account[]>("delete_account", { id }),
  moveAccount: (id: string, delta: number) => call<Account[]>("move_account", { id, delta }),
  setDefaultGame: (id: string, game: Game | null) =>
    call<Account[]>("set_default_game", { id, game }),
  saveSettings: (settings: Settings) => call<Settings>("save_settings", { settings }),
  detectRiotPath: () => call<string | null>("detect_riot_path"),
  status: () => call<StatusView>("status"),
  switchAccount: (id: string, game: Game | null) => call<Outcome>("switch_account", { id, game }),
  signOut: (launcher: Launcher) => call<void>("sign_out", { launcher }),
  capturePoint: (launcher: Launcher, delayMs: number) =>
    call<RelPoint>("capture_point", { launcher, delayMs }),
  exportBackup: (path: string, password: string) =>
    call<number>("export_backup", { path, password }),
  importBackup: (path: string, password: string, replace: boolean) =>
    call<ImportResult>("import_backup", { path, password, replace }),
  openDataDir: () => call<void>("open_data_dir"),
  quit: () => call<void>("quit_app"),
};

export function on<T>(event: string, handler: (payload: T) => void): Promise<UnlistenFn> {
  if (!isTauri) return mock.listen(event, handler as (p: unknown) => void);
  return listen<T>(event, (e) => handler(e.payload));
}

export type { Progress };

const EXE = [{ name: "Ứng dụng", extensions: ["exe"] }];
const BACKUP = [{ name: "Sao lưu Game Account Switcher", extensions: ["gasbackup"] }];

export const pick = {
  exe: async (): Promise<string | null> => {
    if (!isTauri) return "C:\\Program Files\\Example\\Launcher.exe";
    const r = await open({ multiple: false, directory: false, filters: EXE });
    return typeof r === "string" ? r : null;
  },
  backupToSave: async (): Promise<string | null> => {
    if (!isTauri) return "C:\\Users\\me\\Desktop\\accounts.gasbackup";
    const d = new Date().toISOString().slice(0, 10);
    return (await save({ defaultPath: `tai-khoan-${d}.gasbackup`, filters: BACKUP })) ?? null;
  },
  backupToOpen: async (): Promise<string | null> => {
    if (!isTauri) return "C:\\Users\\me\\Desktop\\accounts.gasbackup";
    const r = await open({ multiple: false, directory: false, filters: BACKUP });
    return typeof r === "string" ? r : null;
  },
};

/** Tauri commands reject with a string; normalise for display. */
export function errorText(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return String(e);
}
