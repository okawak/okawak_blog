import { expect, test } from "@playwright/test";

const baseURL = "http://127.0.0.1:8008";

test("language links select existing translations and matching metadata", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", error => errors.push(error.message));
  await page.goto("/");
  await expect(page.locator("html")).toHaveAttribute("lang", "ja");
  await page.getByRole("link", { name: "English", exact: true }).click();
  await expect(page).toHaveURL(`${baseURL}/en`);
  await expect(page.locator("html")).toHaveAttribute("lang", "en");
  await expect(page.getByRole("heading", { name: "Recent articles" })).toBeVisible();
  await expect(page.locator('link[rel="canonical"]')).toHaveAttribute("href", `${baseURL}/en`);
  await expect(page.locator('meta[property="og:locale"]')).toHaveAttribute("content", "en_US");
  await expect(page.locator('link[rel="alternate"][hreflang="ja"]')).toHaveAttribute("href", baseURL);
  await page.getByRole("link", { name: "Translated E2E Article", exact: true }).click();
  await expect(page).toHaveURL(`${baseURL}/en/tech/e2e-article`);
  await expect(page.locator('link[rel="canonical"]')).toHaveAttribute("href", `${baseURL}/en/tech/e2e-article`);
  await expect(page.locator("main")).toContainText("#Browser testing");
  await page.getByRole("link", { name: "日本語", exact: true }).click();
  await expect(page).toHaveURL(`${baseURL}/tech/e2e-article`);
  expect(errors).toEqual([]);
});

test("English category filtering uses translated labels and keeps its locale after shard updates", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", error => errors.push(error.message));
  await page.goto("/en/tech");
  const query = page.getByRole("searchbox", { name: "Filter articles" });
  await expect(query).toHaveAttribute("placeholder", "Title, description, or tags");
  await query.fill("browser testing");
  await expect(page.getByRole("status")).toHaveText("1 article");
  await expect(page.locator("#category-articles")).toContainText("#Browser testing");
  await query.fill("missing tag");
  await expect(page.getByRole("status")).toHaveText("0 articles");
  await expect(page.getByText("No matching articles.", { exact: true })).toBeVisible();
  await query.fill("");
  await expect(page.getByRole("status")).toHaveText("1 article");
  await expect(query).toHaveAttribute("placeholder", "Title, description, or tags");
  await expect(page.locator("html")).toHaveAttribute("lang", "en");
  await expect(page.locator('meta[name="description"]')).toHaveAttribute("content", "English category description");
  expect(errors).toEqual([]);
});

test("English mobile navigation keeps localized client labels", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/en/about");
  await page.getByRole("button", { name: "Open navigation menu" }).click();
  await expect(page.getByRole("button", { name: "Close navigation menu" })).toHaveAttribute("aria-expanded", "true");
  await page.getByRole("link", { name: "Home", exact: true }).click();
  await expect(page).toHaveURL(`${baseURL}/en`);
  await expect(page.locator("html")).toHaveAttribute("lang", "en");
});

test("English SSR works without JavaScript and untranslated pages do not masquerade as translations", async ({ browser }) => {
  const context = await browser.newContext({ javaScriptEnabled: false });
  try {
    const page = await context.newPage();
    const response = await page.goto(`${baseURL}/en/tech/e2e-article`);
    expect(response?.status()).toBe(200);
    await expect(page.getByRole("heading", { name: "Translated E2E Article", exact: true })).toBeVisible();
    await expect(page.locator("html")).toHaveAttribute("lang", "en");
    const missing = await page.goto(`${baseURL}/en/daily`);
    expect(missing?.status()).toBe(404);
    await expect(page.getByText("Page not found.", { exact: true })).toBeVisible();
    await expect(page.locator('link[rel="alternate"]')).toHaveCount(0);
  } finally {
    await context.close();
  }
});
