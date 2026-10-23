import { useState, type ReactNode } from "react";
import { api, errorText, pick } from "../lib/api";
import type { Launcher, RelPoint, Settings } from "../lib/types";
import { IconFolder, IconTarget, IconX } from "./Icons";
import { Modal } from "./Modal";
import { useToast } from "./Toasts";

type Tab = "general" | "riot" | "garena" | "data";

interface Props {
  settings: Settings;
  version: string;
  dataDir: string;
  onClose: () => void;
  onSaved: (s: Settings) => void;
  onBackup: (mode: "export" | "import") => void;
}

export function SettingsDialog({ settings, version, dataDir, onClose, onSaved, onBackup }: Props) {
  const [s, setS] = useState<Settings>(structuredClone(settings));
  const [tab, setTab] = useState<Tab>("general");
  const [saving, setSaving] = useState(false);
  const toast = useToast();

  const patch = (f: (d: Settings) => void) =>
    setS((cur) => {
      const next = structuredClone(cur);
      f(next);
      return next;
    });

  const save = async () => {
    setSaving(true);
    try {
      onSaved(await api.saveSettings(s));
      toast("success", "Đã lưu cài đặt");
    } catch (e) {
      toast("error", errorText(e));
      setSaving(false);
    }
  };

  return (
    <Modal
      title="Cài đặt"
      wide
      onClose={onClose}
      footer={
        <>
          <button className="btn ghost" onClick={onClose}>Huỷ</button>
          <button className="btn primary" onClick={save} disabled={saving}>
            {saving ? "Đang lưu…" : "Lưu"}
          </button>
        </>
      }
    >
      <nav className="tabs small">
        {(
          [
            ["general", "Chung"],
            ["riot", "Riot Client"],
            ["garena", "Garena"],
            ["data", "Dữ liệu"],
          ] as [Tab, string][]
        ).map(([t, label]) => (
          <button key={t} className={tab === t ? "on" : ""} onClick={() => setTab(t)}>
            {label}
          </button>
        ))}
      </nav>

      {tab === "general" && (
        <div className="form">
          <Toggle label="Tự bấm Đăng nhập sau khi điền" checked={s.auto_submit} onChange={(v) => patch((d) => (d.auto_submit = v))} />
          <Toggle
            label="Hỏi chọn game mỗi lần đăng nhập Riot"
            hint="Tắt để dùng luôn game mặc định của tài khoản."
            checked={s.ask_game_each_time}
            onChange={(v) => patch((d) => (d.ask_game_each_time = v))}
          />
          <Toggle
            label="Phím tắt toàn cục"
            hint="Ctrl+Alt+A: hiện/ẩn cửa sổ · Ctrl+Alt+1…9: đăng nhập tài khoản số 1…9."
            checked={s.hotkeys_enabled}
            onChange={(v) => patch((d) => (d.hotkeys_enabled = v))}
          />
          <Toggle label="Đóng cửa sổ thì thu xuống khay hệ thống" checked={s.close_to_tray} onChange={(v) => patch((d) => (d.close_to_tray = v))} />
          <Toggle label="Khởi động cùng Windows" checked={s.run_at_startup} onChange={(v) => patch((d) => (d.run_at_startup = v))} />
          <Toggle label="Mở app ở dạng thu nhỏ" checked={s.start_minimized} onChange={(v) => patch((d) => (d.start_minimized = v))} />
        </div>
      )}

      {tab === "riot" && (
        <div className="form">
          <PathField
            label="RiotClientServices.exe"
            value={s.riot.client_path}
            placeholder="Để trống để tự tìm"
            onChange={(v) => patch((d) => (d.riot.client_path = v))}
            extra={
              <button
                className="btn ghost small"
                onClick={async () => {
                  const p = await api.detectRiotPath();
                  if (p) patch((d) => (d.riot.client_path = p));
                  else toast("error", "Không tự tìm thấy Riot Client");
                }}
              >
                Tự tìm
              </button>
            }
          />
          <NumberField
            label="Chờ sau khi cửa sổ hiện (ms)"
            value={s.riot.ready_delay_ms}
            onChange={(v) => patch((d) => (d.riot.ready_delay_ms = v))}
          />
          <h3>Hiệu chỉnh (không bắt buộc)</h3>
          <p className="hint">
            App tự tìm ô đăng nhập. Nếu hay thất bại, mở Riot Client ở màn hình đăng nhập rồi ghi vị trí ô Tên đăng nhập.
          </p>
          <Calibrate
            launcher="riot"
            label="Ô Tên đăng nhập"
            value={s.riot.username_point}
            onChange={(p) => patch((d) => (d.riot.username_point = p))}
          />
        </div>
      )}

      {tab === "garena" && (
        <div className="form">
          <PathField
            label="Launcher Garena / Delta Force (.exe)"
            value={s.garena.launcher_path}
            placeholder="Chọn file .exe của launcher"
            onChange={(v) => patch((d) => (d.garena.launcher_path = v))}
          />
          <div className="grid2">
            <NumberField
              label="Chờ sau khi cửa sổ hiện (ms)"
              value={s.garena.ready_delay_ms}
              onChange={(v) => patch((d) => (d.garena.ready_delay_ms = v))}
            />
            <NumberField
              label="Chờ đăng nhập xong trước khi bấm Khởi động (ms)"
              value={s.garena.launch_delay_ms}
              onChange={(v) => patch((d) => (d.garena.launch_delay_ms = v))}
            />
          </div>

          <h3>Hiệu chỉnh vị trí</h3>
          <p className="hint">
            Mở launcher tới đúng màn hình, bấm <b>Ghi vị trí</b>, rồi trong 5 giây đặt chuột lên ô/nút cần ghi (không cần bấm).
            Vị trí được lưu theo tỉ lệ cửa sổ nên di chuyển cửa sổ vẫn dùng được.
          </p>
          <Calibrate
            launcher="garena"
            label="Tab đăng nhập bằng tài khoản Garena"
            optional
            value={s.garena.login_tab_point}
            onChange={(p) => patch((d) => (d.garena.login_tab_point = p))}
          />
          <Calibrate
            launcher="garena"
            label="Ô Tài khoản"
            value={s.garena.username_point}
            onChange={(p) => patch((d) => (d.garena.username_point = p))}
          />
          <Calibrate
            launcher="garena"
            label='Nút "Khởi động" Delta Force'
            value={s.garena.launch_button_point}
            onChange={(p) => patch((d) => (d.garena.launch_button_point = p))}
          />

          <h3>Nâng cao</h3>
          <ListField
            label="Tiến trình game (mỗi dòng một tên .exe)"
            hint="Dùng để biết game đang chạy và tắt game khi đổi tài khoản."
            value={s.garena.game_processes}
            onChange={(v) => patch((d) => (d.garena.game_processes = v))}
          />
          <ListField
            label="File/thư mục lưu phiên đăng nhập (xoá khi đăng xuất)"
            hint="Chỉ điền nếu launcher tự đăng nhập lại tài khoản cũ. Hỗ trợ %LOCALAPPDATA%, %APPDATA%."
            value={s.garena.session_paths}
            onChange={(v) => patch((d) => (d.garena.session_paths = v))}
          />
        </div>
      )}

      {tab === "data" && (
        <div className="form">
          <p className="hint">
            Tài khoản được mã hoá bằng Windows DPAPI: chỉ đọc được bởi chính user Windows này trên máy này. Muốn chuyển sang máy khác,
            hãy xuất file sao lưu có đặt mật khẩu.
          </p>
          <div className="row">
            <button className="btn" onClick={() => onBackup("export")}>Xuất sao lưu…</button>
            <button className="btn" onClick={() => onBackup("import")}>Nhập sao lưu…</button>
          </div>
          <div className="row">
            <button className="btn ghost" onClick={() => api.openDataDir().catch((e) => toast("error", errorText(e)))}>
              <IconFolder /> Mở thư mục dữ liệu
            </button>
          </div>
          <p className="hint mono">{dataDir}</p>
          <p className="hint">Phiên bản {version}</p>
        </div>
      )}
    </Modal>
  );
}

function Toggle(p: { label: string; hint?: string; checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <label className="toggle">
      <span className="toggle-text">
        {p.label}
        {p.hint && <small className="hint">{p.hint}</small>}
      </span>
      <input type="checkbox" role="switch" checked={p.checked} onChange={(e) => p.onChange(e.target.checked)} />
    </label>
  );
}

function PathField(p: {
  label: string;
  value: string;
  placeholder?: string;
  onChange: (v: string) => void;
  extra?: ReactNode;
}) {
  return (
    <label className="field">
      <span>{p.label}</span>
      <div className="input-row">
        <input className="mono" value={p.value} placeholder={p.placeholder} onChange={(e) => p.onChange(e.target.value)} spellCheck={false} />
        <button
          type="button"
          className="btn ghost small"
          onClick={async () => {
            const path = await pick.exe();
            if (path) p.onChange(path);
          }}
        >
          Chọn…
        </button>
        {p.extra}
      </div>
    </label>
  );
}

function NumberField(p: { label: string; value: number; onChange: (v: number) => void }) {
  return (
    <label className="field">
      <span>{p.label}</span>
      <input
        type="number"
        min={0}
        step={500}
        value={p.value}
        onChange={(e) => p.onChange(Math.max(0, Number(e.target.value) || 0))}
      />
    </label>
  );
}

function ListField(p: { label: string; hint?: string; value: string[]; onChange: (v: string[]) => void }) {
  const [text, setText] = useState(p.value.join("\n"));
  return (
    <label className="field">
      <span>{p.label}</span>
      <textarea
        className="mono"
        rows={3}
        value={text}
        spellCheck={false}
        onChange={(e) => {
          setText(e.target.value);
          p.onChange(e.target.value.split("\n").map((x) => x.trim()).filter(Boolean));
        }}
      />
      {p.hint && <small className="hint">{p.hint}</small>}
    </label>
  );
}

function Calibrate(p: {
  launcher: Launcher;
  label: string;
  optional?: boolean;
  value: RelPoint | null;
  onChange: (p: RelPoint | null) => void;
}) {
  const [busy, setBusy] = useState(false);
  const toast = useToast();
  const record = async () => {
    setBusy(true);
    try {
      const point = await api.capturePoint(p.launcher, 5000);
      p.onChange(point);
      toast("success", `Đã ghi vị trí "${p.label}" — nhớ bấm Lưu`);
    } catch (e) {
      toast("error", errorText(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="calib">
      <div className="calib-text">
        <span>
          {p.label} {p.optional && <em>(tuỳ chọn)</em>}
        </span>
        <small className={p.value ? "ok" : "muted"}>
          {p.value ? `Đã ghi (${Math.round(p.value.x * 100)}%, ${Math.round(p.value.y * 100)}%)` : "Chưa ghi — dùng tự động"}
        </small>
      </div>
      {p.value && (
        <button className="icon-btn small" onClick={() => p.onChange(null)} aria-label="Xoá vị trí">
          <IconX />
        </button>
      )}
      <button className="btn ghost small" onClick={record} disabled={busy}>
        <IconTarget /> {busy ? "Đặt chuột vào vị trí…" : "Ghi vị trí"}
      </button>
    </div>
  );
}
