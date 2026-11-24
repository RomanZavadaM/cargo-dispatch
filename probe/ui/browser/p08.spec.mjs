import { test, expect } from "@playwright/test";
import fs from "node:fs";

test("browser document upload and download stay browser-native", async ({ page }) => {
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto("http://127.0.0.1:4174", { waitUntil: "domcontentloaded" });

  const file = page.locator("#driver-file");
  await file.setInputFiles({
    name: "camera-proof.png",
    mimeType: "image/png",
    buffer: Buffer.from([0x89, 0x50, 0x4e, 0x47, 1, 2, 3, 4]),
  });

  const metadata = page.locator("#selected-file-metadata");
  await expect(metadata).toContainText("camera-proof.png");
  await expect(metadata).toContainText("image/png");
  await expect(metadata).toHaveAttribute("data-size", "8");

  const downloadLink = page.locator("#document-download");
  await expect(downloadLink).toHaveAttribute("data-document-id", "doc-ttn");
  await expect(downloadLink).toHaveAttribute("data-mime", "application/pdf");
  await expect(downloadLink).toHaveAttribute("download", "trip-a-ttn.pdf");

  const downloadPromise = page.waitForEvent("download");
  await downloadLink.click();
  const download = await downloadPromise;
  expect(download.suggestedFilename()).toBe("trip-a-ttn.pdf");
  const stream = await download.createReadStream();
  const chunks = [];
  for await (const chunk of stream) chunks.push(chunk);
  const bytes = Buffer.concat(chunks);
  expect(bytes.subarray(0, 4).toString("ascii")).toBe("%PDF");

  const hasTauriInternals = await page.evaluate(() => Boolean(window.__TAURI_INTERNALS__));
  expect(hasTauriInternals).toBe(false);
  expect(pageErrors).toEqual([]);

  fs.mkdirSync("probe/ui/out", { recursive: true });
  fs.writeFileSync(
    "probe/ui/out/p08-documents.json",
    JSON.stringify(
      {
        probe: "P08",
        pass: true,
        browser_upload: true,
        browser_download: true,
        shared_document_metadata: true,
        desktop_api_absent: true,
        page_errors: pageErrors,
      },
      null,
      2,
    ) + "\n",
  );
});
