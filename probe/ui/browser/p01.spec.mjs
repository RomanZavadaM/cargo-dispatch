import { test, expect } from "@playwright/test";
import fs from "node:fs";

test.setTimeout(90_000);

test("P01 full 5k planner pool remains interactive", async ({ page }) => {
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  const timings = {};
  let started = Date.now();

  await page.goto("http://127.0.0.1:4173", { waitUntil: "domcontentloaded" });
  await expect(page.locator('[data-probe="leptos"]')).toBeVisible();
  await expect(page.locator("#pool-count")).toHaveText("5000", { timeout: 30_000 });
  await expect(page.locator(".pool-row")).toHaveCount(5000, { timeout: 30_000 });
  timings.initial_5000_rows_ms = Date.now() - started;

  const filter = page.locator("#pool-filter");
  started = Date.now();
  await filter.fill("Berlin");
  await expect(page.locator("#pool-count")).not.toHaveText("5000");
  const filteredCount = Number(await page.locator("#pool-count").textContent());
  expect(filteredCount).toBeGreaterThan(0);
  expect(filteredCount).toBeLessThan(5000);
  await expect(page.locator(".pool-row")).toHaveCount(filteredCount);
  timings.filter_berlin_ms = Date.now() - started;

  await filter.fill("");
  await expect(page.locator("#pool-count")).toHaveText("5000");
  await expect(page.locator(".pool-row")).toHaveCount(5000);

  started = Date.now();
  await page.locator("#sort-rate").click();
  const rateValues = await page
    .locator(".pool-row")
    .evaluateAll((rows) =>
      rows.slice(0, 50).map((row) => Number(row.getAttribute("data-rate"))),
    );
  expect(rateValues.length).toBe(50);
  for (let index = 1; index < rateValues.length; index += 1) {
    expect(rateValues[index - 1]).toBeGreaterThanOrEqual(rateValues[index]);
  }
  timings.sort_rate_ms = Date.now() - started;

  await page.locator(".row-select").first().check();
  await expect(page.locator("#selected-count")).toHaveText("1");

  const firstRowId = await page.locator(".pool-row").first().getAttribute("data-row-id");
  await page.locator(".open-row").first().click();
  await expect(page.locator("#row-detail")).toBeVisible();
  await expect(page.locator("#row-detail")).toContainText(`Request #${firstRowId}`);

  started = Date.now();
  const lastRow = page.locator(".pool-row").last();
  await lastRow.scrollIntoViewIfNeeded();
  await expect(lastRow).toBeVisible();
  timings.scroll_to_last_row_ms = Date.now() - started;

  expect(pageErrors).toEqual([]);

  fs.mkdirSync("probe/ui/out", { recursive: true });
  fs.writeFileSync(
    "probe/ui/out/p01-browser.json",
    JSON.stringify(
      {
        probe: "P01",
        pass: true,
        rendered_rows: 5000,
        filtered_rows: filteredCount,
        page_errors: pageErrors,
        timings_ms: timings,
      },
      null,
      2,
    ) + "\n",
  );
});
