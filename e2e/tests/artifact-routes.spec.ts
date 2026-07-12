import { expect, test, type Page } from "@playwright/test";

const SITE_NAME = "ぶくせんの探窟メモ";

async function expectMetadata(
  page: Page,
  title: string,
  canonicalPath: string,
  ogType = "website",
) {
  await expect(page.locator('link[rel="canonical"]')).toHaveCount(1);
  await expect(page.locator('meta[property="og:title"]')).toHaveCount(1);
  await expect(page.locator('meta[property="og:type"]')).toHaveCount(1);
  await expect(page).toHaveTitle(title);
  await expect(page.locator('link[rel="canonical"]')).toHaveAttribute(
    "href",
    `http://127.0.0.1:8008${canonicalPath}`,
  );
  await expect(page.locator('meta[property="og:title"]')).toHaveAttribute(
    "content",
    title,
  );
  await expect(page.locator('meta[property="og:type"]')).toHaveAttribute(
    "content",
    ogType,
  );
}

async function expectNotFoundMetadata(page: Page, canonicalPath: string) {
  const title = `ページが見つかりません | ${SITE_NAME}`;
  const description = "お探しのページは見つかりませんでした。";

  await expectMetadata(page, title, canonicalPath);
  await expect(page.locator('meta[name="description"]')).toHaveAttribute(
    "content",
    description,
  );
  await expect(page.locator('meta[property="og:description"]')).toHaveAttribute(
    "content",
    description,
  );
}

test("runtime probes distinguish liveness and artifact readiness", async ({ request }) => {
  const healthResponse = await request.get("/api/health");
  expect(healthResponse.status()).toBe(200);
  expect(await healthResponse.text()).toBe("OK");

  const readinessResponse = await request.get("/api/ready");
  expect(readinessResponse.status()).toBe(200);
  expect(await readinessResponse.text()).toBe("READY");
});

test("home renders artifacts and hydrates article navigation", async ({ page }) => {
  const response = await page.goto("/");

  expect(response?.status()).toBe(200);
  await expect(page.locator("main").getByRole("heading", { name: SITE_NAME })).toBeVisible();
  await expect(page.getByText("Fixture home content")).toBeVisible();
  await expect(page.getByRole("link", { name: "E2E Article" })).toBeVisible();
  await expectMetadata(page, SITE_NAME, "");

  let documentRequests = 0;
  page.on("request", (request) => {
    if (request.resourceType() === "document") documentRequests += 1;
  });

  await page.getByRole("link", { name: "E2E Article" }).click();

  await expect(page).toHaveURL(/\/tech\/e2e-article$/);
  await expect(page.getByRole("heading", { name: "E2E Article" })).toBeVisible();
  expect(documentRequests).toBe(0);
  await expectMetadata(
    page,
    `E2E Article | ${SITE_NAME}`,
    "/tech/e2e-article",
    "article",
  );
});

test("about renders its page artifact", async ({ page }) => {
  const response = await page.goto("/about");

  expect(response?.status()).toBe(200);
  await expect(page.getByRole("heading", { name: "Fixture About" })).toBeVisible();
  await expect(page.getByText("About fixture body")).toBeVisible();
  await expectMetadata(page, `Fixture About | ${SITE_NAME}`, "/about");
});

test("category renders landing content and grouped articles", async ({ page }) => {
  const response = await page.goto("/tech");

  expect(response?.status()).toBe(200);
  await expect(page.getByText("Fixture Tech", { exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Tech landing" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "rust / async" })).toBeVisible();
  await expect(page.getByRole("link", { name: "E2E Article" })).toBeVisible();
  await expectMetadata(page, `Fixture Tech | ${SITE_NAME}`, "/tech");
});

test("missing article and category return 404 pages", async ({ page }) => {
  const articleResponse = await page.goto("/tech/missing-article");

  expect(articleResponse?.status()).toBe(404);
  await expect(page.getByText("ページが見つかりませんでした。")).toBeVisible();
  await expectNotFoundMetadata(page, "/tech/missing-article");

  const categoryResponse = await page.goto("/statistics");

  expect(categoryResponse?.status()).toBe(404);
  await expect(page.getByText("ページが見つかりませんでした。")).toBeVisible();
  await expectNotFoundMetadata(page, "/statistics");
});
