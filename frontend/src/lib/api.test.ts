import { afterEach, describe, expect, it, vi } from "vitest";

import { browserLocation, fetchShellData } from "./api";

describe("fetchShellData", () => {
  afterEach(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it("redirects to login when an expired session returns redirected HTML", async () => {
    const jsonSpy = vi.fn();
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      redirected: true,
      url: "https://auth.pushkind.com/login",
      headers: new Headers({
        "content-type": "text/html; charset=utf-8",
      }),
      json: jsonSpy,
    } as unknown as Response);

    vi.stubGlobal("fetch", fetchMock);
    const assignSpy = vi
      .spyOn(browserLocation, "assign")
      .mockImplementation(() => {});

    await expect(fetchShellData()).rejects.toThrow(
      "Сессия истекла. Выполняется переход на страницу входа.",
    );

    expect(fetchMock).toHaveBeenCalledWith("/api/v1/iam", {
      headers: {
        Accept: "application/json",
      },
      cache: "no-store",
      credentials: "include",
    });
    expect(assignSpy).toHaveBeenCalledWith("https://auth.pushkind.com/login");
    expect(jsonSpy).not.toHaveBeenCalled();
  });
});
