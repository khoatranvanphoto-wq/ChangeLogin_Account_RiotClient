import { useState, type FormEvent } from "react";
import { api, errorText, pick } from "../lib/api";
import type { Account } from "../lib/types";
import { Modal } from "./Modal";
import { useToast } from "./Toasts";

interface Props {
  mode: "export" | "import";
  onClose: () => void;
  onImported: (accounts: Account[]) => void;
}

export function BackupDialog({ mode, onClose, onImported }: Props) {
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [replace, setReplace] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const toast = useToast();

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    if (mode === "export") {
      if (password.length < 6) return setError("Mật khẩu cần ít nhất 6 ký tự");
      if (password !== confirm) return setError("Hai mật khẩu không khớp");
    }
    const path = mode === "export" ? await pick.backupToSave() : await pick.backupToOpen();
    if (!path) return;
    setBusy(true);
    try {
      if (mode === "export") {
        const n = await api.exportBackup(path, password);
        toast("success", `Đã xuất ${n} tài khoản`);
      } else {
        const r = await api.importBackup(path, password, replace);
        onImported(r.accounts);
        toast("success", replace ? `Đã thay bằng ${r.added} tài khoản` : `Đã thêm ${r.added} tài khoản mới`);
      }
      onClose();
    } catch (err) {
      setError(errorText(err));
      setBusy(false);
    }
  };

  return (
    <Modal
      title={mode === "export" ? "Xuất sao lưu" : "Nhập sao lưu"}
      onClose={onClose}
      footer={
        <>
          <button className="btn ghost" onClick={onClose}>Huỷ</button>
          <button className="btn primary" type="submit" form="backup-form" disabled={busy}>
            {busy ? "Đang xử lý…" : mode === "export" ? "Chọn nơi lưu…" : "Chọn file…"}
          </button>
        </>
      }
    >
      <form id="backup-form" className="form" onSubmit={submit}>
        <p className="hint">
          {mode === "export"
            ? "File sao lưu được mã hoá AES-256 bằng mật khẩu bạn đặt. Mất mật khẩu thì không mở lại được."
            : "Nhập mật khẩu đã dùng khi xuất file sao lưu."}
        </p>
        <label className="field">
          <span>Mật khẩu sao lưu</span>
          <input type="password" value={password} onChange={(e) => setPassword(e.target.value)} autoFocus />
        </label>
        {mode === "export" ? (
          <label className="field">
            <span>Nhập lại mật khẩu</span>
            <input type="password" value={confirm} onChange={(e) => setConfirm(e.target.value)} />
          </label>
        ) : (
          <div className="segmented">
            <button type="button" className={!replace ? "on" : ""} onClick={() => setReplace(false)}>
              Gộp (bỏ qua trùng)
            </button>
            <button type="button" className={replace ? "on" : ""} onClick={() => setReplace(true)}>
              Thay toàn bộ
            </button>
          </div>
        )}
        {error && <div className="form-error">{error}</div>}
      </form>
    </Modal>
  );
}
