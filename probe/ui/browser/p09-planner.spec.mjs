import { test, expect } from "@playwright/test";
import fs from "node:fs";

test("planner switches Ukrainian English and German without restart", async ({ page }) => {
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto("http://127.0.0.1:4173", { waitUntil: "domcontentloaded" });

  const language = page.locator("#ui-language");
  await expect(language).toHaveValue("uk");
  await expect(page.locator("#planner-title")).toHaveText("Пул заявок і редагування");
  await expect(page.locator("#table-origin")).toHaveText("Звідки");
  await expect(page.locator("#number-format-sample")).toHaveText("12 345");
  await expect(page.locator("#date-format-sample")).toHaveText("24.11.2026");

  await language.selectOption("de");
  await expect(page.locator("#planner-title")).toHaveText("Auftragspool und Auftragsbearbeitung");
  await expect(page.locator("#table-origin")).toHaveText("Abholort");
  await expect(page.locator("#table-destination")).toHaveText("Zielort");
  await expect(page.locator("#number-format-sample")).toHaveText("12.345");
  await expect(page.locator("#date-format-sample")).toHaveText("24.11.2026");

  await page.locator(".open-row").first().click();
  await page.locator("#edit-request").click();
  await expect(page.locator("#mode-phone")).toHaveText("Telefonisch aufgenommener Auftrag");
  const palletLabel = page
    .locator(".editor-field-wrap")
    .filter({ has: page.locator('[data-field-key="requires_pallet_exchange"]') })
    .locator("span");
  await expect(palletLabel).toContainText("Palettentausch mit Rückgabe erforderlich");
  await expect(page.locator("#editor-save")).toHaveText("Speichern");
  await expect(page.locator("#editor-cancel")).toHaveText("Abbrechen");

  const origin = await page.locator("#editor-origin").inputValue();
  await page.locator("#editor-destination").fill(origin);
  await page.locator("#editor-save").click();
  await expect(page.locator("#validation-errors")).toContainText(
    "Abholort und Zielort müssen unterschiedlich sein",
  );

  await language.selectOption("en");
  await expect(page.locator("#planner-title")).toHaveText("Planner pool and request editor");
  await expect(page.locator("#editor-save")).toHaveText("Save");
  await expect(page.locator("#number-format-sample")).toHaveText("12,345");
  await expect(page.locator("#date-format-sample")).toHaveText("11/24/2026");

  await language.selectOption("uk");
  await expect(page.locator("#editor-save")).toHaveText("Зберегти");
  await expect(page.locator("#validation-errors")).toContainText(
    "місце відправлення і призначення мають відрізнятися",
  );

  expect(errors).toEqual([]);

  fs.mkdirSync("probe/ui/out", { recursive: true });
  fs.writeFileSync(
    "probe/ui/out/p09-i18n-planner.json",
    JSON.stringify(
      {
        probe: "P09",
        pass: true,
        languages: ["uk", "en", "de"],
        runtime_switch: true,
        table_headers: true,
        dialog_buttons: true,
        localized_error: true,
        long_label: true,
        number_format: true,
        date_format: true,
        page_errors: errors,
      },
      null,
      2,
    ) + "\n",
  );
});
