import { useRef, useState, useEffect } from "react";
import type { FormEvent } from "react";

import {
  DropdownMultiSelect,
  type DropdownMultiSelectOption,
} from "../components/DropdownMultiSelect";
import { EmailerShell } from "../components/EmailerShell";
import { EmailerShellFatalState } from "../components/EmailerShellFatalState";
import {
  fetchGroupModalData,
  fetchGroupsPageData,
  isApiMutationError,
  postEmpty,
  postForm,
  toFieldErrorMap,
  type FieldErrorMap,
} from "../lib/api";
import type { GroupModalData, GroupsPageData } from "../lib/models";
import { useEmailerShell } from "../lib/useEmailerShell";

type PageState =
  | { status: "loading" }
  | { status: "ready"; data: GroupsPageData }
  | { status: "error"; message: string };

type GroupModalState =
  | { status: "closed" }
  | { status: "loading"; groupId: number }
  | { status: "error"; groupId: number; message: string }
  | { status: "ready"; data: GroupModalData };

export function GroupsBootstrap() {
  const shellState = useEmailerShell("Не удалось загрузить оболочку Emailer.");
  const [pageState, setPageState] = useState<PageState>({ status: "loading" });
  const [groupName, setGroupName] = useState("");
  const [filter, setFilter] = useState("");
  const [modalState, setModalState] = useState<GroupModalState>({
    status: "closed",
  });
  const [selectedRecipientIds, setSelectedRecipientIds] = useState<number[]>(
    [],
  );
  const [isSubmittingGroup, setIsSubmittingGroup] = useState(false);
  const [isAssigning, setIsAssigning] = useState(false);
  const [isDeleting, setIsDeleting] = useState(false);
  const [createFieldErrors, setCreateFieldErrors] = useState<FieldErrorMap>({});
  const [assignFieldErrors, setAssignFieldErrors] = useState<FieldErrorMap>({});
  const modalRequestIdRef = useRef(0);
  const groupModalRef = useRef<HTMLDivElement | null>(null);

  const loadPage = async (showLoadingState = true) => {
    if (showLoadingState) {
      setPageState({ status: "loading" });
    }

    try {
      const data = await fetchGroupsPageData();
      setPageState({ status: "ready", data });
    } catch (error) {
      setPageState({
        status: "error",
        message:
          error instanceof Error
            ? error.message
            : "Не удалось загрузить страницу групп.",
      });
    }
  };

  useEffect(() => {
    void loadPage();
  }, []);

  const getModalInstance = () => {
    if (groupModalRef.current == null) {
      return null;
    }

    return window.bootstrap?.Modal.getOrCreateInstance(groupModalRef.current);
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

  const openModal = (groupId: number) => {
    modalRequestIdRef.current += 1;
    const requestId = modalRequestIdRef.current;

    setModalState({ status: "loading", groupId });
    getModalInstance()?.show();

    void fetchGroupModalData(groupId)
      .then((data) => {
        if (modalRequestIdRef.current !== requestId) {
          return;
        }

        setSelectedRecipientIds(
          data.recipients.map((recipient) => recipient.id),
        );
        setModalState({ status: "ready", data });
      })
      .catch((error) => {
        if (modalRequestIdRef.current !== requestId) {
          return;
        }

        setModalState({
          status: "error",
          groupId,
          message:
            error instanceof Error
              ? error.message
              : "Не удалось загрузить группу.",
        });
      });
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

  const filteredGroups = pageState.data.groups.filter((group) =>
    group.name.toLowerCase().includes(filter.trim().toLowerCase()),
  );

  const recipientOptions: DropdownMultiSelectOption[] =
    pageState.data.recipients.map((recipient) => ({
      value: String(recipient.id),
      label: recipient.text,
      details: Object.entries(recipient.fields).map(
        ([name, value]) => `${name}: ${value}`,
      ),
    }));

  const handleCreateGroup = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setIsSubmittingGroup(true);

    try {
      const body = new URLSearchParams();
      body.set("name", groupName);
      const response = await postForm("/group/add", body);
      setCreateFieldErrors({});
      window.showFlashMessage?.(response.message, "primary");
      setGroupName("");
      await loadPage(false);
    } catch (error) {
      if (isApiMutationError(error)) {
        setCreateFieldErrors(toFieldErrorMap(error));
        window.showFlashMessage?.(error.message, "danger");
      } else {
        showError(error, "Не удалось создать группу.");
      }
    } finally {
      setIsSubmittingGroup(false);
    }
  };

  const handleAssign = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (modalState.status !== "ready") {
      return;
    }

    setIsAssigning(true);

    try {
      const body = new URLSearchParams();
      selectedRecipientIds.forEach((recipientId) => {
        body.append("recipient_id", String(recipientId));
      });
      const response = await postForm(
        `/group/${modalState.data.group.id}/assign`,
        body,
      );
      setAssignFieldErrors({});
      window.showFlashMessage?.(response.message, "primary");
      getModalInstance()?.hide();
      setModalState({ status: "closed" });
      await loadPage(false);
    } catch (error) {
      if (isApiMutationError(error)) {
        setAssignFieldErrors(toFieldErrorMap(error));
        window.showFlashMessage?.(error.message, "danger");
      } else {
        showError(error, "Не удалось назначить получателей в группу.");
      }
    } finally {
      setIsAssigning(false);
    }
  };

  const handleDelete = async () => {
    if (modalState.status !== "ready") {
      return;
    }
    if (!window.confirm("Удалить?")) {
      return;
    }

    setIsDeleting(true);

    try {
      const response = await postEmpty(
        `/group/${modalState.data.group.id}/delete`,
      );
      window.showFlashMessage?.(response.message, "primary");
      getModalInstance()?.hide();
      setModalState({ status: "closed" });
      await loadPage(false);
    } catch (error) {
      showError(error, "Не удалось удалить группу.");
    } finally {
      setIsDeleting(false);
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
          <div className="col">
            <form onSubmit={handleCreateGroup}>
              <div className="row">
                <div className="col">
                  <input
                    className="form-control my-1"
                    type="text"
                    name="name"
                    placeholder="Название"
                    required
                    value={groupName}
                    onChange={(event) => {
                      setGroupName(event.currentTarget.value);
                      setCreateFieldErrors((current) => ({
                        ...current,
                        name: [],
                      }));
                    }}
                  />
                  {fieldError(createFieldErrors, "name") ? (
                    <div className="invalid-feedback d-block">
                      {fieldError(createFieldErrors, "name")}
                    </div>
                  ) : null}
                </div>
                <div className="col-auto text-end">
                  <button
                    className="btn btn-primary my-1"
                    type="submit"
                    disabled={isSubmittingGroup}
                  >
                    {isSubmittingGroup ? "Добавление..." : "Добавить"}
                  </button>
                </div>
              </div>
            </form>
          </div>
        </div>

        {pageState.data.groups.length > 0 ? (
          <>
            <div className="container mb-1 px-0">
              <div className="row">
                <div className="col">
                  <input
                    type="text"
                    className="form-control"
                    placeholder="Фильтр"
                    value={filter}
                    onChange={(event) => setFilter(event.currentTarget.value)}
                  />
                </div>
              </div>
            </div>

            <div className="container px-0">
              <div className="row">
                <div className="col">
                  <ul className="list-group" id="items">
                    {filteredGroups.map((group) => (
                      <li
                        key={group.id}
                        className="list-group-item selectable"
                        role="button"
                        onClick={() => openModal(group.id)}
                      >
                        <strong>{group.name}</strong>
                        <br />
                        {group.createdAt ? (
                          <small className="text-muted">
                            {group.createdAt}
                          </small>
                        ) : null}
                      </li>
                    ))}
                  </ul>
                </div>
              </div>
            </div>
          </>
        ) : null}

        <div
          className="modal fade"
          id="groupModal"
          ref={groupModalRef}
          tabIndex={-1}
          aria-labelledby="groupModalLabel"
          aria-hidden="true"
        >
          <div className="modal-dialog modal-lg">
            <div className="modal-content">
              <div className="modal-header">
                <h1 className="modal-title fs-5" id="groupModalLabel">
                  Редактировать группу
                </h1>
                <button
                  type="button"
                  className="btn-close"
                  data-bs-dismiss="modal"
                  aria-label="Close"
                  onClick={() => setModalState({ status: "closed" })}
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
                    <h5>
                      {modalState.data.group.name} (
                      {modalState.data.recipients.length} получателей)
                    </h5>
                    <form onSubmit={handleAssign} id="recipients-assign-form">
                      <div className="row mb-3">
                        <div className="col-lg">
                          <DropdownMultiSelect
                            options={recipientOptions}
                            selectedValues={selectedRecipientIds.map(String)}
                            onChange={(values) =>
                              setSelectedRecipientIds(values.map(Number))
                            }
                            className="my-1"
                            menuHeightClassName="emailer-dropdown-multiselect-options-lg"
                            searchPlaceholder="Поиск получателей"
                            clearable
                            clearLabel="Очистить выбранных получателей"
                          />
                          {fieldError(assignFieldErrors, "recipient_id") ? (
                            <div className="invalid-feedback d-block">
                              {fieldError(assignFieldErrors, "recipient_id")}
                            </div>
                          ) : null}
                        </div>
                        <div className="col-auto text-end">
                          <button
                            className="btn btn-primary my-1"
                            type="submit"
                            disabled={isAssigning}
                          >
                            {isAssigning ? "Назначение..." : "Назначить"}
                          </button>
                        </div>
                      </div>
                    </form>
                  </div>
                  <div className="modal-footer">
                    <button
                      className="btn btn-danger"
                      type="button"
                      disabled={isDeleting}
                      onClick={() => void handleDelete()}
                    >
                      {isDeleting ? "Удаление..." : "Удалить"}
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
