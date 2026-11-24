import { test, expect } from "@playwright/test";
import fs from "node:fs";

test("editor X cancel discard save and reopen protect saved state", async ({ page }) => {
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  await page.goto("http://127.0.0.1:4173", { waitUntil: "domcontentloaded" });
  await page.locator("#ui-language").selectOption("en");

  const firstRow = page.locator(".pool-row").first();
  const originalOrigin = await firstRow.locator(".origin-cell").textContent();

  await firstRow.locator(".open-row").click();
  const edit = page.locator("#edit-request");
  await edit.click();
  await page.locator("#editor-origin").fill("Vienna");

  await page.locator("#editor-x").click();
  await expect(page.locator("#unsaved-warning")).toBeVisible();
  await expect(firstRow.locator(".origin-cell")).toHaveText(originalOrigin ?? "");

  await page.locator("#keep-editing").click();
  await expect(page.locator("#editor-origin")).toHaveValue("Vienna");
  await page.locator("#editor-cancel").click();
  await expect(page.locator("#request-editor")).toHaveCount(0);
  await expect(firstRow.locator(".origin-cell")).toHaveText(originalOrigin ?? "");

  await edit.click();
  await page.locator("#editor-origin").fill("Vienna");
  await page.locator("#editor-save").click();
  await expect(firstRow.locator(".origin-cell")).toHaveText("Vienna");

  await page.locator("#editor-x").click();
  await expect(page.locator("#request-editor")).toHaveCount(0);
  await edit.click();
  await expect(page.locator("#editor-origin")).toHaveValue("Vienna");

  await page.locator("#editor-origin").fill("Paris");
  await page.locator("#editor-x").click();
  await expect(page.locator("#unsaved-warning")).toBeVisible();
  await page.locator("#discard-close").click();
  await expect(page.locator("#request-editor")).toHaveCount(0);
  await expect(firstRow.locator(".origin-cell")).toHaveText("Vienna");

  await edit.click();
  await expect(page.locator("#editor-origin")).toHaveValue("Vienna");

  expect(pageErrors).toEqual([]);
  fs.mkdirSync("probe/ui/out", { recursive: true });
  fs.writeFileSync(
    "probe/ui/out/p11-lifecycle.json",
    JSON.stringify(
      {
        probe: "P11",
        pass: true,
        x_warns_on_dirty: true,
        cancel_restores_saved_snapshot: true,
        discard_restores_saved_snapshot: true,
        save_close_reopen: true,
        unsaved_never_silently_commits: true,
        page_errors: pageErrors,
      },
      null,
      2,
    ) + "\n",
  );
});
