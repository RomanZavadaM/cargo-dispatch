import { test, expect } from "@playwright/test";
import fs from "node:fs";

test("driver browser page stays standalone and exposes phone actions", async ({ browser }) => {
  const context = await browser.newContext({
    geolocation: { latitude: 49.8397, longitude: 24.0297 },
    permissions: ["geolocation"],
  });
  const page = await context.newPage();
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto("http://127.0.0.1:4174", { waitUntil: "domcontentloaded" });

  await expect(page.locator("[data-probe='driver-browser']")).toBeVisible();
  await expect(page.locator("#driver-trip")).toHaveText("TRIP-A");
  await expect(page.locator("#current-point")).not.toHaveText("");
  await expect(page.locator("#current-task")).not.toHaveText("");
  await expect(page.locator("#driver-documents li")).toHaveCount(2);

  await expect(page.locator("#driver-status")).toHaveText("У дорозі");
  await page.locator("#status-button").click();
  await expect(page.locator("#driver-status")).toHaveText("Прибув");

  const file = page.locator("#driver-file");
  await expect(file).toHaveAttribute("accept", /image\/\*/);
  await expect(file).toHaveAttribute("accept", /application\/pdf/);
  await expect(file).toHaveAttribute("capture", "environment");

  await page.locator("#geo-button").click();
  await expect(page.locator("#geo-status")).toContainText("49.83970", { timeout: 10_000 });

  const navigator = page.locator("#navigator-link");
  await expect(navigator).toHaveAttribute("target", "_blank");
  await expect(navigator).toHaveAttribute("href", /maps\/dir/);
  await expect(navigator).toHaveAttribute("href", /49\.8397/);

  expect(pageErrors).toEqual([]);

  fs.mkdirSync("probe/ui/out", { recursive: true });
  fs.writeFileSync(
    "probe/ui/out/p07-driver-browser.json",
    JSON.stringify(
      {
        probe: "P07",
        pass: true,
        standalone_browser: true,
        status_action: true,
        file_picker: true,
        camera_capture_hint: true,
        geolocation: true,
        external_navigator: true,
        page_errors: pageErrors,
      },
      null,
      2,
    ) + "\n",
  );

  await context.close();
});
