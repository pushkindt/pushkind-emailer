import { ServiceNavbar } from "@pushkind/frontend-shell/ServiceNavbar";
import type { NavigationItem, UserMenuItem } from "../lib/models";

type EmailerNavbarProps = {
  navigation: NavigationItem[];
  currentUserEmail: string;
  homeUrl: string;
  localMenuItems: UserMenuItem[];
  fetchedMenuItems: UserMenuItem[];
};

export function EmailerNavbar({
  navigation,
  currentUserEmail,
  homeUrl,
  localMenuItems,
  fetchedMenuItems,
}: EmailerNavbarProps) {
  const pathname = window.location.pathname;

  const isActive = (url: string) => {
    if (url === "/") {
      return pathname === "/";
    }

    return pathname === url || pathname.startsWith(`${url}/`);
  };

  return (
    <ServiceNavbar
      brand={
        <>
          <img
            className="logo d-inline-block align-text-top"
            src="/assets/logo.png"
            alt="Logo"
          />
          Emailer
        </>
      }
      collapseId="navbarSupportedContent"
      navigation={navigation}
      currentUserEmail={currentUserEmail}
      homeUrl={homeUrl}
      localMenuItems={localMenuItems}
      fetchedMenuItems={fetchedMenuItems}
      logoutAction="/logout"
      isNavigationItemActive={(item) => isActive(item.url)}
      userMenuWrapperClassName=""
    />
  );
}
