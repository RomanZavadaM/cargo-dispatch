import { test, expect } from "@playwright/test";
import fs from "node:fs";

test.use({ viewport: { width: 360, height: 760 } });

test("driver page remains touchable, labelled and usable in portrait", async ({ page }) => {
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  await page.goto("http://127.0.0.1:4174", { waitUntil: "domcontentloaded" });

  const noHorizontalOverflow = await page.evaluate(
    () => document.documentElement.scrollWidth <= window.innerWidth,
  );
  expect(noHorizontalOverflow).toBe(true);

  const controls = [
    page.locator("#driver-language"),
    page.locator("#status-button"),
    page.locator("#geo-button"),
    page.locator("#navigator-link"),
    page.locator("#document-download"),
  ];
  for (const control of controls) {
    const box = await control.boundingBox();
    expect(box).not.toBeNull();
    expect(box.height).toBeGreaterThanOrEqual(44);
  }

  await page.keyboard.press("Tab");
  await expect(page.locator("#driver-language")).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(page.locator("#status-button")).toBeFocused();

  const focusOutline = await page.locator("#status-button").evaluate((element) => {
    const style = getComputedStyle(element);
    return { style: style.outlineStyle, width: style.outlineWidth };
  });
  expect(focusOutline.style).not.toBe("none");
  expect(focusOutline.width).not.toBe("0px");

  await page.keyboard.press("Space");
  await expect(page.locator("#driver-status")).toHaveText("Прибув");

  await expect(page.getByRole("button", { name: "Змінити статус" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Відкрити навігатор" })).toBeVisible();
  await expect(page.locator("#driver-status")).toHaveAttribute("role", "status");
  await expect(page.locator("#driver-status")).toHaveAttribute("aria-live", "polite");

  const fileHasLabel = await page.locator("#driver-file").evaluate(
    (element) => element.labels && element.labels.length === 1,
  );
  expect(fileHasLabel).toBe(true);
  await expect(page.locator("#driver-documents")).toHaveAttribute("aria-label", "Документи");

  expect(pageErrors).toEqual([]);
  fs.mkdirSync("probe/ui/out", { recursive: true });
  fs.writeFileSync(
    "probe/ui/out/p10-accessibility-driver.json",
    JSON.stringify(
      {
        probe: "P10",
        pass: true,
        portrait_width: 360,
        no_horizontal_overflow: true,
        touch_targets_min_44: true,
        keyboard_activation: true,
        visible_focus: true,
        screen_reader_labels: true,
        page_errors: pageErrors,
      },
      null,
      2,
    ) + "\n",
  );
});
