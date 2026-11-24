import { test, expect } from "@playwright/test";
import fs from "node:fs";

test("synthetic failures stay human-readable and do not break the UI", async ({ page }) => {
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  await page.goto("http://127.0.0.1:4173", { waitUntil: "domcontentloaded" });
  await page.locator("#ui-language").selectOption("en");

  const cases = [
    ["#fail-storage", "storage_write_failed", "Could not save data", true],
    ["#fail-route", "route_calculation_failed", "Could not recalculate route", true],
    ["#fail-offline", "connectivity_offline", "Connection is offline", true],
    ["#fail-package", "invalid_trip_package", "Trip package is damaged or incompatible", false],
    ["#fail-transfer", "file_transfer_failed", "Could not transfer file", true],
  ];

  for (const [button, code, title, retryable] of cases) {
    await page.locator(button).click();
    const panel = page.locator("#error-panel");
    await expect(panel).toBeVisible();
    await expect(panel).toHaveAttribute("data-code", code);
    await expect(page.locator("#error-title")).toHaveText(title);
    await expect(page.locator("#error-message")).not.toHaveText("");
    await expect(page.locator("#error-action")).not.toHaveText("");
    await page.locator("#error-panel details").evaluate((element) => {
      element.open = true;
    });
    await expect(page.locator("#error-details")).toContainText("synthetic failure");
    await expect(page.locator("#error-details")).not.toContainText("panicked at");

    if (retryable) {
      await expect(page.locator("#error-retry")).toBeVisible();
    } else {
      await expect(page.locator("#error-retry")).toHaveCount(0);
    }

    await page.locator("#pool-filter").fill("Berlin");
    await expect(page.locator("#pool-count")).not.toHaveText("0");
    await page.locator("#pool-filter").fill("");

    await page.locator("#error-dismiss").click();
    await expect(panel).toHaveCount(0);
  }

  expect(pageErrors).toEqual([]);
  fs.mkdirSync("probe/ui/out", { recursive: true });
  fs.writeFileSync(
    "probe/ui/out/p12-errors.json",
    JSON.stringify(
      {
        probe: "P12",
        pass: true,
        storage_error: true,
        route_error: true,
        offline_error: true,
        invalid_trip_package: true,
        file_transfer_error: true,
        human_message: true,
        technical_details: true,
        ui_remains_usable: true,
        raw_panic_hidden: true,
        page_errors: pageErrors,
      },
      null,
      2,
    ) + "\n",
  );
});
