import { test, expect } from "@playwright/test";
import fs from "node:fs";

test("driver browser uses the same language codes without losing state", async ({ page }) => {
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto("http://127.0.0.1:4174", { waitUntil: "domcontentloaded" });

  const language = page.locator("#driver-language");
  await expect(language).toHaveValue("uk");
  await expect(page.locator("#driver-status")).toHaveText("У дорозі");
  await expect(page.locator("#status-button")).toHaveText("Змінити статус");

  await page.locator("#status-button").click();
  await expect(page.locator("#driver-status")).toHaveText("Прибув");

  await language.selectOption("de");
  await expect(page.locator("#driver-status")).toHaveText("Angekommen");
  await expect(page.locator("#status-button")).toHaveText("Status ändern");
  await expect(page.locator("#document-download")).toHaveText("Dokument herunterladen");
  await expect(page.locator("#driver-documents li").first()).toContainText("erforderlich");
  await expect(page.locator("html")).toHaveAttribute("lang", "de");

  await language.selectOption("en");
  await expect(page.locator("#driver-status")).toHaveText("Arrived");
  await expect(page.locator("#status-button")).toHaveText("Change status");
  await expect(page.locator("#navigator-link")).toHaveText("Open navigator");

  await language.selectOption("uk");
  await expect(page.locator("#driver-status")).toHaveText("Прибув");
  await expect(page.locator("#navigator-link")).toHaveText("Відкрити навігатор");

  expect(errors).toEqual([]);

  fs.mkdirSync("probe/ui/out", { recursive: true });
  fs.writeFileSync(
    "probe/ui/out/p09-i18n-driver.json",
    JSON.stringify(
      {
        probe: "P09",
        pass: true,
        languages: ["uk", "en", "de"],
        runtime_switch: true,
        state_preserved: true,
        shared_language_codes: true,
        driver_labels: true,
        page_errors: errors,
      },
      null,
      2,
    ) + "\n",
  );
});
