import type { Progress, Step } from "../lib/types";
import { IconAlert, IconCheck, IconSpinner } from "./Icons";

const STEPS: { step: Step; label: string }[] = [
  { step: "closing", label: "Đăng xuất tài khoản cũ" },
  { step: "launching", label: "Mở launcher" },
  { step: "waiting_window", label: "Chờ màn hình đăng nhập" },
  { step: "signing_in", label: "Điền tài khoản" },
  { step: "starting_game", label: "Mở game" },
];

interface Props {
  title: string;
  withGame: boolean;
  progress: Progress | null;
  error: string | null;
  onRetry: () => void;
  onClose: () => void;
}

export function ProgressOverlay({ title, withGame, progress, error, onRetry, onClose }: Props) {
  const steps = withGame ? STEPS : STEPS.filter((s) => s.step !== "starting_game");
  const currentIdx = progress ? steps.findIndex((s) => s.step === progress.step) : -1;
  const done = progress?.step === "done";

  return (
    <div className="backdrop solid">
      <div className="progress-panel" role="status">
        <h2>{title}</h2>
        <ol className="steps">
          {steps.map((s, i) => {
            const state = done || i < currentIdx ? "done" : i === currentIdx ? (error ? "failed" : "now") : "todo";
            return (
              <li key={s.step} className={"step " + state}>
                <span className="step-dot">
                  {state === "done" ? <IconCheck /> : state === "now" ? <IconSpinner /> : state === "failed" ? <IconAlert /> : null}
                </span>
                {s.label}
              </li>
            );
          })}
        </ol>

        {error ? (
          <>
            <div className="form-error">{error}</div>
            <div className="row-end">
              <button className="btn ghost" onClick={onClose}>Đóng</button>
              <button className="btn primary" onClick={onRetry}>Thử lại</button>
            </div>
          </>
        ) : (
          <p className="progress-msg">
            {progress?.message ?? "Đang chuẩn bị…"}
            {!done && <span className="warn">Đừng chạm chuột hoặc bàn phím cho tới khi xong.</span>}
          </p>
        )}
      </div>
    </div>
  );
}
