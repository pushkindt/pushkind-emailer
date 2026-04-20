import { ModalFlashShell } from "@pushkind/frontend-shell/ModalFlashShell";
import type { ReactNode } from "react";

import { EmailerNavbar } from "./EmailerNavbar";
import type { NavigationItem, UserMenuItem } from "../lib/models";

type EmailerShellProps = {
  navigation: NavigationItem[];
  currentUserEmail: string;
  homeUrl: string;
  localMenuItems: UserMenuItem[];
  fetchedMenuItems: UserMenuItem[];
  children: ReactNode;
};

export function EmailerShell({
  navigation,
  currentUserEmail,
  homeUrl,
  localMenuItems,
  fetchedMenuItems,
  children,
}: EmailerShellProps) {
  return (
    <ModalFlashShell
      navbar={
        <EmailerNavbar
          navigation={navigation}
          currentUserEmail={currentUserEmail}
          homeUrl={homeUrl}
          localMenuItems={localMenuItems}
          fetchedMenuItems={fetchedMenuItems}
        />
      }
      enablePopovers
      enableTooltips
    >
      {children}
    </ModalFlashShell>
  );
}
