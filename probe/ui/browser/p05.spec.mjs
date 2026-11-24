import { test, expect } from "@playwright/test";
import fs from "node:fs";

test("standalone browser stays independent from desktop commands", async ({ page }) => {
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto("http://127.0.0.1:4173", { waitUntil: "domcontentloaded" });
  await expect(page.locator("#pool-count")).toHaveText("5000", { timeout: 30_000 });

  const available = await page.evaluate(() => window.cargoDesktopBridge.available());
  expect(available).toBe(false);

  const errorMessage = await page.evaluate(async () => {
    try {
      await window.cargoDesktopBridge.loadRequestPage({
        filter: "",
        offset: 0,
        limit: 25,
      });
      return "unexpected-success";
    } catch (error) {
      return String(error && error.message ? error.message : error);
    }
  });
  expect(errorMessage).toContain("not available");
  expect(pageErrors).toEqual([]);

  fs.mkdirSync("probe/ui/out", { recursive: true });
  fs.writeFileSync(
    "probe/ui/out/p05-browser.json",
    JSON.stringify(
      {
        probe: "desktop-command-boundary",
        pass: true,
        standalone_browser_has_no_desktop_runtime: true,
        unavailable_command_is_explicit_error: true,
        page_errors: pageErrors,
      },
      null,
      2,
    ) + "\n",
  );
});
