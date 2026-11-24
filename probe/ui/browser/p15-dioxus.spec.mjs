import { test, expect } from "@playwright/test";
import fs from "node:fs";

test("Dioxus harness uses the same application service for 5k pool and edit", async ({ page }) => {
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  const started = Date.now();
  await page.goto("http://127.0.0.1:4175", { waitUntil: "domcontentloaded" });
  await expect(page.locator("#dioxus-title")).toBeVisible();
  const usefulRenderMs = Date.now() - started;

  await expect(page.locator("#dioxus-total")).toHaveText("5000");
  await expect(page.locator(".dioxus-row")).toHaveCount(5000);
  const tableReadyMs = Date.now() - started;

  await page.locator("#dioxus-filter").fill("Berlin");
  const visible = Number((await page.locator("#dioxus-visible").textContent())?.split(" ")[0] ?? "0");
  expect(visible).toBeGreaterThan(0);
  expect(visible).toBeLessThan(5000);
  await page.locator("#dioxus-filter").fill("");

  const firstRow = page.locator(".dioxus-row").first();
  await firstRow.locator(".dioxus-open").click();
  await expect(page.locator("#dioxus-editor")).toBeVisible();

  await page.locator("#dioxus-origin-input").fill("Vienna");
  const beforeRevision = Number(await page.locator("#dioxus-revision").textContent());
  await page.locator("#dioxus-save").click();
  await expect(page.locator("#dioxus-message")).toHaveText("Saved");
  await expect(firstRow.locator(".dioxus-origin")).toHaveText("Vienna");
  const afterRevision = Number(await page.locator("#dioxus-revision").textContent());
  expect(afterRevision).toBe(beforeRevision + 1);

  expect(pageErrors).toEqual([]);

  fs.mkdirSync("probe/ui/out", { recursive: true });
  fs.writeFileSync(
    "probe/ui/out/p15-dioxus.json",
    JSON.stringify(
      {
        probe: "P15-secondary",
        pass: true,
        shared_application_service: true,
        rows: 5000,
        filter: true,
        editor_save: true,
        useful_render_ms: usefulRenderMs,
        table_ready_ms: tableReadyMs,
        page_errors: pageErrors,
      },
      null,
      2,
    ) + "\n",
  );
});
