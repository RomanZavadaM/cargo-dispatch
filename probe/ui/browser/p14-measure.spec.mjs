import { test, expect } from "@playwright/test";
import fs from "node:fs";

test("measure useful render, table/map readiness and browser heap", async ({ page, context }) => {
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  const started = Date.now();
  await page.goto("http://127.0.0.1:4173", { waitUntil: "domcontentloaded" });
  await expect(page.locator("#planner-title")).toBeVisible();
  const usefulRenderMs = Date.now() - started;

  await expect(page.locator("#pool-count")).toHaveText("5000");
  const tableReadyMs = Date.now() - started;

  await expect(page.locator("#map-generation")).not.toHaveText("0", { timeout: 30_000 });
  const mapReadyMs = Date.now() - started;

  const session = await context.newCDPSession(page);
  await session.send("Performance.enable");
  const result = await session.send("Performance.getMetrics");
  const metrics = Object.fromEntries(result.metrics.map((item) => [item.name, item.value]));
  const jsHeapUsedBytes = Math.round(metrics.JSHeapUsedSize ?? 0);
  const jsHeapTotalBytes = Math.round(metrics.JSHeapTotalSize ?? 0);

  expect(jsHeapUsedBytes).toBeGreaterThan(0);
  expect(pageErrors).toEqual([]);

  fs.mkdirSync("probe/ui/out", { recursive: true });
  fs.writeFileSync(
    "probe/ui/out/p14-runtime.json",
    JSON.stringify(
      {
        probe: "P14",
        useful_render_ms: usefulRenderMs,
        table_ready_ms: tableReadyMs,
        map_ready_ms: mapReadyMs,
        browser_js_heap_used_bytes_after_table_map: jsHeapUsedBytes,
        browser_js_heap_total_bytes_after_table_map: jsHeapTotalBytes,
        page_errors: pageErrors,
      },
      null,
      2,
    ) + "\n",
  );
});
