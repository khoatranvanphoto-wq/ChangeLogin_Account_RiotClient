import { useState, type FormEvent } from "react";
import { api, errorText } from "../lib/api";
import type { Account, AccountInput, Game, Launcher } from "../lib/types";
import { IconEye, IconEyeOff } from "./Icons";
import { Modal } from "./Modal";

interface Props {
  launcher: Launcher;
  account: Account | null;
  onClose: () => void;
  onSaved: (accounts: Account[]) => void;
}

export function AccountDialog({ launcher, account, onClose, onSaved }: Props) {
  const editing = !!account;
  const [form, setForm] = useState<AccountInput>({
    id: account?.id ?? null,
    launcher,
    label: account?.label ?? "",
    username: account?.username ?? "",
    password: "",
    note: account?.note ?? "",
    default_game: account ? account.default_game : launcher === "garena" ? "delta_force" : null,
  });
  const [showPw, setShowPw] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const set = <K extends keyof AccountInput>(k: K, v: AccountInput[K]) => setForm((f) => ({ ...f, [k]: v }));

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (!form.username.trim()) return setError("Nhập tên đăng nhập");
    if (!editing && !form.password) return setError("Nhập mật khẩu");
    setSaving(true);
    try {
      onSaved(await api.saveAccount(form));
    } catch (err) {
      setError(errorText(err));
      setSaving(false);
    }
  };

  return (
    <Modal
      title={editing ? "Sửa tài khoản" : launcher === "riot" ? "Thêm tài khoản Riot" : "Thêm tài khoản Garena"}
      onClose={onClose}
      footer={
        <>
          <button className="btn ghost" type="button" onClick={onClose}>Huỷ</button>
          <button className="btn primary" type="submit" form="account-form" disabled={saving}>
            {saving ? "Đang lưu…" : "Lưu"}
          </button>
        </>
      }
    >
      <form id="account-form" className="form" onSubmit={submit} autoComplete="off">
        <label className="field">
          <span>Tên hiển thị <em>(tuỳ chọn)</em></span>
          <input value={form.label} onChange={(e) => set("label", e.target.value)} placeholder="VD: Acc chính, Smurf…" autoFocus />
        </label>

        <label className="field">
          <span>{launcher === "riot" ? "Tên đăng nhập Riot" : "Tài khoản Garena"}</span>
          <input
            value={form.username}
            onChange={(e) => set("username", e.target.value)}
            spellCheck={false}
            className="mono"
          />
        </label>

        <label className="field">
          <span>
            Mật khẩu {editing && <em>(để trống nếu không đổi)</em>}
          </span>
          <div className="input-row">
            <input
              type={showPw ? "text" : "password"}
              value={form.password}
              onChange={(e) => set("password", e.target.value)}
              className="mono"
            />
            <button type="button" className="icon-btn" onClick={() => setShowPw((s) => !s)} aria-label={showPw ? "Ẩn" : "Hiện"}>
              {showPw ? <IconEyeOff /> : <IconEye />}
            </button>
          </div>
        </label>

        {launcher === "riot" ? (
          <div className="field">
            <span>Game mặc định</span>
            <div className="segmented">
              {([null, "league", "valorant"] as (Game | null)[]).map((g) => (
                <button
                  type="button"
                  key={g ?? "none"}
                  className={form.default_game === g ? "on" : ""}
                  onClick={() => set("default_game", g)}
                >
                  {g === null ? "Chỉ đăng nhập" : g === "league" ? "LoL" : "VALORANT"}
                </button>
              ))}
            </div>
            <small className="hint">Dùng khi bấm từ khay hệ thống / phím tắt, hoặc khi tắt "Hỏi chọn game mỗi lần".</small>
          </div>
        ) : (
          <label className="check">
            <input
              type="checkbox"
              checked={form.default_game === "delta_force"}
              onChange={(e) => set("default_game", e.target.checked ? "delta_force" : null)}
            />
            <span>Tự mở Delta Force sau khi đăng nhập</span>
          </label>
        )}

        <label className="field">
          <span>Ghi chú <em>(tuỳ chọn)</em></span>
          <input value={form.note} onChange={(e) => set("note", e.target.value)} />
        </label>

        {error && <div className="form-error">{error}</div>}
        <p className="hint">Mật khẩu được mã hoá bằng Windows DPAPI và chỉ lưu trên máy này.</p>
      </form>
    </Modal>
  );
}
