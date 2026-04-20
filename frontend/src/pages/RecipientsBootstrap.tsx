import { useEffect, useRef, useState } from "react";
import type { ChangeEvent, FormEvent } from "react";

import {
  DropdownMultiSelect,
  type DropdownMultiSelectOption,
} from "@pushkind/frontend-shell/DropdownMultiSelect";
import { EmailerShell } from "../components/EmailerShell";
import { EmailerShellFatalState } from "../components/EmailerShellFatalState";
import {
  fetchHubMenuItems,
  fetchRecipientModalData,
  fetchRecipientsPageData,
  fetchShellData,
  isApiMutationError,
  postEmpty,
  postForm,
  postMultipartForm,
  toFieldErrorMap,
  type FieldErrorMap,
} from "../lib/api";
import type {
  RecipientField,
  RecipientModalData,
  RecipientsPageData,
  ShellData,
  UserMenuItem,
} from "../lib/models";
import { useServiceShell } from "@pushkind/frontend-shell/useServiceShell";

type PageState =
  | { status: "loading" }
  | { status: "ready"; data: RecipientsPageData }
  | { status: "error"; message: string };

type RecipientModalState =
  | { status: "closed" }
  | { status: "loading"; recipientId: number }
  | { status: "error"; recipientId: number; message: string }
  | { status: "ready"; data: RecipientModalData };

type RecipientFieldRow = RecipientField & {
  rowId: number;
};

function buildSearchUrl(page?: number) {
  const params = new URLSearchParams(window.location.search);

  if (page == null || page === 1) {
    params.delete("page");
  } else {
    params.set("page", String(page));
  }

  const query = params.toString();
  return query ? `/recipients?${query}` : "/recipients";
}

export function RecipientsBootstrap() {
  const shellState = useServiceShell<ShellData, UserMenuItem>({
    errorMessage: "Не удалось загрузить оболочку Emailer.",
    menuLoadWarning:
      "Failed to load auth navigation menu. Falling back to local Emailer menu only.",
    fetchShellData,
    fetchHubMenuItems,
  });
  const [pageState, setPageState] = useState<PageState>({ status: "loading" });
  const [modalState, setModalState] = useState<RecipientModalState>({
    status: "closed",
  });
  const modalRequestIdRef = useRef(0);
  const recipientModalRef = useRef<HTMLDivElement | null>(null);
  const recipientFieldRowIdRef = useRef(0);
  const [addName, setAddName] = useState("");
  const [addEmail, setAddEmail] = useState("");
  const [source, setSource] = useState("");
  const [uploadFile, setUploadFile] = useState<File | null>(null);
  const [isSubmittingAdd, setIsSubmittingAdd] = useState(false);
  const [isSubmittingSource, setIsSubmittingSource] = useState(false);
  const [isSubmittingUpload, setIsSubmittingUpload] = useState(false);
  const [isCleaning, setIsCleaning] = useState(false);
  const [isSavingRecipient, setIsSavingRecipient] = useState(false);
  const [isDeletingRecipient, setIsDeletingRecipient] = useState(false);
  const [addFieldErrors, setAddFieldErrors] = useState<FieldErrorMap>({});
  const [sourceFieldErrors, setSourceFieldErrors] = useState<FieldErrorMap>({});
  const [saveFieldErrors, setSaveFieldErrors] = useState<FieldErrorMap>({});
  const [recipientName, setRecipientName] = useState("");
  const [recipientGroupIds, setRecipientGroupIds] = useState<number[]>([]);
  const [recipientFields, setRecipientFields] = useState<RecipientFieldRow[]>(
    [],
  );

  const withRowIds = (fields: RecipientField[]): RecipientFieldRow[] =>
    fields.map((field) => ({
      ...field,
      rowId: recipientFieldRowIdRef.current++,
    }));

  const updateRecipientFieldRow = (
    rowId: number,
    patch: Partial<RecipientField>,
  ) => {
    setRecipientFields((current) =>
      current.map((item) =>
        item.rowId === rowId ? { ...item, ...patch } : item,
      ),
    );
  };

  const removeRecipientFieldRow = (rowId: number) => {
    setRecipientFields((current) =>
      current.filter((item) => item.rowId !== rowId),
    );
  };

  const getModalInstance = () => {
    if (recipientModalRef.current == null) {
      return null;
    }

    return window.bootstrap?.Modal.getOrCreateInstance(
      recipientModalRef.current,
    );
  };

  const loadPage = async (showLoadingState = true) => {
    if (showLoadingState) {
      setPageState({ status: "loading" });
    }

    try {
      const data = await fetchRecipientsPageData(
        new URLSearchParams(window.location.search),
      );
      setPageState({ status: "ready", data });
    } catch (error) {
      setPageState({
        status: "error",
        message:
          error instanceof Error
            ? error.message
            : "Не удалось загрузить страницу получателей.",
      });
    }
  };

  useEffect(() => {
    void loadPage();
  }, []);

  const closeRecipientModal = () => {
    setSaveFieldErrors({});
    setModalState({ status: "closed" });
  };

  const openRecipientModal = (recipientId: number) => {
    modalRequestIdRef.current += 1;
    const requestId = modalRequestIdRef.current;

    setSaveFieldErrors({});
    setModalState({ status: "loading", recipientId });
    getModalInstance()?.show();

    void fetchRecipientModalData(recipientId)
      .then((data) => {
        if (modalRequestIdRef.current !== requestId) {
          return;
        }

        setRecipientName(data.recipient.name);
        setRecipientGroupIds(data.recipient.groupIds);
        setRecipientFields(withRowIds(data.recipient.fields));
        setModalState({ status: "ready", data });
      })
      .catch((error) => {
        if (modalRequestIdRef.current !== requestId) {
          return;
        }

        setModalState({
          status: "error",
          recipientId,
          message:
            error instanceof Error
              ? error.message
              : "Не удалось загрузить получателя.",
        });
      });
  };

  const showError = (error: unknown, fallbackMessage: string) => {
    if (isApiMutationError(error)) {
      window.showFlashMessage?.(error.message, "danger");
      return;
    }

    console.error(fallbackMessage, error);
    window.showFlashMessage?.(fallbackMessage, "danger");
  };

  const fieldError = (errors: FieldErrorMap, field: string) =>
    errors[field]?.[0];

  if (shellState.status === "loading" || pageState.status === "loading") {
    return null;
  }

  if (shellState.status === "error") {
    return <EmailerShellFatalState message={shellState.message} />;
  }

  if (pageState.status === "error") {
    return <EmailerShellFatalState message={pageState.message} />;
  }

  const sourceOptions = [
    { label: "CRM", value: `${pageState.data.crmServiceUrl}/api/v1/clients` },
    { label: "USERS", value: `${shellState.shell.homeUrl}/api/v1/users` },
  ];
  const groupOptions: DropdownMultiSelectOption[] =
    modalState.status === "ready"
      ? modalState.data.groups.map((group) => ({
          value: String(group.id),
          label: group.name,
        }))
      : [];

  const handleAddRecipient = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setIsSubmittingAdd(true);

    try {
      const body = new URLSearchParams();
      body.set("name", addName);
      body.set("email", addEmail);
      const response = await postForm("/recipient/add", body);
      setAddFieldErrors({});
      window.showFlashMessage?.(response.message, "primary");
      setAddName("");
      setAddEmail("");
      await loadPage(false);
    } catch (error) {
      if (isApiMutationError(error)) {
        setAddFieldErrors(toFieldErrorMap(error));
        window.showFlashMessage?.(error.message, "danger");
      } else {
        showError(error, "Не удалось добавить получателя.");
      }
    } finally {
      setIsSubmittingAdd(false);
    }
  };

  const handleSourceImport = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setIsSubmittingSource(true);

    try {
      const body = new URLSearchParams();
      body.set("source", source);
      const response = await postForm("/recipients/source", body);
      setSourceFieldErrors({});
      window.showFlashMessage?.(response.message, "primary");
      await loadPage(false);
    } catch (error) {
      if (isApiMutationError(error)) {
        setSourceFieldErrors(toFieldErrorMap(error));
        window.showFlashMessage?.(error.message, "danger");
      } else {
        showError(error, "Не удалось загрузить получателей из источника.");
      }
    } finally {
      setIsSubmittingSource(false);
    }
  };

  const handleUpload = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (uploadFile == null) {
      return;
    }

    setIsSubmittingUpload(true);

    try {
      const body = new FormData();
      body.set("csv", uploadFile);
      const response = await postMultipartForm("/recipients/upload", body);
      window.showFlashMessage?.(response.message, "primary");
      setUploadFile(null);
      event.currentTarget.reset();
      await loadPage(false);
    } catch (error) {
      showError(error, "Не удалось загрузить CSV с получателями.");
    } finally {
      setIsSubmittingUpload(false);
    }
  };

  const handleClean = async () => {
    if (!window.confirm("Удалить всё?")) {
      return;
    }

    setIsCleaning(true);

    try {
      const response = await postEmpty("/recipients/clean");
      window.showFlashMessage?.(response.message, "primary");
      await loadPage(false);
    } catch (error) {
      showError(error, "Не удалось очистить список получателей.");
    } finally {
      setIsCleaning(false);
    }
  };

  const handleSaveRecipient = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (modalState.status !== "ready") {
      return;
    }

    setIsSavingRecipient(true);

    try {
      const body = new URLSearchParams();
      body.set("name", recipientName);
      body.set("email", modalState.data.recipient.email);

      recipientGroupIds.forEach((groupId) => {
        body.append("groups", String(groupId));
      });

      recipientFields.forEach((field) => {
        body.append("field", field.name);
        body.append("value", field.value);
      });

      const response = await postForm(
        `/recipient/${modalState.data.recipient.id}/save`,
        body,
      );
      setSaveFieldErrors({});
      window.showFlashMessage?.(response.message, "primary");
      getModalInstance()?.hide();
      closeRecipientModal();
      await loadPage(false);
    } catch (error) {
      if (isApiMutationError(error)) {
        setSaveFieldErrors(toFieldErrorMap(error));
        window.showFlashMessage?.(error.message, "danger");
      } else {
        showError(error, "Не удалось сохранить получателя.");
      }
    } finally {
      setIsSavingRecipient(false);
    }
  };

  const handleDeleteRecipient = async () => {
    if (modalState.status !== "ready") {
      return;
    }
    if (!window.confirm("Удалить?")) {
      return;
    }

    setIsDeletingRecipient(true);

    try {
      const response = await postEmpty(
        `/recipient/${modalState.data.recipient.id}/delete`,
      );
      window.showFlashMessage?.(response.message, "primary");
      getModalInstance()?.hide();
      closeRecipientModal();
      await loadPage(false);
    } catch (error) {
      showError(error, "Не удалось удалить получателя.");
    } finally {
      setIsDeletingRecipient(false);
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
        <div className="row">
          <div className="col-lg-6">
            <h5>Получатели</h5>
            <form onSubmit={handleAddRecipient}>
              <div className="row my-1">
                <div className="col-lg">
                  <input
                    className="form-control"
                    type="text"
                    name="name"
                    placeholder="Имя"
                    required
                    value={addName}
                    onChange={(event) => {
                      setAddName(event.currentTarget.value);
                      setAddFieldErrors((current) => ({
                        ...current,
                        name: [],
                      }));
                    }}
                  />
                  {fieldError(addFieldErrors, "name") ? (
                    <div className="invalid-feedback d-block">
                      {fieldError(addFieldErrors, "name")}
                    </div>
                  ) : null}
                </div>
                <div className="col-lg">
                  <input
                    className="form-control"
                    type="email"
                    name="email"
                    placeholder="Электронный адрес"
                    required
                    value={addEmail}
                    onChange={(event) => {
                      setAddEmail(event.currentTarget.value);
                      setAddFieldErrors((current) => ({
                        ...current,
                        email: [],
                      }));
                    }}
                  />
                  {fieldError(addFieldErrors, "email") ? (
                    <div className="invalid-feedback d-block">
                      {fieldError(addFieldErrors, "email")}
                    </div>
                  ) : null}
                </div>
                <div className="col-lg-3 text-end">
                  <button
                    className="btn btn-primary"
                    type="submit"
                    disabled={isSubmittingAdd}
                  >
                    {isSubmittingAdd ? "Добавление..." : "Добавить"}
                  </button>
                </div>
              </div>
            </form>
            <form onSubmit={handleSourceImport}>
              <div className="row my-1">
                <div className="col">
                  <select
                    className="form-select"
                    aria-label="API source"
                    required
                    name="source"
                    value={source}
                    onChange={(event) => {
                      setSource(event.currentTarget.value);
                      setSourceFieldErrors((current) => ({
                        ...current,
                        source: [],
                      }));
                    }}
                  >
                    <option value="">Выбор сервиса</option>
                    {sourceOptions.map((option) => (
                      <option key={option.label} value={option.value}>
                        {option.label}
                      </option>
                    ))}
                  </select>
                  {fieldError(sourceFieldErrors, "source") ? (
                    <div className="invalid-feedback d-block">
                      {fieldError(sourceFieldErrors, "source")}
                    </div>
                  ) : null}
                </div>
                <div className="col-lg-3 text-end">
                  <button
                    className="btn btn-primary"
                    type="submit"
                    disabled={isSubmittingSource}
                  >
                    {isSubmittingSource ? "Загрузка..." : "Загрузить"}
                  </button>
                </div>
              </div>
            </form>
          </div>
          <div className="col-lg-6">
            <h5>Массовая загрузка/удаление</h5>
            <form onSubmit={handleUpload}>
              <div className="row my-1">
                <div className="col-lg">
                  <input
                    className="form-control"
                    type="file"
                    name="csv"
                    accept=".csv"
                    required
                    onChange={(event: ChangeEvent<HTMLInputElement>) =>
                      setUploadFile(event.currentTarget.files?.[0] ?? null)
                    }
                  />
                </div>
                <div className="col-lg-3 text-end">
                  <button
                    className="btn btn-primary"
                    type="submit"
                    disabled={isSubmittingUpload}
                  >
                    {isSubmittingUpload ? "Загрузка..." : "Загрузить"}
                  </button>
                </div>
              </div>
            </form>
            <div className="row my-1">
              <div className="col-lg">
                <small className="text-muted">
                  "name","email","group1,group2","произвольные","поля"
                </small>
              </div>
              <div className="col-lg-3 text-end">
                <button
                  className="btn btn-danger my-1"
                  type="button"
                  disabled={isCleaning}
                  onClick={() => void handleClean()}
                >
                  {isCleaning ? "Очистка..." : "Очистить"}
                </button>
              </div>
            </div>
          </div>
        </div>

        <div className="container mb-1 px-0">
          <form method="GET" action="/recipients">
            <div className="row">
              <div className="col">
                <div className="input-group me-2">
                  <input
                    name="q"
                    className="form-control"
                    type="search"
                    placeholder="Поиск"
                    aria-label="Search"
                    defaultValue={pageState.data.searchQuery ?? ""}
                  />
                  <button className="btn btn-outline-secondary" type="submit">
                    <i className="bi bi-search" />
                  </button>
                </div>
              </div>
            </div>
          </form>
        </div>

        <div className="container border bg-white px-0">
          <div className="row mb-3 fw-bold px-3 pt-3">
            <div className="col overflow-hidden">Имя</div>
            <div className="col overflow-hidden">Email</div>
            <div className="col overflow-hidden">Теги</div>
          </div>
          {pageState.data.recipients.items.map((recipient) => (
            <div
              key={recipient.id}
              className="row mb-3 border-bottom selectable px-3 pb-3 emailer-selectable-row"
              role="button"
              onClick={() => openRecipientModal(recipient.id)}
            >
              <div className="col overflow-hidden">{recipient.name}</div>
              <div className="col overflow-hidden">{recipient.email}</div>
              <div className="col overflow-hidden">
                {Object.entries(recipient.fields).map(([field, value]) => (
                  <span
                    key={`${recipient.id}-${field}`}
                    className="badge rounded-pill text-bg-light me-1"
                  >
                    {value.length > 15 ? `${value.slice(0, 15)}…` : value}
                  </span>
                ))}
              </div>
            </div>
          ))}

          {pageState.data.recipients.items.length === 0 ? (
            <div className="px-3 pb-3 text-muted">Получатели не найдены.</div>
          ) : null}

          {pageState.data.recipients.pages.length > 1 ? (
            <nav aria-label="pagination" className="mt-3">
              <ul className="pagination justify-content-center flex-wrap">
                {pageState.data.recipients.pages.map((page, index) =>
                  page ? (
                    page !== pageState.data.recipients.page ? (
                      <li className="page-item" key={`${page}-${index}`}>
                        <a className="page-link" href={buildSearchUrl(page)}>
                          {page}
                        </a>
                      </li>
                    ) : (
                      <li className="page-item active" key={`${page}-${index}`}>
                        <span className="page-link">{page}</span>
                      </li>
                    )
                  ) : (
                    <li
                      className="page-item disabled"
                      key={`ellipsis-${index}`}
                    >
                      <span className="page-link">…</span>
                    </li>
                  ),
                )}
              </ul>
            </nav>
          ) : null}
        </div>

        <div
          className="modal fade"
          id="recipientModal"
          ref={recipientModalRef}
          tabIndex={-1}
          aria-labelledby="recipientModalLabel"
          aria-hidden="true"
        >
          <div className="modal-dialog modal-lg">
            <div className="modal-content">
              <div className="modal-header">
                <h1 className="modal-title fs-5" id="recipientModalLabel">
                  Редактировать получателя
                </h1>
                <button
                  type="button"
                  className="btn-close"
                  data-bs-dismiss="modal"
                  aria-label="Close"
                  onClick={closeRecipientModal}
                />
              </div>
              {modalState.status === "loading" ? (
                <div className="modal-body">Загрузка...</div>
              ) : null}
              {modalState.status === "error" ? (
                <div className="modal-body">
                  <div className="alert alert-danger mb-0">
                    {modalState.message}
                  </div>
                </div>
              ) : null}
              {modalState.status === "ready" ? (
                <>
                  <div className="modal-body">
                    <form onSubmit={handleSaveRecipient}>
                      <div className="row mb-3">
                        <label
                          htmlFor="modalUserName"
                          className="col-md-2 col-form-label"
                        >
                          Имя
                        </label>
                        <div className="col-md-10">
                          <input
                            name="name"
                            type="text"
                            className="form-control"
                            id="modalUserName"
                            value={recipientName}
                            placeholder="Имя"
                            required
                            onChange={(event) => {
                              setRecipientName(event.currentTarget.value);
                              setSaveFieldErrors((current) => ({
                                ...current,
                                name: [],
                              }));
                            }}
                          />
                          {fieldError(saveFieldErrors, "name") ? (
                            <div className="invalid-feedback d-block">
                              {fieldError(saveFieldErrors, "name")}
                            </div>
                          ) : null}
                          {modalState.data.recipient.unsubscribedAt ? (
                            <small className="text-body-secondary">
                              Получатель отписался{" "}
                              {modalState.data.recipient.unsubscribedAt}
                            </small>
                          ) : null}
                        </div>
                      </div>
                      <div className="row mb-3">
                        <label
                          htmlFor="modalUserEmail"
                          className="col-md-2 col-form-label"
                        >
                          Электронный адрес
                        </label>
                        <div className="col-md-10">
                          <input
                            name="email"
                            readOnly
                            type="email"
                            className="form-control-plaintext"
                            id="modalUserEmail"
                            value={modalState.data.recipient.email}
                            placeholder="Электронный адрес"
                          />
                        </div>
                      </div>
                      <div className="row mb-3">
                        <label
                          htmlFor="recipients-assign-form-group-id"
                          className="col-md-2 col-form-label"
                        >
                          Группы
                        </label>
                        <div className="col-md-10">
                          <DropdownMultiSelect
                            id="recipients-assign-form-group-id"
                            options={groupOptions}
                            selectedValues={recipientGroupIds.map(String)}
                            onChange={(values) =>
                              setRecipientGroupIds(values.map(Number))
                            }
                            className="my-1"
                            menuHeightClassName="emailer-dropdown-multiselect-options-sm"
                            searchPlaceholder="Поиск групп"
                            clearable
                            clearLabel="Очистить выбранные группы"
                          />
                          {fieldError(saveFieldErrors, "groups") ? (
                            <div className="invalid-feedback d-block">
                              {fieldError(saveFieldErrors, "groups")}
                            </div>
                          ) : null}
                        </div>
                      </div>

                      <div id="custom-fields">
                        {recipientFields.map((field) => (
                          <div className="row mb-3" key={field.rowId}>
                            <div className="col">
                              <input
                                type="text"
                                className="form-control"
                                value={field.name}
                                required
                                placeholder="Поле"
                                onChange={(event) => {
                                  updateRecipientFieldRow(field.rowId, {
                                    name: event.currentTarget.value,
                                  });
                                }}
                              />
                            </div>
                            <div className="col">
                              <input
                                type="text"
                                className="form-control"
                                value={field.value}
                                required
                                placeholder="Значение"
                                onChange={(event) => {
                                  updateRecipientFieldRow(field.rowId, {
                                    value: event.currentTarget.value,
                                  });
                                }}
                              />
                            </div>
                            <div className="col-auto">
                              <button
                                type="button"
                                className="btn btn-danger btn-sm"
                                onClick={() =>
                                  removeRecipientFieldRow(field.rowId)
                                }
                              >
                                <i className="bi bi-slash-circle" />
                              </button>
                            </div>
                          </div>
                        ))}
                      </div>
                      {fieldError(saveFieldErrors, "field") ? (
                        <div className="invalid-feedback d-block mb-3">
                          {fieldError(saveFieldErrors, "field")}
                        </div>
                      ) : null}
                      {fieldError(saveFieldErrors, "value") ? (
                        <div className="invalid-feedback d-block mb-3">
                          {fieldError(saveFieldErrors, "value")}
                        </div>
                      ) : null}

                      <div className="row mb-3">
                        <div className="col">
                          <button
                            className="btn btn-primary"
                            type="submit"
                            disabled={isSavingRecipient}
                          >
                            {isSavingRecipient ? "Сохранение..." : "Сохранить"}
                          </button>
                        </div>
                        <div className="col-auto">
                          <button
                            type="button"
                            className="btn btn-primary btn-sm"
                            onClick={() =>
                              setRecipientFields((current) => [
                                ...current,
                                {
                                  rowId: recipientFieldRowIdRef.current++,
                                  name: "",
                                  value: "",
                                },
                              ])
                            }
                          >
                            <i className="bi bi-plus" />
                          </button>
                        </div>
                      </div>
                    </form>
                  </div>
                  <div className="modal-footer">
                    <button
                      className="btn btn-danger"
                      type="button"
                      disabled={isDeletingRecipient}
                      onClick={() => void handleDeleteRecipient()}
                    >
                      {isDeletingRecipient ? "Удаление..." : "Удалить"}
                    </button>
                  </div>
                </>
              ) : null}
            </div>
          </div>
        </div>
      </main>
    </EmailerShell>
  );
}
