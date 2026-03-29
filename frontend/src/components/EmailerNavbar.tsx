import { UserMenuDropdown } from "./UserMenuDropdown";
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
    <div className="container">
      <nav className="navbar navbar-expand-sm bg-body-tertiary">
        <div className="container-fluid">
          <a className="navbar-brand" href="/">
            <img
              className="logo d-inline-block align-text-top"
              src="/assets/logo.png"
              alt="Logo"
            />
            Emailer
          </a>
          <button
            className="navbar-toggler"
            type="button"
            data-bs-toggle="collapse"
            data-bs-target="#navbarSupportedContent"
            aria-controls="navbarSupportedContent"
            aria-expanded="false"
            aria-label="Toggle navigation"
          >
            <span className="navbar-toggler-icon" />
          </button>
          <div className="collapse navbar-collapse" id="navbarSupportedContent">
            <ul className="navbar-nav me-auto">
              {navigation.map((item) => (
                <li className="nav-item" key={item.url}>
                  <a
                    className={`nav-link ${isActive(item.url) ? "active" : ""}`}
                    href={item.url}
                  >
                    {item.name}
                  </a>
                </li>
              ))}
            </ul>
          </div>
          <UserMenuDropdown
            currentUserEmail={currentUserEmail}
            localItems={[{ name: "Домой", url: homeUrl }, ...localMenuItems]}
            fetchedItems={fetchedMenuItems}
            logoutAction="/logout"
          />
        </div>
      </nav>
    </div>
  );
}
