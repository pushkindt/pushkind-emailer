import { useEffect, useState } from "react";
import { EmailerShell } from "../components/EmailerShell";
import { EmailerShellFatalState } from "../components/EmailerShellFatalState";
import {
  fetchHubMenuItems,
  fetchShellData,
  fetchUnsubscribedPageData,
} from "../lib/api";
import type {
  ShellData,
  UnsubscribedPageData,
  UserMenuItem,
} from "../lib/models";
import { useServiceShell } from "@pushkind/frontend-shell/useServiceShell";

export function UnsubscribedBootstrap() {
  const shellState = useServiceShell<ShellData, UserMenuItem>({
    errorMessage: "Не удалось загрузить оболочку Emailer.",
    menuLoadWarning:
      "Failed to load auth navigation menu. Falling back to local Emailer menu only.",
    fetchShellData,
    fetchHubMenuItems,
  });
  const [data, setData] = useState<UnsubscribedPageData | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void fetchUnsubscribedPageData()
      .then(setData)
      .catch((err) =>
        setError(
          err instanceof Error
            ? err.message
            : "Не удалось загрузить отписавшихся.",
        ),
      );
  }, []);

  if (shellState.status === "loading" || (data == null && error == null))
    return null;
  if (shellState.status === "error")
    return <EmailerShellFatalState message={shellState.message} />;
  if (error != null) return <EmailerShellFatalState message={error} />;

  return (
    <EmailerShell
      navigation={shellState.shell.navigation}
      currentUserEmail={shellState.shell.currentUser.email}
      homeUrl={shellState.shell.homeUrl}
      localMenuItems={shellState.shell.localMenuItems}
      fetchedMenuItems={shellState.authMenuItems}
    >
      <main className="container py-4">
        <div className="card shadow-sm">
          <div className="card-body p-0">
            <div className="table-responsive">
              <table className="table table-hover align-middle mb-0">
                <thead className="table-light">
                  <tr>
                    <th style={{ width: "40%" }}>Электронный адрес</th>
                    <th style={{ width: "40%" }}>Причина</th>
                    <th style={{ width: "20%" }}>Дата</th>
                  </tr>
                </thead>
                <tbody>
                  {data?.items.map((item) => (
                    <tr key={`${item.email}-${item.unsubscribedAt}`}>
                      <td>
                        <i className="bi-envelope me-1" />
                        <a href={`mailto:${item.email}`}>{item.email}</a>
                      </td>
                      <td>{item.reason ?? "—"}</td>
                      <td>{item.unsubscribedAt}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
          <div className="card-footer d-flex justify-content-between align-items-center">
            <div className="text-muted small">
              <i className="bi-info-circle me-1" />
              Всего {data?.items.length ?? 0} отписавшихся.
            </div>
          </div>
        </div>
      </main>
    </EmailerShell>
  );
}
