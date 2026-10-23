import { useCallback, useEffect, useRef, useState } from "react";
import { api, errorText, on } from "./lib/api";
import {
  GAMES,
  LAUNCHERS,
  displayName,
  hotkeyNumber,
  type Account,
  type ActiveState,
  type Bootstrap,
  type Game,
  type Launcher,
  type Progress,
  type Settings,
  type StatusView,
} from "./lib/types";
import { AccountCard } from "./components/AccountCard";
import { AccountDialog } from "./components/AccountDialog";
import { BackupDialog } from "./components/BackupDialog";
import { GamePicker } from "./components/GamePicker";
import { IconAlert, IconLogout, IconPlus, IconSettings, Logo } from "./components/Icons";
import { ConfirmDialog } from "./components/Modal";
import { ProgressOverlay } from "./components/ProgressOverlay";
import { SettingsDialog } from "./components/SettingsDialog";
import { useToast } from "./components/Toasts";

interface Run {
  account: Account;
  game: Game | null;
  progress: Progress | null;
  error: string | null;
}

export default function App() {
  const toast = useToast();
  const [boot, setBoot] = useState<Bootstrap | null>(null);
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [active, setActive] = useState<ActiveState>({ riot: null, garena: null });
  const [status, setStatus] = useState<StatusView | null>(null);
  const [tab, setTab] = useState<Launcher>("riot");

  const [editing, setEditing] = useState<{ launcher: Launcher; account: Account | null } | null>(null);
  const [deleting, setDeleting] = useState<Account | null>(null);
  const [picking, setPicking] = useState<Account | null>(null);
  const [confirmKill, setConfirmKill] = useState<{ account: Account; game: Game | null } | null>(null);
  const [run, setRun] = useState<Run | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [backup, setBackup] = useState<"export" | "import" | null>(null);
  const [signingOut, setSigningOut] = useState<Launcher | null>(null);

  // Latest values for event handlers registered once.
  const latest = useRef({ accounts, settings, run, status });
  latest.current = { accounts, settings, run, status };

  const reload = useCallback(async () => {
    const b = await api.bootstrap();
    setBoot(b);
    setAccounts(b.accounts);
    setSettings(b.settings);
    setActive(b.active);
    return b;
  }, []);

  const refreshStatus = useCallback(async () => {
    try {
      const s = await api.status();
      setStatus(s);
      setActive(s.active);
    } catch {
      /* status is best effort */
    }
  }, []);

  useEffect(() => {
    reload()
      .then((b) => {
        const n = b.import_notice;
        if (n?.report && n.report.accounts_added > 0) {
          toast("success", `Đã chuyển ${n.report.accounts_added} tài khoản từ Riot Account Manager bản cũ`);
        } else if (n?.error) {
          toast("error", `Không chuyển được dữ liệu bản cũ: ${n.error}`);
        }
      })
      .catch((e) => toast("error", errorText(e)));
  }, [reload, toast]);

  useEffect(() => {
    refreshStatus();
    const t = setInterval(() => document.visibilityState === "visible" && refreshStatus(), 4000);
    return () => clearInterval(t);
  }, [refreshStatus]);

  // ---------------------------------------------------------------- switching

  const execute = useCallback(
    async (account: Account, game: Game | null) => {
      setRun({ account, game, progress: null, error: null });
      try {
        const outcome = await api.switchAccount(account.id, game);
        setRun(null);
        const what = game && outcome.game_started ? ` và mở ${GAMES[game].name}` : "";
        toast("success", outcome.signed_in ? `Đã đăng nhập ${displayName(account)}${what}` : `Đã mở ${displayName(account)}${what}`);
        outcome.notes.forEach((n) => toast("info", n));
      } catch (e) {
        setRun((r) => (r ? { ...r, error: errorText(e) } : r));
      } finally {
        reload().catch(() => {});
        refreshStatus();
      }
    },
    [reload, refreshStatus, toast],
  );

  const proceed = useCallback(
    async (account: Account, game: Game | null) => {
      try {
        const st = await api.status();
        const isActive = st.active[account.launcher] === account.id;
        if (!isActive && st[account.launcher].game_running) {
          setConfirmKill({ account, game });
          return;
        }
      } catch {
        /* fall through */
      }
      execute(account, game);
    },
    [execute],
  );

  const startSwitch = useCallback(
    (account: Account) => {
      if (latest.current.run) return;
      if (latest.current.status && !latest.current.status[account.launcher].configured) {
        toast(
          "error",
          account.launcher === "riot"
            ? "Chưa tìm thấy Riot Client — hãy chọn RiotClientServices.exe."
            : "Chưa chọn launcher Garena / Delta Force.",
        );
        setShowSettings(true);
        return;
      }
      const s = latest.current.settings;
      if (account.launcher === "riot") {
        if (!s || s.ask_game_each_time || !account.default_game) {
          setPicking(account);
          return;
        }
        proceed(account, account.default_game);
      } else {
        proceed(account, account.default_game === "delta_force" ? "delta_force" : null);
      }
    },
    [proceed, toast],
  );

  const onPicked = async (game: Game | null, remember: boolean) => {
    const account = picking!;
    setPicking(null);
    if (remember && game !== account.default_game) {
      try {
        setAccounts(await api.setDefaultGame(account.id, game));
      } catch (e) {
        toast("error", errorText(e));
      }
    }
    proceed({ ...account, default_game: remember ? game : account.default_game }, game);
  };

  const signOut = useCallback(
    async (launcher: Launcher) => {
      setSigningOut(launcher);
      try {
        await api.signOut(launcher);
        toast("success", `Đã đăng xuất ${LAUNCHERS[launcher].name}`);
      } catch (e) {
        toast("error", errorText(e));
      } finally {
        setSigningOut(null);
        refreshStatus();
      }
    },
    [refreshStatus, toast],
  );

  // ------------------------------------------------------------ backend events

  useEffect(() => {
    const subs = [
      on<Progress>("switch-progress", (p) => setRun((r) => (r ? { ...r, progress: p } : r))),
      on<{ id: string; source: string }>("request-switch", ({ id }) => {
        const acc = latest.current.accounts.find((a) => a.id === id);
        if (acc) {
          setTab(acc.launcher);
          startSwitch(acc);
        }
      }),
      on<Launcher>("request-sign-out", (l) => signOut(l)),
      on<null>("accounts-changed", () => reload().catch(() => {})),
    ];
    return () => subs.forEach((p) => p.then((un) => un()));
  }, [startSwitch, signOut, reload]);

  // ------------------------------------------------------------------- render

  if (!boot || !settings) {
    return <div className="loading">Đang tải…</div>;
  }

  const list = accounts.filter((a) => a.launcher === tab);
  const st = status?.[tab];
  const configured = st?.configured ?? true;
  const activeId = active[tab];

  return (
    <div className={"app theme-" + tab}>
      <header className="topbar">
        <div className="brand">
          <Logo />
          <div>
            <h1>Game Account Switcher</h1>
            <small>Đổi tài khoản trong một cú bấm</small>
          </div>
        </div>
        <button className="icon-btn" onClick={() => setShowSettings(true)} aria-label="Cài đặt">
          <IconSettings />
        </button>
      </header>

      <nav className="tabs">
        {(["riot", "garena"] as Launcher[]).map((l) => (
          <button key={l} className={(tab === l ? "on " : "") + "tab-" + l} onClick={() => setTab(l)}>
            {LAUNCHERS[l].tab}
            <span className="count">{accounts.filter((a) => a.launcher === l).length}</span>
          </button>
        ))}
      </nav>

      {!boot.supported && (
        <div className="banner info">Bản xem trước giao diện — chỉ đổi tài khoản thật được trên Windows.</div>
      )}
      {boot.load_error && (
        <div className="banner error">
          <IconAlert /> {boot.load_error}
        </div>
      )}
      {!configured && (
        <div className="banner warn">
          <IconAlert />
          <span>
            {tab === "riot" ? "Chưa tìm thấy Riot Client." : "Chưa chọn launcher Garena / Delta Force."}
          </span>
          <button className="btn small" onClick={() => setShowSettings(true)}>Cài đặt</button>
        </div>
      )}

      <div className="statusbar">
        <span className={"dot " + (st?.client_running ? "on" : "")} />
        <span>
          {LAUNCHERS[tab].name} {st?.client_running ? "đang chạy" : "chưa mở"}
          {st?.game_running && " · game đang chạy"}
        </span>
        {st?.client_running && (
          <button className="link" onClick={() => signOut(tab)} disabled={!!signingOut || !!run}>
            <IconLogout /> {signingOut === tab ? "Đang đăng xuất…" : "Đăng xuất"}
          </button>
        )}
      </div>

      <main className="list-wrap">
        {list.length === 0 ? (
          <div className="empty">
            <p>Chưa có tài khoản {tab === "riot" ? "Riot" : "Garena"} nào.</p>
            <button className="btn primary" onClick={() => setEditing({ launcher: tab, account: null })}>
              <IconPlus /> Thêm tài khoản đầu tiên
            </button>
          </div>
        ) : (
          <ul className="list">
            {list.map((a, i) => (
              <AccountCard
                key={a.id}
                account={a}
                active={a.id === activeId}
                hotkey={settings.hotkeys_enabled ? hotkeyNumber(accounts, a.id) : null}
                first={i === 0}
                last={i === list.length - 1}
                disabled={!!run || !!signingOut}
                onLogin={() => startSwitch(a)}
                onEdit={() => setEditing({ launcher: a.launcher, account: a })}
                onDelete={() => setDeleting(a)}
                onMove={async (d) => {
                  try {
                    setAccounts(await api.moveAccount(a.id, d));
                  } catch (e) {
                    toast("error", errorText(e));
                  }
                }}
              />
            ))}
          </ul>
        )}
      </main>

      {list.length > 0 && (
        <footer className="bottombar">
          <button className="btn add" onClick={() => setEditing({ launcher: tab, account: null })}>
            <IconPlus /> Thêm tài khoản
          </button>
        </footer>
      )}

      {editing && (
        <AccountDialog
          launcher={editing.launcher}
          account={editing.account}
          onClose={() => setEditing(null)}
          onSaved={(list) => {
            setAccounts(list);
            setEditing(null);
            toast("success", editing.account ? "Đã cập nhật tài khoản" : "Đã thêm tài khoản");
          }}
        />
      )}

      {deleting && (
        <ConfirmDialog
          title="Xoá tài khoản?"
          message={
            <>
              Xoá <b>{displayName(deleting)}</b> khỏi danh sách. Thao tác này không hoàn tác được (tài khoản game không bị ảnh hưởng).
            </>
          }
          confirmText="Xoá"
          danger
          onCancel={() => setDeleting(null)}
          onConfirm={async () => {
            const id = deleting.id;
            setDeleting(null);
            try {
              setAccounts(await api.deleteAccount(id));
              toast("success", "Đã xoá");
            } catch (e) {
              toast("error", errorText(e));
            }
          }}
        />
      )}

      {picking && <GamePicker account={picking} onPick={onPicked} onCancel={() => setPicking(null)} />}

      {confirmKill && (
        <ConfirmDialog
          title="Game đang chạy"
          message={`Đổi sang ${displayName(confirmKill.account)} sẽ đóng game và launcher hiện tại. Tiếp tục?`}
          confirmText="Đóng game và đổi"
          danger
          onCancel={() => setConfirmKill(null)}
          onConfirm={() => {
            const c = confirmKill;
            setConfirmKill(null);
            execute(c.account, c.game);
          }}
        />
      )}

      {run && (
        <ProgressOverlay
          title={`Đăng nhập ${displayName(run.account)}`}
          withGame={!!run.game}
          progress={run.progress}
          error={run.error}
          onClose={() => setRun(null)}
          onRetry={() => execute(run.account, run.game)}
        />
      )}

      {showSettings && (
        <SettingsDialog
          settings={settings}
          version={boot.version}
          dataDir={boot.data_dir}
          onClose={() => setShowSettings(false)}
          onSaved={(s) => {
            setSettings(s);
            setShowSettings(false);
            refreshStatus();
          }}
          onBackup={(m) => setBackup(m)}
        />
      )}

      {backup && (
        <BackupDialog mode={backup} onClose={() => setBackup(null)} onImported={(list) => setAccounts(list)} />
      )}
    </div>
  );
}
