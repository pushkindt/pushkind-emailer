import { useEffect, useState } from "react";
import type { FormEvent } from "react";
import { MarkdownComposer } from "@pushkind/frontend-shell/markdown";
import { EmailerShell } from "../components/EmailerShell";
import { EmailerShellFatalState } from "../components/EmailerShellFatalState";
import {
  fetchHubMenuItems,
  fetchSettingsPageData,
  fetchShellData,
  isApiMutationError,
  postForm,
  toFieldErrorMap,
  type FieldErrorMap,
} from "../lib/api";
import type { SettingsPageData, ShellData, UserMenuItem } from "../lib/models";
import { useServiceShell } from "@pushkind/frontend-shell/useServiceShell";

export function SettingsBootstrap() {
  const shellState = useServiceShell<ShellData, UserMenuItem>({
    errorMessage: "Не удалось загрузить оболочку Emailer.",
    menuLoadWarning:
      "Failed to load auth navigation menu. Falling back to local Emailer menu only.",
    fetchShellData,
    fetchHubMenuItems,
  });
  const [data, setData] = useState<SettingsPageData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [fieldErrors, setFieldErrors] = useState<FieldErrorMap>({});
  const [form, setForm] = useState({
    login: "",
    password: "",
    sender: "",
    smtpServer: "",
    smtpPort: "",
    imapServer: "",
    imapPort: "",
    message: "",
  });

  useEffect(() => {
    void fetchSettingsPageData()
      .then((pageData) => {
        setData(pageData);
        setForm({
          login: pageData.login ?? "",
          password: pageData.password ?? "",
          sender: pageData.sender ?? "",
          smtpServer: pageData.smtpServer ?? "",
          smtpPort: pageData.smtpPort == null ? "" : String(pageData.smtpPort),
          imapServer: pageData.imapServer ?? "",
          imapPort: pageData.imapPort == null ? "" : String(pageData.imapPort),
          message: pageData.message ?? "",
        });
      })
      .catch((err) => {
        setError(
          err instanceof Error
            ? err.message
            : "Не удалось загрузить настройки.",
        );
      });
  }, []);

  if (shellState.status === "loading" || (data == null && error == null))
    return null;
  if (shellState.status === "error")
    return <EmailerShellFatalState message={shellState.message} />;
  if (error != null) return <EmailerShellFatalState message={error} />;
  if (data == null) return null;

  const fieldError = (field: string) => fieldErrors[field]?.[0];

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setSaving(true);
    try {
      const body = new URLSearchParams();
      body.set("login", form.login);
      body.set("password", form.password);
      body.set("sender", form.sender);
      body.set("smtp_server", form.smtpServer);
      body.set("smtp_port", form.smtpPort);
      body.set("imap_server", form.imapServer);
      body.set("imap_port", form.imapPort);
      body.set("message", form.message);
      const response = await postForm("/settings/save", body);
      setFieldErrors({});
      window.showFlashMessage?.(response.message, "primary");
    } catch (err) {
      if (isApiMutationError(err)) {
        setFieldErrors(toFieldErrorMap(err));
        window.showFlashMessage?.(err.message, "danger");
      } else {
        console.error("Failed to save settings.", err);
        window.showFlashMessage?.("Не удалось сохранить настройки.", "danger");
      }
    } finally {
      setSaving(false);
    }
  };

  return (
    <EmailerShell
      navigation={shellState.shell.navigation}
      currentUserEmail={shellState.shell.currentUser.email}
      homeUrl={shellState.shell.homeUrl}
      localMenuItems={shellState.shell.localMenuItems}
      fetchedMenuItems={shellState.authMenuItems}
    >
      <main className="container my-2">
        <form className="my-2" onSubmit={handleSubmit}>
          {[
            ["Логин", "login", form.login],
            ["Пароль", "password", form.password],
            ["Отправитель", "sender", form.sender],
            ["SMTP сервер", "smtpServer", form.smtpServer],
            ["SMTP порт", "smtpPort", form.smtpPort],
            ["IMAP сервер", "imapServer", form.imapServer],
            ["IMAP порт", "imapPort", form.imapPort],
          ].map(([label, key, value]) => (
            <div className="row mb-3" key={String(key)}>
              <label className="col-sm-2 col-form-label">{label}</label>
              <div className="col-sm-10">
                <input
                  type={key.toString().includes("Port") ? "number" : "text"}
                  className="form-control"
                  value={String(value)}
                  onChange={(event) => {
                    const nextValue = event.currentTarget.value;

                    setForm((current) => ({
                      ...current,
                      [key]: nextValue,
                    }));
                  }}
                />
                {fieldError(
                  key === "smtpServer"
                    ? "smtp_server"
                    : key === "smtpPort"
                      ? "smtp_port"
                      : key === "imapServer"
                        ? "imap_server"
                        : key === "imapPort"
                          ? "imap_port"
                          : String(key),
                ) ? (
                  <div className="invalid-feedback d-block">
                    {fieldError(
                      key === "smtpServer"
                        ? "smtp_server"
                        : key === "smtpPort"
                          ? "smtp_port"
                          : key === "imapServer"
                            ? "imap_server"
                            : key === "imapPort"
                              ? "imap_port"
                              : String(key),
                    )}
                  </div>
                ) : null}
              </div>
            </div>
          ))}
          <h6>
            Шаблон сообщения (доступны переменные{" "}
            {"{name} {message} {unsubscribe_url}"}):
          </h6>
          <MarkdownComposer
            id="settings-message-input"
            value={form.message}
            onChange={(nextValue) => {
              setForm((current) => ({
                ...current,
                message: nextValue,
              }));
            }}
            className="mb-2"
            rows={10}
            editorLabel="Маркдаун"
            previewLabel="Превью"
            fileBrowserLabel="Файлы"
            previewClassName="emailer-markdown-preview"
            emptyPreviewLabel="Введите markdown, чтобы увидеть превью."
            fileBrowser={
              data.filesServiceUrl
                ? {
                    baseUrl: data.filesServiceUrl,
                    helpText:
                      "Загрузите или найдите файл, скопируйте ссылку и вставьте её в markdown.",
                  }
                : undefined
            }
          />
          {fieldError("message") ? (
            <div className="invalid-feedback d-block">
              {fieldError("message")}
            </div>
          ) : null}
          <div className="row mb-2">
            <div className="col">
              <small className="text-muted">
                Сообщение в формате{" "}
                <a href="https://www.markdownguide.org/basic-syntax/">
                  markdown
                </a>
                . Подставляются {"{теги}"} получателей.
              </small>
            </div>
          </div>
          <div className="row">
            <div className="col">
              <button
                type="submit"
                className="btn btn-primary"
                disabled={saving}
              >
                {saving ? "Сохранение..." : "Сохранить"}
              </button>
            </div>
          </div>
        </form>
      </main>
    </EmailerShell>
  );
}
