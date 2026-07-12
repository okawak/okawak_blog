import { expect, test } from "@playwright/test";

const SITE_NAME = "ぶくせんの探窟メモ";

test("empty home keeps its metadata in the initial response", async ({ page }) => {
  const response = await page.goto("/");

  expect(response?.status()).toBe(200);
  await expect(page.getByText("記事がありません")).toBeVisible();
  await expect(page).toHaveTitle(SITE_NAME);
  await expect(page.locator('meta[name="description"]')).toHaveAttribute(
    "content",
    "0件の記事を0カテゴリで公開しています。",
  );
  await expect(page.locator('link[rel="canonical"]')).toHaveAttribute(
    "href",
    "http://127.0.0.1:8008",
  );
  await expect(page.locator('meta[property="og:title"]')).toHaveAttribute(
    "content",
    SITE_NAME,
  );
  await expect(page.locator('meta[property="og:description"]')).toHaveAttribute(
    "content",
    "0件の記事を0カテゴリで公開しています。",
  );
  await expect(page.locator('meta[property="og:url"]')).toHaveAttribute(
    "content",
    "http://127.0.0.1:8008",
  );
});
