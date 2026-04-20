import { ServiceNoAccessPage } from "@pushkind/frontend-shell/noAccess";

import { EmailerShell } from "../components/EmailerShell";
import { EmailerShellFatalState } from "../components/EmailerShellFatalState";
import {
  fetchHubMenuItems,
  fetchNoAccessData,
  fetchShellData,
} from "../lib/api";
import type { NoAccessData, ShellData, UserMenuItem } from "../lib/models";

export function NoAccessPage() {
  return (
    <ServiceNoAccessPage<NoAccessData, ShellData, UserMenuItem>
      serviceLabel="Emailer"
      fetchShellData={fetchShellData}
      fetchHubMenuItems={fetchHubMenuItems}
      fetchNoAccessData={fetchNoAccessData}
      ShellComponent={EmailerShell}
      FatalStateComponent={EmailerShellFatalState}
      menuLoadWarning="Failed to load auth navigation menu. Falling back to local Emailer menu only."
    />
  );
}
