import { expect, test, type Page } from "@playwright/test";

const SITE_NAME = "ぶくせんの探窟メモ";

function captureReactiveWarnings(page: Page) {
  const warnings: string[] = [];
  page.on("console", (message) => {
    if (message.text().includes("outside a reactive tracking context")) {
      warnings.push(message.text());
    }
  });
  return warnings;
}

function captureBrowserErrors(page: Page) {
  const errors: string[] = [];
  page.on("pageerror", (error) => {
    errors.push(`pageerror: ${error.message}`);
  });
  page.on("console", (message) => {
    if (message.type() === "error") {
      errors.push(`console.error: ${message.text()}`);
    }
  });
  return errors;
}

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

async function expectFormattedFixtureDates(page: Page) {
  await expect(
    page.locator('time[datetime="2026-01-01T00:00"]'),
  ).toHaveText("2026年1月1日");
  await expect(
    page.locator('time[datetime="2026-01-02T00:00:00+09:00"]'),
  ).toHaveText("2026年1月2日");
}

test("runtime probes distinguish liveness and artifact readiness", async ({ request }) => {
  const healthResponse = await request.get("/api/health");
  expect(healthResponse.status()).toBe(200);
  expect(await healthResponse.text()).toBe("OK");

  const readinessResponse = await request.get("/api/ready");
  expect(readinessResponse.status()).toBe(200);
  expect(await readinessResponse.text()).toBe("READY");
});

test("frontend assets fall back to the previous release", async ({ request }) => {
  const response = await request.get("/pkg/e2e-previous-release.txt");

  expect(response.status()).toBe(200);
  expect(await response.text()).toBe("previous release asset\n");
});

test("site declares and serves its favicon", async ({ page, request }) => {
  await page.goto("/");

  const iconLink = page.locator('link[rel~="icon"]');
  await expect(iconLink).toHaveCount(1);
  await expect(iconLink).toHaveAttribute(
    "href",
    "/favicon.ico?v=f544a69c",
  );
  await expect(iconLink).toHaveAttribute("sizes", "16x16 32x32 48x48");

  const response = await request.get("/favicon.ico?v=f544a69c");
  expect(response.status()).toBe(200);
  expect(response.headers()["content-type"]).toMatch(/^image\//);
  expect((await response.body()).byteLength).toBeGreaterThan(0);
});

test("home renders artifacts and hydrates article navigation", async ({ page }) => {
  const reactiveWarnings = captureReactiveWarnings(page);
  const browserErrors = captureBrowserErrors(page);

  const response = await page.goto("/");

  expect(response?.status()).toBe(200);
  await expect(page.locator('link#leptos[rel="stylesheet"]')).toHaveAttribute(
    "href",
    /\/pkg\/web\.[A-Za-z0-9_-]+\.css$/,
  );
  await expect(page.locator("main").getByRole("heading", { name: SITE_NAME })).toBeVisible();
  await expect(page.getByText("Fixture home content")).toBeVisible();
  await expect(page.locator("main .content-prose")).toContainText("Fixture home content");
  await expect(page.getByRole("link", { name: "E2E Article" })).toBeVisible();
  await expectFormattedFixtureDates(page);
  await expectMetadata(page, SITE_NAME, "");

  let documentRequests = 0;
  page.on("request", (request) => {
    if (request.resourceType() === "document") documentRequests += 1;
  });

  await page.getByRole("link", { name: "E2E Article" }).click();

  await expect(page).toHaveURL(/\/tech\/e2e-article$/);
  await expect(page.getByRole("heading", { name: "E2E Article" })).toBeVisible();
  await expect(page.locator("main .content-prose")).toContainText("Article fixture body");
  await expectFormattedFixtureDates(page);
  const articleWidths = await page.locator("main article").evaluate((article) => {
    const header = article.querySelector(":scope > header");
    const prose = article.querySelector(":scope > .content-prose");
    return {
      header: header?.getBoundingClientRect().width ?? 0,
      prose: prose?.getBoundingClientRect().width ?? 0,
    };
  });
  expect(articleWidths.prose).toBeCloseTo(articleWidths.header, 0);
  expect(documentRequests).toBe(0);
  await expectMetadata(
    page,
    `E2E Article | ${SITE_NAME}`,
    "/tech/e2e-article",
    "article",
  );
  expect(reactiveWarnings).toEqual([]);
  expect(browserErrors).toEqual([]);
});

test("site shell keeps the warm gradient background", async ({ page }) => {
  await page.goto("/");

  const backgroundImage = await page.evaluate(
    () => getComputedStyle(document.body).backgroundImage,
  );
  expect(backgroundImage).toContain("linear-gradient");
});

test("mobile navigation stays in the viewport and exposes its state", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/");

  const header = page.locator("header");
  const logo = header.getByRole("link", { name: SITE_NAME });
  const menuButton = header.locator('button[aria-controls="site-header-nav"]');
  const navigation = header.locator("#site-header-nav");

  await expect(logo).toBeVisible();
  await expect(menuButton).toBeVisible();
  await expect(menuButton).toHaveAttribute("aria-expanded", "false");
  await expect(menuButton).toHaveAttribute(
    "aria-label",
    "ナビゲーションメニューを開く",
  );
  await expect(navigation).toBeHidden();

  const logoBox = await logo.boundingBox();
  const buttonBox = await menuButton.boundingBox();
  expect(logoBox).not.toBeNull();
  expect(buttonBox).not.toBeNull();
  expect(logoBox!.x + logoBox!.width).toBeLessThan(buttonBox!.x);

  await menuButton.click();

  await expect(menuButton).toHaveAttribute("aria-expanded", "true");
  await expect(menuButton).toHaveAttribute(
    "aria-label",
    "ナビゲーションメニューを閉じる",
  );
  await expect(navigation).toBeVisible();

  const navigationBox = await navigation.boundingBox();
  expect(navigationBox).not.toBeNull();
  expect(navigationBox!.x).toBeGreaterThanOrEqual(0);
  expect(navigationBox!.x + navigationBox!.width).toBeLessThanOrEqual(390);

  await navigation.getByRole("link", { name: "About" }).click();

  await expect(page).toHaveURL(/\/about$/);
  await expect(navigation).toBeHidden();
  await expect(menuButton).toHaveAttribute("aria-expanded", "false");
});

test("home article cards stay within the mobile viewport", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/");

  const articleCard = page.locator("main article").filter({ hasText: "E2E Article" });
  await expect(articleCard).toBeVisible();

  const cardBox = await articleCard.boundingBox();
  expect(cardBox).not.toBeNull();
  expect(cardBox!.x).toBeGreaterThanOrEqual(0);
  expect(cardBox!.x + cardBox!.width).toBeLessThanOrEqual(390);
});

test("about renders its page artifact", async ({ page }) => {
  const response = await page.goto("/about");

  expect(response?.status()).toBe(200);
  await expect(page.getByRole("heading", { name: "Fixture About" })).toBeVisible();
  await expect(page.getByText("About fixture body")).toBeVisible();
  await expect(page.locator("main .content-prose")).toContainText("About fixture body");
  await expectMetadata(page, `Fixture About | ${SITE_NAME}`, "/about");
});

test("category renders landing content and grouped articles", async ({ page }) => {
  const reactiveWarnings = captureReactiveWarnings(page);
  const response = await page.goto("/tech");

  expect(response?.status()).toBe(200);
  await expect(page.getByText("Fixture Tech", { exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Tech landing" })).toBeVisible();
  await expect(page.locator("main .content-prose")).toContainText("Category fixture body");
  await expect(page.getByRole("heading", { name: "rust / async" })).toBeVisible();
  await expect(page.getByRole("link", { name: "E2E Article" })).toBeVisible();
  await expectFormattedFixtureDates(page);
  await expectMetadata(page, `Fixture Tech | ${SITE_NAME}`, "/tech");
  expect(reactiveWarnings).toEqual([]);
});

test("category landing content stays within the mobile viewport", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/tech");

  const wideContent = page.getByTestId("wide-landing-content");
  await expect(wideContent).toBeVisible();

  const fitsLandingSection = await wideContent.evaluate((element) => {
    const landingSection = element.closest("section");
    if (!landingSection) {
      return false;
    }

    return element.getBoundingClientRect().width <= landingSection.getBoundingClientRect().width;
  });
  expect(fitsLandingSection).toBe(true);

  const pageHasNoHorizontalOverflow = await page.evaluate(
    () => document.documentElement.scrollWidth <= document.documentElement.clientWidth,
  );
  expect(pageHasNoHorizontalOverflow).toBe(true);
});

test("generated article content stays readable on mobile", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/tech/e2e-article");

  const prose = page.locator("main .content-prose");
  const wideImage = page.getByTestId("article-wide-image");
  const wideCode = page.getByTestId("article-wide-code");
  const wideTable = page.getByTestId("article-wide-table");

  await expect(prose).toBeVisible();
  await expect(page.getByTestId("article-bookmark")).toBeVisible();
  await expect(page.getByTestId("article-katex")).toBeVisible();
  await expect(wideCode.locator("code")).toHaveClass(/hljs/);
  await expect(wideCode.locator(".hljs-keyword").first()).toBeVisible();

  const contentStyles = await prose.evaluate((element) => ({
    textAlign: getComputedStyle(element).textAlign,
    imageFits: (() => {
      const image = element.querySelector('[data-testid="article-wide-image"]');
      return image
        ? image.getBoundingClientRect().width <= element.getBoundingClientRect().width
        : false;
    })(),
  }));
  expect(contentStyles.textAlign).toBe("left");
  expect(contentStyles.imageFits).toBe(true);
  await expect(wideImage).toBeVisible();
  expect(await wideCode.evaluate((element) => getComputedStyle(element).overflowX)).toBe("auto");
  expect(await wideTable.evaluate((element) => getComputedStyle(element).overflowX)).toBe("auto");

  const pageHasNoHorizontalOverflow = await page.evaluate(
    () => document.documentElement.scrollWidth <= document.documentElement.clientWidth,
  );
  expect(pageHasNoHorizontalOverflow).toBe(true);
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

test("artifact read errors return 500 responses", async ({ request }) => {
  const response = await request.get("/physics");

  expect(response.status()).toBe(500);
  expect(response.headers().etag).toBeUndefined();
});
