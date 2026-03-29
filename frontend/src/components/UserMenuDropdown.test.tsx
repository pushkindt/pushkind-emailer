import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { UserMenuDropdown } from "./UserMenuDropdown";

describe("UserMenuDropdown", () => {
  it("renders local menu items, home link, and logout action", () => {
    const markup = renderToStaticMarkup(
      <UserMenuDropdown
        currentUserEmail="user@example.com"
        localItems={[
          { name: "Домой", url: "https://users.pushkind.com" },
          { name: "Настройки", url: "/settings" },
        ]}
        fetchedItems={[{ name: "Отписавшиеся", url: "/unsubscribed" }]}
        logoutAction="/logout"
      />,
    );

    expect(markup).toContain("user@example.com");
    expect(markup.indexOf("Домой")).toBeLessThan(markup.indexOf("Настройки"));
    expect(markup.indexOf("Настройки")).toBeLessThan(
      markup.indexOf("Отписавшиеся"),
    );
    expect(markup.indexOf("Отписавшиеся")).toBeLessThan(
      markup.lastIndexOf("Выйти"),
    );
    expect(markup).toContain("/settings");
    expect(markup).toContain("bi bi-gear mb-2");
    expect(markup).toContain("/unsubscribed");
    expect(markup).toContain("bi bi-person-x mb-2");
    expect(markup).toContain("https://users.pushkind.com");
    expect(markup).toContain("/logout");
  });
});
