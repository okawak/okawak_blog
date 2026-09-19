import { expect, test } from "@playwright/test";

const baseURL = "http://127.0.0.1:8008";

type ArticleIndex = {
  articles: Array<{
    category: string;
    slug: string;
    title: string;
  }>;
};

test("S3 release artifacts pass readiness and render a published article", async ({
  page,
  request,
}) => {
  const healthResponse = await request.get("/api/health");
  expect(healthResponse.status()).toBe(200);
  expect(await healthResponse.text()).toBe("OK");

  const readinessResponse = await request.get("/api/ready");
  expect(readinessResponse.status()).toBe(200);
  expect(await readinessResponse.text()).toBe("READY");

  const articleIndexResponse = await request.get("/api/articles");
  expect(articleIndexResponse.status()).toBe(200);
  const etag = articleIndexResponse.headers().etag;
  const lastModified = articleIndexResponse.headers()["last-modified"];
  expect(Boolean(lastModified)).toBe(Boolean(etag));
  if (etag && lastModified) {
    expect(Number.isNaN(Date.parse(lastModified))).toBe(false);

    const conditionalResponse = await request.get("/api/articles", {
      headers: { "If-Modified-Since": lastModified },
    });
    expect(conditionalResponse.status()).toBe(304);

    const etagPrecedenceResponse = await request.get("/api/articles", {
      headers: {
        "If-None-Match": '"different"',
        "If-Modified-Since": lastModified,
      },
    });
    expect(etagPrecedenceResponse.status()).toBe(200);
  }
  const articleIndex = (await articleIndexResponse.json()) as ArticleIndex;
  expect(articleIndex.articles.length).toBeGreaterThan(0);

  const homeResponse = await page.goto("/?lang=ja");
  expect(homeResponse?.status()).toBe(200);
  await expect(page.locator("main")).toBeVisible();
  await expect(page.locator('link[rel="canonical"]')).toHaveAttribute(
    "href",
    baseURL,
  );

  const article = articleIndex.articles[0];
  const articlePath = `/${encodeURIComponent(article.category)}/${encodeURIComponent(article.slug)}`;
  const articleResponse = await page.goto(articlePath);

  expect(articleResponse?.status()).toBe(200);
  await expect(
    page.getByRole("heading", { level: 1, name: article.title, exact: true }),
  ).toBeVisible();
  await expect(page.locator('link[rel="canonical"]')).toHaveAttribute(
    "href",
    `${baseURL}${articlePath}`,
  );
  await expect(page.locator('meta[property="og:type"]')).toHaveAttribute(
    "content",
    "article",
  );
});

test("S3 release serves the languages declared by the reviewed public build", async ({ page }) => {
  const expectedRoot = process.env.OKAWAK_BLOG_EXPECTED_ARTIFACT_ROOT;
  if (expectedRoot) {
    const { readFileSync } = await import("node:fs");
    const { join } = await import("node:path");
    const { verifyReleaseLocales } = await import("../../helpers/release-locales");
    const catalog = JSON.parse(readFileSync(join(expectedRoot, "locales.json"), "utf8"));
    await verifyReleaseLocales(page, catalog, baseURL);
  } else {
    // Manual smoke can target an existing release without a matching local build.
    await page.goto("/?lang=ja");
    const english = page.locator('link[rel="alternate"][hreflang="en"]');
    if (await english.count()) {
      await page.goto((await english.getAttribute("href"))!);
      await expect(page.locator("html")).toHaveAttribute("lang", "en");
      await expect(page.locator("main")).toBeVisible();
    }
  }
});
