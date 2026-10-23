import { useState } from "react";
import { displayName, type Account, type Game } from "../lib/types";
import { Modal } from "./Modal";

interface Props {
  account: Account;
  onPick: (game: Game | null, remember: boolean) => void;
  onCancel: () => void;
}

const OPTIONS: { game: Game | null; title: string; sub: string; cls: string }[] = [
  { game: "league", title: "League of Legends", sub: "Đăng nhập rồi mở LMHT", cls: "lol" },
  { game: "valorant", title: "VALORANT", sub: "Đăng nhập rồi mở VALORANT", cls: "val" },
  { game: null, title: "Chỉ đăng nhập", sub: "Mở Riot Client, không chạy game", cls: "none" },
];

export function GamePicker({ account, onPick, onCancel }: Props) {
  const [remember, setRemember] = useState(false);
  return (
    <Modal title={`Chơi gì với ${displayName(account)}?`} onClose={onCancel}>
      <div className="game-grid">
        {OPTIONS.map((o) => (
          <button
            key={o.cls}
            className={"game-tile " + o.cls + (account.default_game === o.game ? " is-default" : "")}
            onClick={() => onPick(o.game, remember)}
          >
            <span className="game-title">{o.title}</span>
            <span className="game-sub">{o.sub}</span>
            {account.default_game === o.game && o.game && <span className="badge">Mặc định</span>}
          </button>
        ))}
      </div>
      <label className="check">
        <input type="checkbox" checked={remember} onChange={(e) => setRemember(e.target.checked)} />
        <span>Đặt làm game mặc định cho tài khoản này</span>
      </label>
    </Modal>
  );
}
