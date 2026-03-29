import type {
  EmailPreview,
  GroupOption,
  GroupModalData,
  GroupListItem,
  GroupsPageData,
  HistoryPageData,
  IndexPageData,
  NavigationItem,
  RecipientAssignmentOption,
  RecipientField,
  RecipientListItem,
  RecipientModalData,
  RecipientOption,
  RecipientsPageData,
  SettingsPageData,
  RetryEmail,
  ShellData,
  UnsubscribedPageData,
  UserMenuItem,
} from "./models";

export interface ApiFieldError {
  field: string;
  message: string;
}

export interface ApiMutationSuccess {
  message: string;
  redirect_to: string | null;
}

export interface ApiMutationError {
  message: string;
  field_errors: ApiFieldError[];
}

export type FieldErrorMap = Record<string, string[]>;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function readString(record: Record<string, unknown>, key: string) {
  const value = record[key];
  if (typeof value !== "string") {
    throw new Error(`Invalid API response: expected string at ${key}.`);
  }

  return value;
}

function readOptionalString(record: Record<string, unknown>, key: string) {
  const value = record[key];
  if (value == null) {
    return undefined;
  }
  if (typeof value !== "string") {
    throw new Error(`Invalid API response: expected string at ${key}.`);
  }

  return value;
}

function readNumber(record: Record<string, unknown>, key: string) {
  const value = record[key];
  if (typeof value !== "number") {
    throw new Error(`Invalid API response: expected number at ${key}.`);
  }

  return value;
}

function readStringArray(record: Record<string, unknown>, key: string) {
  const value = record[key];
  if (!Array.isArray(value) || value.some((item) => typeof item !== "string")) {
    throw new Error(`Invalid API response: expected string[] at ${key}.`);
  }

  return value;
}

function parseNavigationItems(payload: unknown): NavigationItem[] {
  if (!Array.isArray(payload)) {
    throw new Error("Invalid navigation payload.");
  }

  return payload.map((item) => {
    if (!isRecord(item)) {
      throw new Error("Invalid navigation item payload.");
    }

    return {
      name: readString(item, "name"),
      url: readString(item, "url"),
    };
  });
}

function parseShellData(payload: unknown): ShellData {
  if (!isRecord(payload) || !isRecord(payload.current_user)) {
    throw new Error("Invalid shell payload.");
  }

  return {
    currentUser: {
      email: readString(payload.current_user, "email"),
      name: readString(payload.current_user, "name"),
      hubId: readNumber(payload.current_user, "hub_id"),
      roles: readStringArray(payload.current_user, "roles"),
    },
    homeUrl: readString(payload, "home_url"),
    navigation: parseNavigationItems(payload.navigation),
    localMenuItems: parseNavigationItems(payload.local_menu_items),
  };
}

function parseMenuItems(payload: unknown): UserMenuItem[] {
  if (!Array.isArray(payload)) {
    throw new Error("Invalid auth menu payload.");
  }

  return payload.map((item) => {
    if (!isRecord(item)) {
      throw new Error("Invalid auth menu item payload.");
    }

    return {
      name: readString(item, "name"),
      url: readString(item, "url"),
    };
  });
}

function parseStringMap(value: unknown) {
  if (!isRecord(value)) {
    return {};
  }

  return Object.fromEntries(
    Object.entries(value).filter((entry): entry is [string, string] => {
      return typeof entry[1] === "string";
    }),
  );
}

function parseRecipients(payload: unknown): RecipientOption[] {
  if (!Array.isArray(payload)) {
    throw new Error("Invalid recipients payload.");
  }

  return payload.map((item) => {
    if (!isRecord(item)) {
      throw new Error("Invalid recipient option payload.");
    }

    return {
      id: readString(item, "id"),
      text: readString(item, "text"),
      fields: parseStringMap(item.fields),
    };
  });
}

function parseRecipientAssignments(
  payload: unknown,
): RecipientAssignmentOption[] {
  if (!Array.isArray(payload)) {
    throw new Error("Invalid recipient assignment payload.");
  }

  return payload.map((item) => {
    if (!isRecord(item)) {
      throw new Error("Invalid recipient assignment option payload.");
    }

    return {
      id: readNumber(item, "id"),
      text: readString(item, "text"),
      fields: parseStringMap(item.fields),
    };
  });
}

function parseGroups(payload: unknown): GroupOption[] {
  if (!Array.isArray(payload)) {
    throw new Error("Invalid groups payload.");
  }

  return payload.map((item) => {
    if (!isRecord(item)) {
      throw new Error("Invalid group option payload.");
    }

    return {
      id: readNumber(item, "id"),
      name: readString(item, "name"),
    };
  });
}

function parseRecipientListItems(payload: unknown): RecipientListItem[] {
  if (!Array.isArray(payload)) {
    throw new Error("Invalid recipients list payload.");
  }

  return payload.map((item) => {
    if (!isRecord(item)) {
      throw new Error("Invalid recipient list item payload.");
    }

    return {
      id: readNumber(item, "id"),
      name: readString(item, "name"),
      email: readString(item, "email"),
      fields: parseStringMap(item.fields),
    };
  });
}

function parseRecipientFields(payload: unknown): RecipientField[] {
  if (!Array.isArray(payload)) {
    throw new Error("Invalid recipient fields payload.");
  }

  return payload.map((item) => {
    if (!isRecord(item)) {
      throw new Error("Invalid recipient field payload.");
    }

    return {
      name: readString(item, "name"),
      value: readString(item, "value"),
    };
  });
}

function parseGroupListItems(payload: unknown): GroupListItem[] {
  if (!Array.isArray(payload)) {
    throw new Error("Invalid groups list payload.");
  }

  return payload.map((item) => {
    if (!isRecord(item)) {
      throw new Error("Invalid group list item payload.");
    }

    return {
      id: readNumber(item, "id"),
      name: readString(item, "name"),
      createdAt: readOptionalString(item, "created_at"),
    };
  });
}

function parseEmailPreviews(payload: unknown): EmailPreview[] {
  if (!Array.isArray(payload)) {
    throw new Error("Invalid emails payload.");
  }

  return payload.map((item) => {
    if (!isRecord(item)) {
      throw new Error("Invalid email preview payload.");
    }

    return {
      id: readNumber(item, "id"),
      createdAt: readString(item, "created_at"),
      subject: readOptionalString(item, "subject"),
      messageHtml: readString(item, "message_html"),
      messagePreview: readString(item, "message_preview"),
      isSent: Boolean(item.is_sent),
      numSent: readNumber(item, "num_sent"),
      numOpened: readNumber(item, "num_opened"),
      numReplied: readNumber(item, "num_replied"),
      recipientCount: readNumber(item, "recipient_count"),
      recipients: Array.isArray(item.recipients)
        ? item.recipients.filter(isRecord).map((recipient) => ({
            address: readString(recipient, "address"),
            opened: Boolean(recipient.opened),
            isSent: Boolean(recipient.is_sent),
            reply: readOptionalString(recipient, "reply"),
          }))
        : [],
    };
  });
}

function parseRetryEmail(payload: unknown): RetryEmail | undefined {
  if (payload == null) {
    return undefined;
  }
  if (!isRecord(payload)) {
    throw new Error("Invalid retry email payload.");
  }

  return {
    id: readNumber(payload, "id"),
    subject: readOptionalString(payload, "subject"),
    message: readString(payload, "message"),
    recipientCount: readNumber(payload, "recipient_count"),
    recipientIds: Array.isArray(payload.recipient_ids)
      ? payload.recipient_ids.filter(
          (item): item is string => typeof item === "string",
        )
      : [],
  };
}

function parseNullableNumberArray(
  record: Record<string, unknown>,
  key: string,
) {
  const value = record[key];
  if (
    !Array.isArray(value) ||
    value.some((item) => item !== null && typeof item !== "number")
  ) {
    throw new Error(
      `Invalid API response: expected (number|null)[] at ${key}.`,
    );
  }

  return value;
}

function parseIndexPageData(payload: unknown): IndexPageData {
  if (!isRecord(payload) || !isRecord(payload.emails)) {
    throw new Error("Invalid index page payload.");
  }

  return {
    retryEmail: parseRetryEmail(payload.retry_email),
    recipients: parseRecipients(payload.recipients),
    groups: parseGroups(payload.groups),
    emails: {
      items: parseEmailPreviews(payload.emails.items),
      page: readNumber(payload.emails, "page"),
      pages: parseNullableNumberArray(payload.emails, "pages"),
    },
    customFields: Array.isArray(payload.custom_fields)
      ? payload.custom_fields.filter(
          (item): item is string => typeof item === "string",
        )
      : [],
    crmServiceUrl: readString(payload, "crm_service_url"),
  };
}

function parseRecipientsPageData(payload: unknown): RecipientsPageData {
  if (!isRecord(payload) || !isRecord(payload.recipients)) {
    throw new Error("Invalid recipients page payload.");
  }

  return {
    recipients: {
      items: parseRecipientListItems(payload.recipients.items),
      page: readNumber(payload.recipients, "page"),
      pages: parseNullableNumberArray(payload.recipients, "pages"),
    },
    searchQuery: readOptionalString(payload, "search_query"),
    crmServiceUrl: readString(payload, "crm_service_url"),
  };
}

function parseRecipientModalData(payload: unknown): RecipientModalData {
  if (!isRecord(payload) || !isRecord(payload.recipient)) {
    throw new Error("Invalid recipient modal payload.");
  }

  return {
    recipient: {
      id: readNumber(payload.recipient, "id"),
      name: readString(payload.recipient, "name"),
      email: readString(payload.recipient, "email"),
      unsubscribedAt: readOptionalString(payload.recipient, "unsubscribed_at"),
      groupIds: Array.isArray(payload.recipient.group_ids)
        ? payload.recipient.group_ids.filter(
            (item): item is number => typeof item === "number",
          )
        : [],
      fields: parseRecipientFields(payload.recipient.fields),
    },
    groups: parseGroups(payload.groups),
  };
}

function parseGroupsPageData(payload: unknown): GroupsPageData {
  if (!isRecord(payload)) {
    throw new Error("Invalid groups page payload.");
  }

  return {
    groups: parseGroupListItems(payload.groups),
    customFields: Array.isArray(payload.custom_fields)
      ? payload.custom_fields.filter(
          (item): item is string => typeof item === "string",
        )
      : [],
    recipients: parseRecipientAssignments(payload.recipients),
  };
}

function parseGroupModalData(payload: unknown): GroupModalData {
  if (!isRecord(payload) || !isRecord(payload.group)) {
    throw new Error("Invalid group modal payload.");
  }

  return {
    group: {
      id: readNumber(payload.group, "id"),
      name: readString(payload.group, "name"),
    },
    recipients: parseRecipientAssignments(payload.recipients),
  };
}

function parseSettingsPageData(payload: unknown): SettingsPageData {
  if (!isRecord(payload)) {
    throw new Error("Invalid settings page payload.");
  }

  return {
    login: readOptionalString(payload, "login"),
    password: readOptionalString(payload, "password"),
    sender: readOptionalString(payload, "sender"),
    smtpServer: readOptionalString(payload, "smtp_server"),
    smtpPort:
      payload.smtp_port == null ? undefined : readNumber(payload, "smtp_port"),
    imapServer: readOptionalString(payload, "imap_server"),
    imapPort:
      payload.imap_port == null ? undefined : readNumber(payload, "imap_port"),
    message: readOptionalString(payload, "message"),
  };
}

function parseUnsubscribedPageData(payload: unknown): UnsubscribedPageData {
  if (!isRecord(payload) || !Array.isArray(payload.items)) {
    throw new Error("Invalid unsubscribed page payload.");
  }

  return {
    items: payload.items.map((item) => {
      if (!isRecord(item)) {
        throw new Error("Invalid unsubscribed item payload.");
      }

      return {
        email: readString(item, "email"),
        reason: readOptionalString(item, "reason"),
        unsubscribedAt: readString(item, "unsubscribed_at"),
      };
    }),
  };
}

function parseHistoryPageData(payload: unknown): HistoryPageData {
  if (!isRecord(payload) || !Array.isArray(payload.items)) {
    throw new Error("Invalid history page payload.");
  }

  return {
    items: payload.items.map((item) => {
      if (!isRecord(item)) {
        throw new Error("Invalid history item payload.");
      }

      return {
        address: readString(item, "address"),
        name: readString(item, "name"),
        updatedAt: readString(item, "updated_at"),
        opened: Boolean(item.opened),
        replied: Boolean(item.replied),
      };
    }),
    crmServiceUrl: readString(payload, "crm_service_url"),
  };
}

function withBaseUrl(baseUrl: string, path: string) {
  return new URL(path, baseUrl).toString();
}

async function fetchJson(url: string) {
  const response = await fetch(url, {
    headers: {
      Accept: "application/json",
    },
    cache: "no-store",
    credentials: "include",
  });

  if (!response.ok) {
    if (response.status === 401) {
      throw new Error("Недостаточно прав для доступа к Emailer.");
    }

    throw new Error(`Request failed with status ${response.status}.`);
  }

  ensureResponseIsNotAuthRedirect(response);
  return response.json();
}

function isJsonResponse(response: Response): boolean {
  return (
    response.headers.get("content-type")?.includes("application/json") ?? false
  );
}

export const browserLocation = {
  assign(url: string) {
    window.location.assign(url);
  },
};

function handleAuthRedirectResponse(response: Response): never {
  browserLocation.assign(response.url);
  throw new Error("Сессия истекла. Выполняется переход на страницу входа.");
}

function ensureResponseIsNotAuthRedirect(response: Response) {
  if (response.redirected && !isJsonResponse(response)) {
    handleAuthRedirectResponse(response);
  }
}

async function readJsonResponse<T>(response: Response, endpoint: string) {
  if (!isJsonResponse(response)) {
    throw new Error(
      `Expected JSON response from ${endpoint} with status ${response.status}.`,
    );
  }

  return (await response.json()) as T;
}

export function isApiMutationError(error: unknown): error is ApiMutationError {
  return (
    isRecord(error) &&
    typeof error.message === "string" &&
    Array.isArray(error.field_errors)
  );
}

export function toFieldErrorMap(error: ApiMutationError): FieldErrorMap {
  return error.field_errors.reduce<FieldErrorMap>((result, fieldError) => {
    if (
      typeof fieldError.field !== "string" ||
      typeof fieldError.message !== "string"
    ) {
      return result;
    }

    if (result[fieldError.field] == null) {
      result[fieldError.field] = [];
    }

    result[fieldError.field].push(fieldError.message);
    return result;
  }, {});
}

export async function fetchShellData(): Promise<ShellData> {
  const payload = await fetchJson("/api/v1/iam");
  return parseShellData(payload);
}

export async function fetchIndexPageData(
  searchParams: URLSearchParams,
): Promise<IndexPageData> {
  const query = searchParams.toString();
  const payload = await fetchJson(
    query ? `/api/v1/emails?${query}` : "/api/v1/emails",
  );
  return parseIndexPageData(payload);
}

export async function fetchRecipientsPageData(
  searchParams: URLSearchParams,
): Promise<RecipientsPageData> {
  const query = searchParams.toString();
  const payload = await fetchJson(
    query ? `/api/v1/recipients?${query}` : "/api/v1/recipients",
  );
  return parseRecipientsPageData(payload);
}

export async function fetchRecipientModalData(
  recipientId: number,
): Promise<RecipientModalData> {
  const payload = await fetchJson(`/api/v1/recipients/${recipientId}`);
  return parseRecipientModalData(payload);
}

export async function fetchGroupsPageData(): Promise<GroupsPageData> {
  const payload = await fetchJson("/api/v1/groups");
  return parseGroupsPageData(payload);
}

export async function fetchGroupModalData(
  groupId: number,
): Promise<GroupModalData> {
  const payload = await fetchJson(`/api/v1/groups/${groupId}`);
  return parseGroupModalData(payload);
}

export async function fetchSettingsPageData(): Promise<SettingsPageData> {
  const payload = await fetchJson("/api/v1/hub-settings");
  return parseSettingsPageData(payload);
}

export async function fetchUnsubscribedPageData(): Promise<UnsubscribedPageData> {
  const payload = await fetchJson("/api/v1/unsubscribed-recipients");
  return parseUnsubscribedPageData(payload);
}

export async function fetchHistoryPageData(): Promise<HistoryPageData> {
  const payload = await fetchJson("/api/v1/email-history");
  return parseHistoryPageData(payload);
}

export async function fetchHubMenuItems(
  authBaseUrl: string,
  hubId: number,
): Promise<UserMenuItem[]> {
  const payload = await fetchJson(
    withBaseUrl(authBaseUrl, `/api/v1/hubs/${hubId}/menu-items`),
  );
  return parseMenuItems(payload);
}

export async function postMultipartForm(
  endpoint: string,
  body: FormData,
): Promise<ApiMutationSuccess> {
  const response = await fetch(endpoint, {
    method: "POST",
    body,
    credentials: "include",
    headers: {
      Accept: "application/json",
    },
  });

  ensureResponseIsNotAuthRedirect(response);

  if (response.ok) {
    return await readJsonResponse<ApiMutationSuccess>(response, endpoint);
  }

  throw await readJsonResponse<ApiMutationError>(response, endpoint);
}

export async function postForm(
  endpoint: string,
  body: URLSearchParams,
): Promise<ApiMutationSuccess> {
  const response = await fetch(endpoint, {
    method: "POST",
    body,
    credentials: "include",
    headers: {
      Accept: "application/json",
      "Content-Type": "application/x-www-form-urlencoded;charset=UTF-8",
    },
  });

  ensureResponseIsNotAuthRedirect(response);

  if (response.ok) {
    return await readJsonResponse<ApiMutationSuccess>(response, endpoint);
  }

  throw await readJsonResponse<ApiMutationError>(response, endpoint);
}

export async function postEmpty(endpoint: string): Promise<ApiMutationSuccess> {
  const response = await fetch(endpoint, {
    method: "POST",
    credentials: "include",
    headers: {
      Accept: "application/json",
    },
  });

  ensureResponseIsNotAuthRedirect(response);

  if (response.ok) {
    return await readJsonResponse<ApiMutationSuccess>(response, endpoint);
  }

  throw await readJsonResponse<ApiMutationError>(response, endpoint);
}
