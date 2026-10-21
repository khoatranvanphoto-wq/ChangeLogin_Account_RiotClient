import type { Account } from "../types";
import { useI18n } from "../i18n/I18nContext";

interface Props {
  accounts: Account[];
  selected: number;
  loggedInUsername?: string | null;
  onSelect: (index: number) => void;
  onLogin: (index: number) => void;
}

export function AccountTable({ accounts, selected, loggedInUsername, onSelect, onLogin }: Props) {
  const { t } = useI18n();

  return (
    <div className="table-wrap">
      <table className="account-table">
        <thead>
          <tr>
            <th>{t("account.table.username")}</th>
            <th>{t("account.table.region")}</th>
            <th>{t("account.table.note")}</th>
          </tr>
        </thead>
        <tbody>
          {accounts.length === 0 ? (
            <tr>
              <td colSpan={3} className="empty-row">{t("account.list.empty")}</td>
            </tr>
          ) : (
            accounts.map((a, i) => (
              <tr
                key={i}
                className={i === selected ? "selected" : ""}
                onClick={() => onSelect(i)}
                onDoubleClick={() => onLogin(i)}
              >
                <td>
                  {a.username}
                  {loggedInUsername && a.username === loggedInUsername && (
                    <span className="row-badge">{t("account.badge.loggedIn")}</span>
                  )}
                </td>
                <td>{a.region}</td>
                <td>{a.note}</td>
              </tr>
            ))
          )}
        </tbody>
      </table>
    </div>
  );
}
