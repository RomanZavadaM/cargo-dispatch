import { test, expect } from "@playwright/test";
import fs from "node:fs";

test.setTimeout(90_000);

test("P02 request editor protects unsaved data and validates outside widgets", async ({ page }) => {
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto("http://127.0.0.1:4173", { waitUntil: "domcontentloaded" });
  await expect(page.locator("#pool-count")).toHaveText("5000", { timeout: 30_000 });
  await expect(page.locator(".pool-row")).toHaveCount(5000, { timeout: 30_000 });

  const firstRow = page.locator(".pool-row").first();
  const originalOrigin = await firstRow.locator(".origin-cell").textContent();
  const originalDestination = await firstRow.locator(".destination-cell").textContent();

  await firstRow.locator(".open-row").click();
  await page.locator("#edit-request").click();
  await expect(page.locator("#request-editor")).toBeVisible();

  const visibleFieldCount = await page.locator(".editor-field").count();
  expect(visibleFieldCount).toBeGreaterThanOrEqual(40);
  await expect(page.locator("#editor-source")).not.toHaveText("");
  await expect(page.locator("#editor-confidence")).toContainText("%");

  await page.locator("#editor-origin").fill("");
  await expect(page.locator("#editor-dirty")).toHaveText("Unsaved changes");
  await expect(firstRow.locator(".origin-cell")).toHaveText(originalOrigin ?? "");

  await page.locator("#editor-close").click();
  await expect(page.locator("#unsaved-warning")).toBeVisible();
  await page.locator("#keep-editing").click();
  await expect(page.locator("#unsaved-warning")).toBeHidden();

  await page.locator("#editor-save").click();
  await expect(page.locator("#validation-errors")).toContainText("Origin is required");
  await expect(firstRow.locator(".origin-cell")).toHaveText(originalOrigin ?? "");

  await page.locator("#editor-origin").fill("Vienna");
  await page.locator("#mode-phone").click();
  await expect(page.locator("#editor-contact_phone")).toBeVisible();
  await page.locator("#editor-contact_phone").fill("");
  await page.locator("#editor-save").click();
  await expect(page.locator("#validation-errors")).toContainText("contact number");

  await page.locator("#editor-contact_phone").fill("+491234567890");
  await page.locator("#editor-save").click();
  await expect(page.locator("#editor-saved")).toHaveText("Saved");
  await expect(page.locator("#editor-dirty")).toHaveText("Saved state");
  await expect(firstRow.locator(".origin-cell")).toHaveText("Vienna");
  await expect(firstRow.locator(".source-cell")).toHaveText("phone");

  await page.locator("#editor-destination").fill("Hamburg");
  await expect(page.locator("#editor-dirty")).toHaveText("Unsaved changes");
  await page.locator("#editor-cancel").click();
  await expect(page.locator("#request-editor")).toBeHidden();
  await expect(firstRow.locator(".origin-cell")).toHaveText("Vienna");
  await expect(firstRow.locator(".destination-cell")).toHaveText(originalDestination ?? "");

  expect(pageErrors).toEqual([]);

  fs.mkdirSync("probe/ui/out", { recursive: true });
  fs.writeFileSync(
    "probe/ui/out/p02-browser.json",
    JSON.stringify(
      {
        probe: "P02",
        pass: true,
        visible_fields: visibleFieldCount,
        unsaved_warning: true,
        validation_before_save: true,
        phone_mode_validation: true,
        explicit_save_updates_row: true,
        cancel_keeps_saved_snapshot: true,
        page_errors: pageErrors,
      },
      null,
      2,
    ) + "\n",
  );
});
