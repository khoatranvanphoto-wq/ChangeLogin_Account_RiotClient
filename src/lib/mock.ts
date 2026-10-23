// In-browser stand-in for the Rust backend so the UI can be previewed with
// `npm run dev` outside Tauri. Nothing here touches a real launcher.

import type { Account, AccountInput, ActiveState, Game, Progress, Settings, Step } from "./types";

type Handler = (payload: unknown) => void;
const listeners = new Map<string, Set<Handler>>();

const emit = (event: string, payload: unknown) =>
  listeners.get(event)?.forEach((h) => h(payload));

const settings: Settings = {
  auto_submit: true,
  ask_game_each_time: true,
  close_to_tray: true,
  start_minimized: false,
  run_at_startup: false,
  hotkeys_enabled: true,
  riot: {
    client_path: "C:\\Riot Games\\Riot Client\\RiotClientServices.exe",
    username_point: null,
    ready_delay_ms: 2500,
  },
  garena: {
    launcher_path: "",
    login_tab_point: null,
    username_point: null,
    launch_button_point: null,
    ready_delay_ms: 3000,
    launch_delay_ms: 10000,
    session_paths: [],
    game_processes: [],
  },
};

let accounts: (Account & { password: string })[] = [
  acc("riot", "Main", "khoa.main", "Rank Kim cương", "valorant"),
  acc("riot", "Smurf", "khoa.smurf2", "", "league"),
  acc("riot", "", "acc.test.vn", "Dùng để thử", null),
  acc("garena", "DF chính", "khoa_df", "", "delta_force"),
];
const active: ActiveState = { riot: accounts[0].id, garena: null };

function acc(
  launcher: Account["launcher"],
  label: string,
  username: string,
  note: string,
  game: Game | null,
): Account & { password: string } {
  return {
    id: Math.random().toString(16).slice(2),
    launcher,
    label,
    username,
    note,
    default_game: game,
    last_used: null,
    has_password: true,
    password: "x",
  };
}

const views = (): Account[] =>
  accounts.map((a) => ({
    id: a.id,
    launcher: a.launcher,
    label: a.label,
    username: a.username,
    note: a.note,
    default_game: a.default_game,
    last_used: a.last_used,
    has_password: a.password.length > 0,
  }));
const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));

async function runSwitch(id: string, game: Game | null) {
  const a = accounts.find((x) => x.id === id);
  if (!a) throw "Không tìm thấy tài khoản";
  const steps: [Step, string][] = [
    ["closing", "Đang đăng xuất tài khoản hiện tại…"],
    ["launching", a.launcher === "riot" ? "Đang mở Riot Client…" : "Đang mở launcher Garena…"],
    ["waiting_window", "Chờ màn hình đăng nhập…"],
    ["signing_in", "Đang điền tài khoản — đừng chạm chuột/bàn phím"],
  ];
  if (game) steps.push(["starting_game", `Đang mở ${game}…`]);
  for (const [step, message] of steps) {
    emit("switch-progress", { step, message } satisfies Progress);
    await wait(900);
  }
  emit("switch-progress", { step: "done", message: "Xong" } satisfies Progress);
  active[a.launcher] = a.id;
  a.last_used = Math.floor(Date.now() / 1000);
  const notes =
    a.launcher === "garena" && game && !settings.garena.launch_button_point
      ? ['Chưa hiệu chỉnh nút "Khởi động" nên app không tự mở Delta Force.']
      : [];
  return { signed_in: true, game_started: !!game && notes.length === 0, notes };
}

export const mock = {
  listen(event: string, handler: Handler) {
    if (!listeners.has(event)) listeners.set(event, new Set());
    listeners.get(event)!.add(handler);
    return Promise.resolve(() => listeners.get(event)?.delete(handler));
  },

  async invoke<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
    await wait(60);
    const r = (v: unknown) => v as T;
    switch (cmd) {
      case "bootstrap":
        return r({
          accounts: views(),
          settings: structuredClone(settings),
          active: { ...active },
          load_error: null,
          supported: false,
          version: "0.1.0-preview",
          data_dir: "%APPDATA%\\GameAccountSwitcher",
          import_notice: null,
        });
      case "save_account": {
        const input = args.input as AccountInput;
        if (!input.username.trim()) throw "Tên đăng nhập không được để trống";
        if (input.id) {
          const a = accounts.find((x) => x.id === input.id)!;
          Object.assign(a, { ...input, password: input.password || a.password });
        } else {
          accounts.push({ ...acc(input.launcher, input.label, input.username, input.note, input.default_game) });
        }
        return r(views());
      }
      case "delete_account":
        accounts = accounts.filter((a) => a.id !== args.id);
        return r(views());
      case "move_account": {
        const i = accounts.findIndex((a) => a.id === args.id);
        const same = accounts.map((a, k) => [a, k] as const).filter(([a]) => a.launcher === accounts[i].launcher);
        const pos = same.findIndex(([, k]) => k === i);
        const target = same[pos + Math.sign(args.delta as number)];
        if (target) [accounts[i], accounts[target[1]]] = [accounts[target[1]], accounts[i]];
        return r(views());
      }
      case "set_default_game": {
        const a = accounts.find((x) => x.id === args.id);
        if (a) a.default_game = args.game as Game | null;
        return r(views());
      }
      case "save_settings":
        Object.assign(settings, args.settings);
        return r(structuredClone(settings));
      case "detect_riot_path":
        return r("C:\\Riot Games\\Riot Client\\RiotClientServices.exe");
      case "status":
        return r({
          riot: { configured: true, client_running: !!active.riot, game_running: false },
          garena: { configured: !!settings.garena.launcher_path, client_running: false, game_running: false },
          active: { ...active },
          busy: false,
        });
      case "switch_account":
        return r(await runSwitch(args.id as string, (args.game as Game | null) ?? null));
      case "sign_out":
        await wait(800);
        active[args.launcher as "riot" | "garena"] = null;
        return r(undefined);
      case "capture_point":
        await wait((args.delayMs as number) ?? 3000);
        return r({ x: 0.31, y: 0.42 });
      case "export_backup":
        return r(accounts.length);
      case "import_backup":
        return r({ added: 0, accounts: views() });
      case "open_data_dir":
      case "quit_app":
        return r(undefined);
      default:
        throw `mock: unknown command ${cmd}`;
    }
  },
};
