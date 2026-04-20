import type {
  FrontendNoAccessData,
  FrontendShellCurrentUser,
  FrontendShellData,
  FrontendShellNavigationItem,
  FrontendShellUserMenuItem,
} from "@pushkind/frontend-shell/types";

export type NavigationItem = FrontendShellNavigationItem;
export type UserMenuItem = FrontendShellUserMenuItem;
export type CurrentUser = FrontendShellCurrentUser;
export type ShellData = FrontendShellData;
export type NoAccessData = FrontendNoAccessData<CurrentUser>;

export type RecipientOption = {
  id: string;
  text: string;
  fields: Record<string, string>;
};

export type RecipientAssignmentOption = {
  id: number;
  text: string;
  fields: Record<string, string>;
};

export type GroupOption = {
  id: number;
  name: string;
};

export type EmailPreview = {
  id: number;
  createdAt: string;
  subject?: string;
  messageHtml: string;
  messagePreview: string;
  isSent: boolean;
  numSent: number;
  numOpened: number;
  numReplied: number;
  recipientCount: number;
  recipients: Array<{
    address: string;
    opened: boolean;
    isSent: boolean;
    reply?: string;
  }>;
};

export type RetryEmail = {
  id: number;
  subject?: string;
  message: string;
  recipientCount: number;
  recipientIds: string[];
};

export type IndexPageData = {
  retryEmail?: RetryEmail;
  recipients: RecipientOption[];
  groups: GroupOption[];
  emails: {
    items: EmailPreview[];
    pages: Array<number | null>;
    page: number;
  };
  customFields: string[];
  crmServiceUrl: string;
  filesServiceUrl: string;
};

export type RecipientListItem = {
  id: number;
  name: string;
  email: string;
  fields: Record<string, string>;
};

export type RecipientsPageData = {
  recipients: {
    items: RecipientListItem[];
    pages: Array<number | null>;
    page: number;
  };
  searchQuery?: string;
  crmServiceUrl: string;
};

export type RecipientField = {
  name: string;
  value: string;
};

export type RecipientModalData = {
  recipient: {
    id: number;
    name: string;
    email: string;
    unsubscribedAt?: string;
    groupIds: number[];
    fields: RecipientField[];
  };
  groups: GroupOption[];
};

export type GroupListItem = {
  id: number;
  name: string;
  createdAt?: string;
};

export type GroupsPageData = {
  groups: GroupListItem[];
  customFields: string[];
  recipients: RecipientAssignmentOption[];
};

export type GroupModalData = {
  group: {
    id: number;
    name: string;
  };
  recipients: RecipientAssignmentOption[];
};

export type SettingsPageData = {
  login?: string;
  password?: string;
  sender?: string;
  smtpServer?: string;
  smtpPort?: number;
  imapServer?: string;
  imapPort?: number;
  message?: string;
  filesServiceUrl: string;
};

export type UnsubscribedPageData = {
  items: Array<{
    email: string;
    reason?: string;
    unsubscribedAt: string;
  }>;
};

export type HistoryPageData = {
  items: Array<{
    address: string;
    name: string;
    updatedAt: string;
    opened: boolean;
    replied: boolean;
  }>;
  crmServiceUrl: string;
};
