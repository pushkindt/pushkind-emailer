import { useEffect, useMemo, useState } from "react";
import type { FormEvent } from "react";

import {
  DropdownMultiSelect,
  type DropdownMultiSelectOption,
} from "@pushkind/frontend-shell/DropdownMultiSelect";
import {
  MarkdownComposer,
  renderMarkdownToHtml,
} from "@pushkind/frontend-shell/markdown";
import { EmailerShell } from "../components/EmailerShell";
import { EmailerShellFatalState } from "../components/EmailerShellFatalState";
import {
  fetchHubMenuItems,
  fetchIndexPageData,
  fetchShellData,
  isApiMutationError,
  postEmpty,
  postMultipartForm,
} from "../lib/api";
import type { IndexPageData, ShellData, UserMenuItem } from "../lib/models";
import { useServiceShell } from "@pushkind/frontend-shell/useServiceShell";

function splitDateTime(value: string) {
  const [date, time] = value.split(" ");
  return {
    date: date ?? value,
    time: time ?? "",
  };
}

type IndexPageContentProps = {
  data: IndexPageData;
  onSend: (body: FormData) => Promise<void>;
  onDelete: (emailId: number) => Promise<void>;
  onResend: (emailId: number) => Promise<void>;
  sending: boolean;
};

function IndexPageContent({
  data,
  onSend,
  onDelete,
  onResend,
  sending,
}: IndexPageContentProps) {
  const [selectedRecipients, setSelectedRecipients] = useState<string[]>(
    data.retryEmail?.recipientIds ?? [],
  );
  const [subject, setSubject] = useState(data.retryEmail?.subject ?? "");
  const [message, setMessage] = useState(data.retryEmail?.message ?? "");
  const [cooldownDays, setCooldownDays] = useState("");
  const [attachment, setAttachment] = useState<File | null>(null);

  useEffect(() => {
    setSelectedRecipients(data.retryEmail?.recipientIds ?? []);
    setSubject(data.retryEmail?.subject ?? "");
    setMessage(data.retryEmail?.message ?? "");
    setCooldownDays("");
    setAttachment(null);
  }, [data.retryEmail]);

  useEffect(() => {
    const storageKey = "savedMessageInput";
    const saved = localStorage.getItem(storageKey);
    if (saved !== null && !data.retryEmail) {
      setMessage(saved);
    }
  }, [data.retryEmail]);

  useEffect(() => {
    localStorage.setItem("savedMessageInput", message);
  }, [message]);

  const recipientOptions = useMemo<DropdownMultiSelectOption[]>(
    () => [
      ...data.groups.map((group) => ({
        value: String(group.id),
        label: `Группа: ${group.name}`,
        details: [] as string[],
      })),
      ...data.recipients.map((recipient) => ({
        value: recipient.id,
        label: recipient.text,
        details: Object.entries(recipient.fields).map(
          ([name, value]) => `${name}: ${value}`,
        ),
      })),
    ],
    [data.groups, data.recipients],
  );

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    const body = new FormData();
    body.set("message", renderMarkdownToHtml(message));
    body.set("subject", subject);
    body.set("cooldown_days", cooldownDays);
    if (attachment) {
      body.set("attachment", attachment);
    }
    body.set(
      "recipients",
      new Blob([JSON.stringify(selectedRecipients)], {
        type: "application/json",
      }),
    );

    await onSend(body);
  };

  return (
    <main className="container my-3">
      <div className="border-bottom my-2 pb-2">
        <form id="send-email-form" className="mb-2" onSubmit={handleSubmit}>
          <div className="row mb-1">
            <a
              className="recipientsDropDown"
              href="#"
              onClick={(event) => event.preventDefault()}
            >
              Получатели
            </a>
            <DropdownMultiSelect
              options={recipientOptions}
              selectedValues={selectedRecipients}
              onChange={setSelectedRecipients}
              className="emailer-dropdown-field"
              menuHeightClassName="emailer-dropdown-multiselect-options-lg"
              searchPlaceholder="Поиск получателей"
              clearable
              clearLabel="Очистить выбранных получателей"
            />
          </div>
          <div className="row">
            <div className="col">
              <input
                type="text"
                className="form-control mb-1"
                placeholder="Тема"
                value={subject}
                onChange={(event) => setSubject(event.currentTarget.value)}
              />
            </div>
          </div>
          <MarkdownComposer
            id="message-input"
            value={message}
            onChange={setMessage}
            className="mb-2"
            rows={10}
            required
            placeholder="Содержание в формате markdown"
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
            <div className="col-md">
              <input
                type="number"
                min="0"
                step="1"
                className="form-control mb-1"
                placeholder="фильтровать недавних получателей (дни)"
                value={cooldownDays}
                onChange={(event) => setCooldownDays(event.currentTarget.value)}
              />
            </div>
            <div className="col">
              <input
                className="form-control"
                type="file"
                onChange={(event) =>
                  setAttachment(event.currentTarget.files?.[0] ?? null)
                }
              />
            </div>
            <div className="col-auto text-end">
              <button
                className="btn btn-primary text-white"
                id="submit-button"
                type="submit"
                disabled={sending || selectedRecipients.length === 0}
              >
                {sending ? "Отправка..." : "Отправить"}
              </button>
            </div>
          </div>
        </form>
      </div>

      <div className="accordion" id="email-accordion">
        {data.emails.items.map((email) => {
          const createdAt = splitDateTime(email.createdAt);

          return (
            <div className="accordion-item" key={email.id}>
              <h2 className="accordion-header">
                <button
                  className={`accordion-button collapsed ${email.isSent ? "text-success" : ""}`}
                  type="button"
                  data-bs-toggle="collapse"
                  data-bs-target={`#email-collapse${email.id}`}
                  aria-expanded="false"
                  aria-controls={`email-collapse${email.id}`}
                >
                  <span className="d-inline-flex flex-column flex-shrink-0">
                    <span>{createdAt.date}</span>
                    <span>{createdAt.time}</span>
                  </span>
                  <span className="ms-2 d-inline-flex flex-wrap gap-3 emailer-email-stats">
                    <span className="d-inline-flex flex-column">
                      <span className="text-body-secondary">Отправлено</span>
                      <span>{email.numSent}</span>
                    </span>
                    <span className="d-inline-flex flex-column">
                      <span className="text-body-secondary">Открыли</span>
                      <span>{email.numOpened}</span>
                    </span>
                    <span className="d-inline-flex flex-column">
                      <span className="text-body-secondary">Ответили</span>
                      <span>{email.numReplied}</span>
                    </span>
                  </span>
                  <strong className="ms-2">"{email.subject ?? ""}"</strong>
                </button>
              </h2>
              <div
                id={`email-collapse${email.id}`}
                className="accordion-collapse collapse"
                data-bs-parent="#email-accordion"
              >
                <div className="accordion-body">
                  <div className="row border-bottom mb-1 pb-1">
                    <div className="col-auto">
                      <button
                        className="btn btn-danger btn-sm"
                        type="button"
                        onClick={() => void onDelete(email.id)}
                      >
                        <i className="bi bi-x-lg" />
                      </button>
                    </div>
                    <div className="col-auto">
                      <a
                        href={`?retry=${email.id}`}
                        className="btn btn-warning btn-sm"
                      >
                        <i className="bi bi-copy" />
                      </a>
                    </div>
                    <div className="col-auto">
                      <button
                        className="btn btn-danger btn-sm"
                        type="button"
                        onClick={() => void onResend(email.id)}
                      >
                        <i className="bi bi-arrow-clockwise" />
                      </button>
                    </div>
                    <div className="col-auto">
                      <a
                        href={`/email/${email.id}/recipients/export`}
                        className="btn btn-info btn-sm"
                      >
                        <i className="bi bi-download" />
                      </a>
                    </div>
                  </div>
                  <div className="row">
                    <div
                      className="col"
                      dangerouslySetInnerHTML={{ __html: email.messageHtml }}
                    />
                    <div className="col">
                      <ol className="list-group list-group-numbered">
                        {email.recipients.map((recipient) => (
                          <li
                            key={`${email.id}-${recipient.address}`}
                            className="list-group-item d-flex justify-content-between align-items-start"
                          >
                            <div className="ms-2 me-auto">
                              <div className="fw-bold">
                                <a
                                  className={
                                    recipient.reply
                                      ? "link-success"
                                      : recipient.opened
                                        ? "link-warning"
                                        : !recipient.isSent
                                          ? "link-secondary"
                                          : undefined
                                  }
                                  href={`${data.crmServiceUrl}?q=${encodeURIComponent(recipient.address)}`}
                                >
                                  {recipient.address}
                                </a>
                              </div>
                              {recipient.reply ? (
                                <div
                                  dangerouslySetInnerHTML={{
                                    __html: recipient.reply,
                                  }}
                                />
                              ) : null}
                            </div>
                            {recipient.isSent ||
                            recipient.opened ||
                            recipient.reply ? (
                              <span className="badge text-bg-primary rounded-pill d-inline-flex align-items-center gap-1">
                                {recipient.isSent ? (
                                  <i
                                    className="bi bi-envelope-check-fill"
                                    title="Сообщение отправлено"
                                  />
                                ) : null}
                                {recipient.opened ? (
                                  <i
                                    className="bi bi-envelope-open-fill"
                                    title="Сообщение просмотрено"
                                  />
                                ) : null}
                                {recipient.reply ? (
                                  <i
                                    className="bi bi-reply-fill"
                                    title="Получен ответ на сообщение"
                                  />
                                ) : null}
                              </span>
                            ) : null}
                          </li>
                        ))}
                      </ol>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          );
        })}
      </div>

      {data.emails.pages.length > 1 ? (
        <nav aria-label="pagination" className="mt-3">
          <ul
            className="pagination justify-content-center flex-wrap"
            id="pagination"
          >
            {data.emails.pages.map((page, index) =>
              page ? (
                page !== data.emails.page ? (
                  <li className="page-item" key={`${page}-${index}`}>
                    <a className="page-link" href={`?page=${page}`}>
                      {page}
                    </a>
                  </li>
                ) : (
                  <li
                    className="page-item active"
                    aria-current="page"
                    key={`${page}-${index}`}
                  >
                    <span className="page-link">{page}</span>
                  </li>
                )
              ) : (
                <li className="page-item" key={`ellipsis-${index}`}>
                  <span className="ellipsis">…</span>
                </li>
              ),
            )}
          </ul>
        </nav>
      ) : null}
    </main>
  );
}

export function IndexBootstrap() {
  const shellState = useServiceShell<ShellData, UserMenuItem>({
    errorMessage: "Не удалось загрузить оболочку Emailer.",
    menuLoadWarning:
      "Failed to load auth navigation menu. Falling back to local Emailer menu only.",
    fetchShellData,
    fetchHubMenuItems,
  });
  const [pageState, setPageState] = useState<
    | { status: "loading" }
    | { status: "ready"; data: IndexPageData }
    | { status: "error"; message: string }
  >({ status: "loading" });
  const [sending, setSending] = useState(false);

  const loadIndexPage = async () => {
    const data = await fetchIndexPageData(
      new URLSearchParams(window.location.search),
    );
    setPageState({ status: "ready", data });
  };

  useEffect(() => {
    let active = true;

    void fetchIndexPageData(new URLSearchParams(window.location.search))
      .then((data) => {
        if (!active) {
          return;
        }

        setPageState({ status: "ready", data });
      })
      .catch((error) => {
        if (!active) {
          return;
        }

        setPageState({
          status: "error",
          message:
            error instanceof Error
              ? error.message
              : "Не удалось загрузить страницу Emailer.",
        });
      });

    return () => {
      active = false;
    };
  }, []);

  const handleSend = async (body: FormData) => {
    setSending(true);

    try {
      const response = await postMultipartForm("/email/send", body);
      window.showFlashMessage?.(response.message, "primary");
      localStorage.removeItem("savedMessageInput");
      await loadIndexPage();
    } catch (error) {
      if (isApiMutationError(error)) {
        window.showFlashMessage?.(error.message, "danger");
      } else {
        console.error("Failed to send email.", error);
        window.showFlashMessage?.(
          "Ошибка при добавлении сообщения в очередь.",
          "danger",
        );
      }
    } finally {
      setSending(false);
    }
  };

  const handleDelete = async (emailId: number) => {
    if (!window.confirm("Удалить?")) {
      return;
    }
    try {
      const response = await postEmpty(`/email/${emailId}/delete`);
      window.showFlashMessage?.(response.message, "primary");
      await loadIndexPage();
    } catch (error) {
      if (isApiMutationError(error)) {
        window.showFlashMessage?.(error.message, "danger");
      } else {
        console.error("Failed to delete email.", error);
        window.showFlashMessage?.("Ошибка при удалении сообщения.", "danger");
      }
    }
  };

  const handleResend = async (emailId: number) => {
    if (!window.confirm("Отправить неотправленные?")) {
      return;
    }
    try {
      const response = await postEmpty(`/email/${emailId}/resend`);
      window.showFlashMessage?.(response.message, "primary");
      await loadIndexPage();
    } catch (error) {
      if (isApiMutationError(error)) {
        window.showFlashMessage?.(error.message, "danger");
      } else {
        console.error("Failed to resend email.", error);
        window.showFlashMessage?.(
          "Ошибка при повторной отправке сообщения.",
          "danger",
        );
      }
    }
  };

  if (shellState.status === "loading" || pageState.status === "loading") {
    return null;
  }

  if (shellState.status === "error") {
    return <EmailerShellFatalState message={shellState.message} />;
  }

  if (pageState.status === "error") {
    return <EmailerShellFatalState message={pageState.message} />;
  }

  return (
    <EmailerShell
      navigation={shellState.shell.navigation}
      currentUserEmail={shellState.shell.currentUser.email}
      homeUrl={shellState.shell.homeUrl}
      localMenuItems={shellState.shell.localMenuItems}
      fetchedMenuItems={shellState.authMenuItems}
    >
      <IndexPageContent
        data={pageState.data}
        onSend={handleSend}
        onDelete={handleDelete}
        onResend={handleResend}
        sending={sending}
      />
    </EmailerShell>
  );
}
