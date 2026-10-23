import { createContext, useCallback, useContext, useState, type ReactNode } from "react";
import { IconAlert, IconCheck, IconX } from "./Icons";

type Kind = "success" | "error" | "info";
interface Toast {
  id: number;
  kind: Kind;
  text: string;
}

const Ctx = createContext<(kind: Kind, text: string) => void>(() => {});

export const useToast = () => useContext(Ctx);

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);

  const dismiss = (id: number) => setToasts((t) => t.filter((x) => x.id !== id));

  const push = useCallback((kind: Kind, text: string) => {
    const id = Date.now() + Math.random();
    setToasts((t) => [...t.slice(-2), { id, kind, text }]);
    setTimeout(() => dismiss(id), kind === "success" ? 3500 : 7000);
  }, []);

  return (
    <Ctx.Provider value={push}>
      {children}
      <div className="toasts" aria-live="polite">
        {toasts.map((t) => (
          <div key={t.id} className={"toast " + t.kind}>
            {t.kind === "error" ? <IconAlert /> : <IconCheck />}
            <span>{t.text}</span>
            <button className="icon-btn small" onClick={() => dismiss(t.id)} aria-label="Đóng">
              <IconX />
            </button>
          </div>
        ))}
      </div>
    </Ctx.Provider>
  );
}
