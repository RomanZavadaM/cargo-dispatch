import { test, expect } from "@playwright/test";
import fs from "node:fs";

test("planner works predictably from keyboard and restores focus", async ({ page }) => {
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  await page.goto("http://127.0.0.1:4173", { waitUntil: "domcontentloaded" });
  await page.locator("#ui-language").selectOption("en");

  const filter = page.locator("#pool-filter");
  await filter.focus();
  await page.keyboard.press("Tab");
  await expect(page.locator("#sort-age")).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(page.locator("#sort-rate")).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(page.locator("#pool-scroll")).toBeFocused();
  await page.keyboard.press("Tab");

  const firstCheckbox = page.locator(".row-select").first();
  await expect(firstCheckbox).toBeFocused();
  const outline = await firstCheckbox.evaluate((element) => {
    const style = getComputedStyle(element);
    return { style: style.outlineStyle, width: style.outlineWidth };
  });
  expect(outline.style).not.toBe("none");
  expect(outline.width).not.toBe("0px");

  await page.keyboard.press("Space");
  await expect(page.locator("#selected-count")).toHaveText("1");

  await page.keyboard.press("Tab");
  const firstOpen = page.locator(".open-row").first();
  await expect(firstOpen).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.locator("#row-detail")).toBeVisible();

  const edit = page.locator("#edit-request");
  await edit.focus();
  await page.keyboard.press("Enter");
  await expect(page.locator("#request-editor")).toBeVisible();
  await expect(page.locator("#editor-origin")).toBeFocused();

  await page.keyboard.press("Escape");
  await expect(page.locator("#request-editor")).toHaveCount(0);
  await expect(edit).toBeFocused();

  await page.keyboard.press("Enter");
  await expect(page.locator("#editor-origin")).toBeFocused();
  await page.locator("#editor-origin").fill("Vienna");
  await page.keyboard.press("Escape");
  await expect(page.locator("#unsaved-warning")).toBeVisible();
  await expect(page.locator("#keep-editing")).toBeFocused();

  await page.keyboard.press("Escape");
  await expect(page.locator("#unsaved-warning")).toHaveCount(0);
  await expect(page.locator("#request-editor")).toBeVisible();
  await expect(page.locator("#editor-close")).toBeFocused();

  const originHasLabel = await page.locator("#editor-origin").evaluate(
    (element) => element.labels && element.labels.length === 1,
  );
  expect(originHasLabel).toBe(true);

  await page.locator("#editor-cancel").focus();
  await page.keyboard.press("Space");
  await expect(page.locator("#request-editor")).toHaveCount(0);
  await expect(edit).toBeFocused();

  expect(pageErrors).toEqual([]);
  fs.mkdirSync("probe/ui/out", { recursive: true });
  fs.writeFileSync(
    "probe/ui/out/p10-accessibility-planner.json",
    JSON.stringify(
      {
        probe: "P10",
        pass: true,
        tab_order: true,
        space_selection: true,
        enter_activation: true,
        escape_behavior: true,
        focus_return: true,
        visible_focus: true,
        explicit_labels: true,
        page_errors: pageErrors,
      },
      null,
      2,
    ) + "\n",
  );
});
