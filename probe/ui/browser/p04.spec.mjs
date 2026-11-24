import { test, expect } from "@playwright/test";
import fs from "node:fs";

test.setTimeout(120_000);

test("map bridge keeps map state outside application widgets", async ({ page }) => {
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto("http://127.0.0.1:4173", { waitUntil: "domcontentloaded" });
  await expect(page.locator("#map-route-count")).toHaveText("3", { timeout: 30_000 });
  await expect(page.locator("#map-point-count")).toHaveText("200");
  await expect(page.locator(".map-route-choice")).toHaveCount(3);
  await expect(page.locator(".map-point-choice")).toHaveCount(200);

  await expect
    .poll(async () => Number(await page.locator("#map-generation").textContent()), {
      timeout: 30_000,
    })
    .toBeGreaterThan(0);

  const initialState = await page.evaluate(() => window.cargoMapBridge.getState());
  expect(initialState.routeCount).toBe(3);
  expect(initialState.pointCount).toBe(200);
  expect(initialState.loaded).toBe(true);

  await page.locator('.map-route-choice[data-route-id="route-b"]').click();
  await expect(page.locator("#map-selected-route")).toHaveText("route-b");
  await expect(page.locator('.map-route-choice[data-route-id="route-b"]')).toHaveAttribute(
    "data-selected",
    "true",
  );
  expect(await page.evaluate(() => window.cargoMapBridge.getState().selectedRoute)).toBe("route-b");

  await page.locator('.map-point-choice[data-point-id="point-042"]').click();
  await expect(page.locator("#map-selected-point")).toHaveText("point-042");
  await expect(page.locator('.map-point-choice[data-point-id="point-042"]')).toHaveAttribute(
    "data-selected",
    "true",
  );

  const manualMarker = page.locator(".manual-map-marker");
  await expect(manualMarker).toBeVisible();
  await manualMarker.click();
  await expect(page.locator("#map-selected-point")).toHaveText("manual-1");
  await expect(page.locator('.map-point-choice[data-point-id="manual-1"]')).toHaveAttribute(
    "data-selected",
    "true",
  );

  const beforeCoordinate = await page.locator("#map-manual-coordinate").textContent();
  const box = await manualMarker.boundingBox();
  expect(box).not.toBeNull();
  const centerX = box.x + box.width / 2;
  const centerY = box.y + box.height / 2;
  await page.mouse.move(centerX, centerY);
  await page.mouse.down();
  await page.mouse.move(centerX + 45, centerY + 24, { steps: 8 });
  await page.mouse.up();

  await expect
    .poll(async () => page.locator("#map-manual-coordinate").textContent(), {
      timeout: 10_000,
    })
    .not.toBe(beforeCoordinate);

  const movedState = await page.evaluate(() => window.cargoMapBridge.getState());
  expect(movedState.selectedPoint).toBe("manual-1");
  expect(movedState.manualCoordinate).not.toEqual(initialState.manualCoordinate);

  await page.locator("#map-fit-all").click();
  const firstGeneration = Number(await page.locator("#map-generation").textContent());
  await page.locator("#map-remount").click();
  await expect
    .poll(async () => Number(await page.locator("#map-generation").textContent()), {
      timeout: 30_000,
    })
    .toBeGreaterThan(firstGeneration);

  const remountedState = await page.evaluate(() => window.cargoMapBridge.getState());
  expect(remountedState.loaded).toBe(true);
  expect(remountedState.routeCount).toBe(3);
  expect(remountedState.pointCount).toBe(200);

  expect(pageErrors).toEqual([]);

  fs.mkdirSync("probe/ui/out", { recursive: true });
  fs.writeFileSync(
    "probe/ui/out/p04-browser.json",
    JSON.stringify(
      {
        probe: "map-bridge",
        pass: true,
        route_alternatives: 3,
        points: 200,
        route_selection_sync: true,
        point_selection_sync: true,
        manual_marker_drag: true,
        remount_cleanup: true,
        generation_before: firstGeneration,
        generation_after: remountedState.generation,
        page_errors: pageErrors,
      },
      null,
      2,
    ) + "\n",
  );
});
