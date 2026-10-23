import { useEffect, useRef, useState } from "react";
import { GAMES, displayName, type Account } from "../lib/types";
import { IconDown, IconEdit, IconLogin, IconMore, IconTrash, IconUp } from "./Icons";

interface Props {
  account: Account;
  active: boolean;
  hotkey: number | null;
  first: boolean;
  last: boolean;
  disabled: boolean;
  onLogin: () => void;
  onEdit: () => void;
  onDelete: () => void;
  onMove: (delta: number) => void;
}

function initials(a: Account): string {
  const words = displayName(a)
    .replace(/[^\p{L}\p{N} ]/gu, " ")
    .trim()
    .split(/\s+/)
    .filter(Boolean);
  if (words.length >= 2) return (words[0][0] + words[1][0]).toUpperCase();
  return (words[0]?.[0] ?? "?").toUpperCase();
}

function lastUsedText(ts: number | null): string | null {
  if (!ts) return null;
  const diff = Date.now() / 1000 - ts;
  if (diff < 60) return "vừa xong";
  if (diff < 3600) return `${Math.floor(diff / 60)} phút trước`;
  if (diff < 86400) return `${Math.floor(diff / 3600)} giờ trước`;
  return `${Math.floor(diff / 86400)} ngày trước`;
}

export function AccountCard(p: Props) {
  const a = p.account;
  const used = lastUsedText(a.last_used);
  const [menu, setMenu] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!menu) return;
    const close = (e: MouseEvent) => {
      if (!menuRef.current?.contains(e.target as Node)) setMenu(false);
    };
    const esc = (e: KeyboardEvent) => e.key === "Escape" && setMenu(false);
    document.addEventListener("mousedown", close);
    document.addEventListener("keydown", esc);
    return () => {
      document.removeEventListener("mousedown", close);
      document.removeEventListener("keydown", esc);
    };
  }, [menu]);

  const pickItem = (f: () => void) => () => {
    setMenu(false);
    f();
  };

  return (
    <li className={"card" + (p.active ? " is-active" : "")}>
      <div className="avatar" aria-hidden>{initials(a)}</div>

      <div className="card-main">
        <div className="card-title">
          <span className="name">{displayName(a)}</span>
          {p.active && <span className="badge live">Đang đăng nhập</span>}
        </div>
        <div className="card-sub">
          {a.label.trim() && <span className="mono">{a.username}</span>}
          {a.default_game && <span className="chip">{GAMES[a.default_game].short}</span>}
          {p.hotkey && <kbd>Ctrl+Alt+{p.hotkey}</kbd>}
          {used && <span className="muted">{used}</span>}
        </div>
        {a.note.trim() && <div className="card-note">{a.note}</div>}
      </div>

      <div className="card-actions">
        <button className="btn primary login" onClick={p.onLogin} disabled={p.disabled}>
          <IconLogin />
          {p.active ? "Mở" : "Đăng nhập"}
        </button>
        <div className="menu-wrap" ref={menuRef}>
          <button
            className={"icon-btn" + (menu ? " on" : "")}
            onClick={() => setMenu((m) => !m)}
            aria-label="Thêm thao tác"
            aria-haspopup="menu"
            aria-expanded={menu}
          >
            <IconMore />
          </button>
          {menu && (
            <div className="menu" role="menu">
              <button role="menuitem" onClick={pickItem(p.onEdit)}>
                <IconEdit /> Sửa
              </button>
              <button role="menuitem" onClick={pickItem(() => p.onMove(-1))} disabled={p.first}>
                <IconUp /> Lên trên
              </button>
              <button role="menuitem" onClick={pickItem(() => p.onMove(1))} disabled={p.last}>
                <IconDown /> Xuống dưới
              </button>
              <hr />
              <button role="menuitem" className="danger" onClick={pickItem(p.onDelete)}>
                <IconTrash /> Xoá
              </button>
            </div>
          )}
        </div>
      </div>
    </li>
  );
}
