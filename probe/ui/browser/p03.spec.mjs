import { test, expect } from "@playwright/test";
import fs from "node:fs";

test.setTimeout(90_000);

test("working route protects saved state until explicit acceptance", async ({ page }) => {
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto("http://127.0.0.1:4173", { waitUntil: "domcontentloaded" });
  await expect(page.locator("#pool-count")).toHaveText("5000", { timeout: 30_000 });
  await expect(page.locator("#route-panel")).toBeVisible();

  await expect(page.locator("#saved-route-revision")).toHaveText("1");
  await expect(page.locator("#saved-route-count")).toHaveText("6");
  await expect(page.locator("#draft-route-count")).toHaveText("6");

  const labels = async () =>
    page.locator(".route-point .route-label").allTextContents();

  const originalOrder = await labels();

  const confirmedPoint = page.locator('.route-point[data-point-id="2"]');
  await confirmedPoint.locator(".move-down").click();
  await expect(page.locator("#route-message")).toContainText("cannot move");
  expect(await labels()).toEqual(originalOrder);

  const tentativePoint = page.locator('.route-point[data-point-id="3"]');
  await tentativePoint.locator(".move-down").click();
  const movedOrder = await labels();
  expect(movedOrder).not.toEqual(originalOrder);
  await expect(page.locator("#saved-route-revision")).toHaveText("1");
  await expect(page.locator("#saved-route-count")).toHaveText("6");

  await page.locator("#undo-route").click();
  expect(await labels()).toEqual(originalOrder);
  await expect(page.locator("#draft-route-count")).toHaveText("6");

  await page.locator('.route-point[data-point-id="3"] .toggle-lock').click();
  await expect(page.locator('.route-point[data-point-id="3"]')).toHaveAttribute("data-locked", "true");
  const lockedOrder = await labels();
  await page.locator('.route-point[data-point-id="3"] .move-down').click();
  await expect(page.locator("#route-message")).toContainText("cannot move");
  expect(await labels()).toEqual(lockedOrder);

  await page.locator('.route-point[data-point-id="3"] .toggle-lock').click();
  await expect(page.locator('.route-point[data-point-id="3"]')).toHaveAttribute("data-locked", "false");

  await page.locator('.route-point[data-point-id="3"] .exclude-cargo').click();
  await expect(page.locator("#draft-route-count")).toHaveText("4");
  await expect(page.locator("#saved-route-count")).toHaveText("6");
  await expect(page.locator("#saved-route-revision")).toHaveText("1");
  await expect(page.locator('.route-point[data-point-id="5"]')).toHaveCount(0);

  await page.locator("#recalculate-route").click();
  await expect(page.locator("#route-preview")).toBeVisible();
  await expect(page.locator("#preview-excluded-count")).toHaveText("1");

  const beforeRevenue = Number(await page.locator("#preview-before-revenue").textContent());
  const afterRevenue = Number(await page.locator("#preview-after-revenue").textContent());
  expect(afterRevenue).toBeLessThan(beforeRevenue);
  await expect(page.locator("#saved-route-revision")).toHaveText("1");

  await page.locator("#accept-route").click();
  await expect(page.locator("#route-preview")).toBeHidden();
  await expect(page.locator("#route-message")).toHaveText("Recalculation accepted");
  await expect(page.locator("#saved-route-revision")).toHaveText("2");
  await expect(page.locator("#saved-route-count")).toHaveText("4");
  await expect(page.locator("#draft-route-count")).toHaveText("4");

  expect(pageErrors).toEqual([]);

  fs.mkdirSync("probe/ui/out", { recursive: true });
  fs.writeFileSync(
    "probe/ui/out/p03-browser.json",
    JSON.stringify(
      {
        probe: "working-route",
        pass: true,
        confirmed_point_protected: true,
        draft_isolated_from_saved_route: true,
        undo_restores_saved_route: true,
        lock_blocks_move: true,
        cargo_exclusion_is_draft_only_before_accept: true,
        recalculation_preview_before_accept: true,
        explicit_accept_updates_saved_revision: true,
        saved_points_after_accept: 4,
        page_errors: pageErrors,
      },
      null,
      2,
    ) + "\n",
  );
});
