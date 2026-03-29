import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { DropdownMultiSelect } from "./DropdownMultiSelect";

describe("DropdownMultiSelect", () => {
  it("renders the provided control id on the toggle button", () => {
    const markup = renderToStaticMarkup(
      <DropdownMultiSelect
        id="recipient-groups"
        options={[{ value: "1", label: "Retail" }]}
        selectedValues={[]}
        onChange={() => {}}
      />,
    );

    expect(markup).toContain('id="recipient-groups"');
    expect(markup).toContain('aria-controls="recipient-groups-menu"');
  });

  it("does not nest the clear button inside the toggle button", () => {
    const markup = renderToStaticMarkup(
      <DropdownMultiSelect
        options={[{ value: "1", label: "Retail" }]}
        selectedValues={["1"]}
        onChange={() => {}}
        clearable
      />,
    );

    expect(markup).toContain('class="emailer-dropdown-multiselect-toggle"');
    expect(markup).toContain('class="emailer-dropdown-multiselect-clear"');
    expect(markup).toContain("</button><span");
  });
});
