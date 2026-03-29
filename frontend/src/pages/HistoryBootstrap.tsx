import { useEffect, useState } from "react";
import { EmailerShell } from "../components/EmailerShell";
import { EmailerShellFatalState } from "../components/EmailerShellFatalState";
import { fetchHistoryPageData } from "../lib/api";
import type { HistoryPageData } from "../lib/models";
import { useEmailerShell } from "../lib/useEmailerShell";

export function HistoryBootstrap() {
  const shellState = useEmailerShell("Не удалось загрузить оболочку Emailer.");
  const [data, setData] = useState<HistoryPageData | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void fetchHistoryPageData()
      .then(setData)
      .catch((err) =>
        setError(
          err instanceof Error ? err.message : "Не удалось загрузить историю.",
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
              <table className="table table-hover align-middle mb-0 caption-top">
                <caption className="px-2">
                  <div className="row">
                    <div className="col">
                      Отображается только дата последнего письма для каждого
                      получателя
                    </div>
                    <div className="col-auto">
                      <a href="/history/download">
                        <i className="bi bi-download" />
                      </a>
                    </div>
                  </div>
                </caption>
                <thead className="table-light">
                  <tr>
                    <th style={{ width: "40%" }}>Получатель</th>
                    <th style={{ width: "20%" }}>Дата</th>
                    <th style={{ width: "20%" }}>Просмотрено</th>
                    <th style={{ width: "20%" }}>Отвечено</th>
                  </tr>
                </thead>
                <tbody>
                  {data?.items.map((item) => (
                    <tr key={`${item.address}-${item.updatedAt}`}>
                      <td>
                        {item.name ? (
                          <>
                            {item.name}
                            <br />
                          </>
                        ) : null}
                        <i className="bi-envelope me-1" />
                        <a
                          href={`${data.crmServiceUrl}?q=${encodeURIComponent(item.address)}`}
                        >
                          {item.address}
                        </a>
                      </td>
                      <td>{item.updatedAt}</td>
                      <td>
                        {item.opened ? (
                          <i className="bi bi-plus-circle" />
                        ) : null}
                      </td>
                      <td>
                        {item.replied ? (
                          <i className="bi bi-plus-circle" />
                        ) : null}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
          <div className="card-footer d-flex justify-content-between align-items-center">
            <div className="text-muted small">
              <i className="bi-info-circle me-1" />
              Всего {data?.items.length ?? 0} получателей.
            </div>
          </div>
        </div>
      </main>
    </EmailerShell>
  );
}
