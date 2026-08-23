import { expect, test, type Page } from "@playwright/test";

const SITE_NAME = "ぶくせんの探窟メモ";
const BASE_URL = "http://127.0.0.1:8008";

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
  description: string,
  ogType = "website",
) {
  await expect(page.locator('link[rel="canonical"]')).toHaveCount(1);
  await expect(page.locator('meta[name="description"]')).toHaveCount(1);
  await expect(page.locator('meta[property="og:title"]')).toHaveCount(1);
  await expect(page.locator('meta[property="og:description"]')).toHaveCount(1);
  await expect(page.locator('meta[property="og:url"]')).toHaveCount(1);
  await expect(page.locator('meta[property="og:type"]')).toHaveCount(1);
  await expect(page).toHaveTitle(title);
  await expect(page.locator('link[rel="canonical"]')).toHaveAttribute(
    "href",
    `${BASE_URL}${canonicalPath}`,
  );
  await expect(page.locator('meta[property="og:title"]')).toHaveAttribute(
    "content",
    title,
  );
  await expect(page.locator('meta[name="description"]')).toHaveAttribute(
    "content",
    description,
  );
  await expect(page.locator('meta[property="og:description"]')).toHaveAttribute(
    "content",
    description,
  );
  await expect(page.locator('meta[property="og:url"]')).toHaveAttribute(
    "content",
    `${BASE_URL}${canonicalPath}`,
  );
  await expect(page.locator('meta[property="og:type"]')).toHaveAttribute(
    "content",
    ogType,
  );
}

async function expectNotFoundMetadata(page: Page, canonicalPath: string) {
  const title = `ページが見つかりません | ${SITE_NAME}`;
  const description = "お探しのページは見つかりませんでした。";

  await expectMetadata(page, title, canonicalPath, description);
}

async function expectFormattedFixtureDates(page: Page) {
  await expect(
    page.locator('time[datetime="2026-01-01T00:00:00+09:00"]'),
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

test("compatibility article API returns the published index", async ({ request }) => {
  const response = await request.get("/api/articles");

  expect(response.status()).toBe(200);
  expect(response.headers()["content-type"]).toMatch(/^application\/json/);
  expect(await response.json()).toEqual({
    articles: [
      {
        slug: "e2e-article",
        title: "E2E Article",
        category: "tech",
        section_path: ["rust", "async"],
        description: "Article fixture description",
        tags: ["rust", "e2e"],
        priority: 10,
        created_at: "2026-01-01T00:00:00+09:00",
        updated_at: "2026-01-02T00:00:00+09:00",
      },
    ],
  });
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

test("home renders artifacts and navigates to an article without a document reload", async ({ page }) => {
  const browserErrors = captureBrowserErrors(page);

  const response = await page.goto("/");

  expect(response?.status()).toBe(200);
  await expect(page.locator('link[rel="stylesheet"][href^="/"]')).toHaveAttribute(
    "href",
    /^\/[^?]*[.-][A-Za-z0-9_-]{8,}\.css$/,
  );
  await expect(page.locator("main").getByRole("heading", { name: SITE_NAME })).toBeVisible();
  await expect(page.getByText("Fixture home content")).toBeVisible();
  await expect(page.locator("main .content-prose")).toContainText("Fixture home content");
  await expect(page.getByRole("link", { name: "E2E Article" })).toBeVisible();
  await expectFormattedFixtureDates(page);
  await expectMetadata(
    page,
    SITE_NAME,
    "",
    "1件の記事を1カテゴリで公開しています。",
  );

  let documentRequests = 0;
  page.on("request", (request) => {
    if (request.resourceType() === "document") documentRequests += 1;
  });

  const articleLink = page.getByRole("link", { name: "E2E Article" });
  const homeScrollY = await articleLink.evaluate((link: HTMLAnchorElement) => {
    window.scrollTo(0, document.documentElement.scrollHeight);
    const scrollY = window.scrollY;
    link.click();
    return scrollY;
  });
  expect(homeScrollY).toBeGreaterThan(500);

  await expect(page).toHaveURL(/\/tech\/e2e-article$/);
  await expect(page.getByRole("heading", { name: "E2E Article" })).toBeVisible();
  await expect(page.locator("main")).toBeFocused();
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
    "Article fixture description",
    "article",
  );

  await page.goBack();

  await expect(page).toHaveURL(/\/$/);
  await expect(page.getByText("Fixture home content")).toBeVisible();
  await expectMetadata(
    page,
    SITE_NAME,
    "",
    "1件の記事を1カテゴリで公開しています。",
  );
  await expect
    .poll(async () => Math.abs((await page.evaluate(() => window.scrollY)) - homeScrollY))
    .toBeLessThanOrEqual(10);
  expect(documentRequests).toBe(0);
  expect(browserErrors).toEqual([]);
});

test("client navigation scrolls to a fragment on another page", async ({ page }) => {
  await page.goto("/");

  await page.getByRole("link", { name: "Generated content section" }).click();

  await expect(page).toHaveURL(/\/tech\/e2e-article#generated-content$/);
  const heading = page.getByRole("heading", { name: "Generated content" });
  await expect(heading).toBeVisible();
  await expect(heading).toBeFocused();
  await expect
    .poll(() => heading.evaluate((element) => element.getBoundingClientRect().top))
    .toBeLessThan(200);
});

test("text fragments use document navigation", async ({ page }) => {
  await page.goto("/");
  let documentRequests = 0;
  page.on("request", (request) => {
    if (
      request.resourceType() === "document" &&
      new URL(request.url()).pathname === "/about"
    ) {
      documentRequests += 1;
    }
  });

  const main = page.locator("main");
  await main.evaluate((element) => element.setAttribute("data-original", "true"));
  const aboutLink = page.getByRole("link", { name: "About", exact: true });
  await aboutLink.evaluate((element) =>
    element.setAttribute("href", "/about#:~:text=About%20fixture%20body"),
  );
  await aboutLink.click();

  // Chromium removes the fragment directive from the document-visible URL.
  await expect(page).toHaveURL(/\/about$/);
  await expect(page.getByRole("heading", { name: "Fixture About" })).toBeVisible();
  await expect(page.locator('main[data-original="true"]')).toHaveCount(0);
  expect(documentRequests).toBe(1);
});

test("an empty same-page fragment does not fetch or replace the page", async ({ page }) => {
  await page.goto("/");
  const navigationFetchHeaders: Promise<string | null>[] = [];
  page.on("request", (request) => {
    if (request.resourceType() === "fetch" && new URL(request.url()).pathname === "/") {
      navigationFetchHeaders.push(request.headerValue("x-okawak-navigation"));
    }
  });
  const main = page.locator("main");
  await main.evaluate((element) => element.setAttribute("data-same-page", "true"));

  const link = page.getByRole("link", { name: "Sanitized unsafe link" });
  await expect(link).toHaveAttribute("href", "#");
  await link.click();

  await expect(main).toHaveAttribute("data-same-page", "true");
  expect(await Promise.all(navigationFetchHeaders)).not.toContain("1");
});

test("same-page fragment history restores scroll without fetching", async ({ page }) => {
  await page.goto("/");
  const link = page.getByRole("link", { name: "Home fragment section" });
  const originalScrollY = await link.evaluate((element: HTMLAnchorElement) => {
    window.scrollTo(0, 500);
    element.click();
    return 500;
  });

  await expect(page).toHaveURL(/\/#home-fragment$/);
  const heading = page.getByRole("heading", { name: "Home fragment" });
  await expect(heading).toBeInViewport();

  await page.goBack();
  await expect(page).toHaveURL(/\/$/);
  await expect
    .poll(async () => Math.abs((await page.evaluate(() => window.scrollY)) - originalScrollY))
    .toBeLessThanOrEqual(10);

  await page.goForward();
  await expect(page).toHaveURL(/\/#home-fragment$/);
  await expect(heading).toBeInViewport();
});

test("initial load restores a saved manual scroll position", async ({ page }) => {
  await page.goto("/");
  const savedScrollY = 500;
  await page.evaluate((y) => {
    const state =
      window.history.state && typeof window.history.state === "object"
        ? window.history.state
        : {};
    window.history.replaceState(
      { ...state, okawakScrollPosition: { x: 0, y } },
      "",
      window.location.href,
    );
  }, savedScrollY);

  await page.reload();

  await expect
    .poll(async () => Math.abs((await page.evaluate(() => window.scrollY)) - savedScrollY))
    .toBeLessThanOrEqual(10);
});

test("forward navigation aborts a pending back navigation", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("link", { name: "E2E Article" }).click();
  await expect(page).toHaveURL(/\/tech\/e2e-article$/);

  let releaseHomeResponse: () => void = () => {};
  let markHomeFetchStarted: () => void = () => {};
  const homeResponseGate = new Promise<void>((resolve) => {
    releaseHomeResponse = resolve;
  });
  const homeFetchStarted = new Promise<void>((resolve) => {
    markHomeFetchStarted = resolve;
  });
  await page.route("**/", async (route) => {
    if (route.request().headers()["x-okawak-navigation"] === "1") {
      markHomeFetchStarted();
      await homeResponseGate;
    }
    await route.continue().catch(() => {});
  });

  await page.goBack();
  await homeFetchStarted;
  await page.goForward();
  releaseHomeResponse();

  await expect(page).toHaveURL(/\/tech\/e2e-article$/);
  await page.waitForTimeout(200);
  await expect(page.getByRole("heading", { name: "E2E Article" })).toBeVisible();
  await expect(page.getByText("Fixture home content")).toHaveCount(0);
});

test("pending popstate keeps the destination scroll state", async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => window.scrollTo(0, 400));
  await expect
    .poll(() =>
      page.evaluate(
        () => history.state?.okawakScrollPosition?.y as number | undefined,
      ),
    )
    .toBeGreaterThan(300);
  await page.getByRole("link", { name: "E2E Article" }).click();
  await expect(page).toHaveURL(/\/tech\/e2e-article$/);
  await page.getByRole("main").evaluate((main) => {
    const spacer = document.createElement("div");
    spacer.style.height = "120rem";
    main.append(spacer);
  });

  let releaseHomeResponse: () => void = () => {};
  let markHomeFetchStarted: () => void = () => {};
  const homeResponseGate = new Promise<void>((resolve) => {
    releaseHomeResponse = resolve;
  });
  const homeFetchStarted = new Promise<void>((resolve) => {
    markHomeFetchStarted = resolve;
  });
  await page.route("**/", async (route) => {
    if (route.request().headers()["x-okawak-navigation"] === "1") {
      markHomeFetchStarted();
      await homeResponseGate;
    }
    await route.continue().catch(() => {});
  });

  await page.goBack();
  await homeFetchStarted;
  const destinationStateBefore = await page.evaluate(
    () => history.state?.okawakScrollPosition,
  );
  await page.evaluate(async () => {
    window.scrollTo(0, 700);
    await new Promise<void>((resolve) =>
      requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
    );
  });
  const destinationStateAfter = await page.evaluate(
    () => history.state?.okawakScrollPosition,
  );

  expect(destinationStateAfter).toEqual(destinationStateBefore);
  await page.goForward();
  releaseHomeResponse();
  await expect(page).toHaveURL(/\/tech\/e2e-article$/);
});

test("same-page fragment aborts a pending client navigation", async ({ page }) => {
  await page.goto("/");

  let releaseArticleResponse: () => void = () => {};
  let markArticleFetchStarted: () => void = () => {};
  const articleResponseGate = new Promise<void>((resolve) => {
    releaseArticleResponse = resolve;
  });
  const articleFetchStarted = new Promise<void>((resolve) => {
    markArticleFetchStarted = resolve;
  });
  await page.route("**/tech/e2e-article", async (route) => {
    if (route.request().headers()["x-okawak-navigation"] === "1") {
      markArticleFetchStarted();
      await articleResponseGate;
    }
    await route.continue().catch(() => {});
  });

  await page.getByRole("link", { name: "E2E Article" }).click();
  await articleFetchStarted;
  const fragmentLink = page.getByRole("link", { name: "Home fragment section" });
  await fragmentLink.evaluate((element) =>
    element.setAttribute("href", "/#home-fragment"),
  );
  await fragmentLink.click();
  releaseArticleResponse();

  await expect(page).toHaveURL(/\/#home-fragment$/);
  await page.waitForTimeout(200);
  await expect(page.getByRole("heading", { name: "Home fragment" })).toBeInViewport();
  await expect(page.getByText("Fixture home content")).toBeVisible();
  await expect(page.getByText("Article fixture body")).toHaveCount(0);
});

test("modified fragment clicks do not abort a pending navigation", async ({ page }) => {
  await page.goto("/");

  let releaseArticleResponse: () => void = () => {};
  let markArticleFetchStarted: () => void = () => {};
  const articleResponseGate = new Promise<void>((resolve) => {
    releaseArticleResponse = resolve;
  });
  const articleFetchStarted = new Promise<void>((resolve) => {
    markArticleFetchStarted = resolve;
  });
  await page.route("**/tech/e2e-article", async (route) => {
    if (route.request().headers()["x-okawak-navigation"] === "1") {
      markArticleFetchStarted();
      await articleResponseGate;
    }
    await route.continue().catch(() => {});
  });

  await page.getByRole("link", { name: "E2E Article" }).click();
  await articleFetchStarted;
  await page.evaluate(() => {
    document.addEventListener(
      "click",
      (event) => {
        if ((event as MouseEvent).metaKey) event.preventDefault();
      },
      { once: true },
    );
  });
  await page
    .getByRole("link", { name: "Home fragment section" })
    .dispatchEvent("click", { button: 0, metaKey: true });
  releaseArticleResponse();

  await expect(page).toHaveURL(/\/tech\/e2e-article$/);
  await expect(page.getByText("Article fixture body")).toBeVisible();
});

test("only the active navigation clears its busy state", async ({ page }) => {
  await page.goto("/");

  let releaseArticleResponse: () => void = () => {};
  let markArticleFetchStarted: () => void = () => {};
  let releaseAboutResponse: () => void = () => {};
  let markAboutFetchStarted: () => void = () => {};
  const articleResponseGate = new Promise<void>((resolve) => {
    releaseArticleResponse = resolve;
  });
  const articleFetchStarted = new Promise<void>((resolve) => {
    markArticleFetchStarted = resolve;
  });
  const aboutResponseGate = new Promise<void>((resolve) => {
    releaseAboutResponse = resolve;
  });
  const aboutFetchStarted = new Promise<void>((resolve) => {
    markAboutFetchStarted = resolve;
  });
  await page.route("**/tech/e2e-article", async (route) => {
    if (route.request().headers()["x-okawak-navigation"] === "1") {
      markArticleFetchStarted();
      await articleResponseGate;
    }
    await route.continue().catch(() => {});
  });
  await page.route("**/about", async (route) => {
    if (route.request().headers()["x-okawak-navigation"] === "1") {
      markAboutFetchStarted();
      await aboutResponseGate;
    }
    await route.continue().catch(() => {});
  });

  await page.getByRole("link", { name: "E2E Article" }).click();
  await articleFetchStarted;
  await page.getByRole("link", { name: "About", exact: true }).click();
  await aboutFetchStarted;
  releaseArticleResponse();

  await page.waitForTimeout(200);
  await expect(page.getByRole("main")).toHaveAttribute("aria-busy", "true");

  releaseAboutResponse();
  await expect(page).toHaveURL(/\/about$/);
  await expect(page.getByRole("heading", { name: "Fixture About" })).toBeVisible();
  await expect(page.getByRole("main")).not.toHaveAttribute("aria-busy", "true");
});

test("fragment navigation resolves a pending popstate document", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("link", { name: "E2E Article" }).click();
  await expect(page).toHaveURL(/\/tech\/e2e-article$/);

  let releaseHomeResponse: () => void = () => {};
  let markHomeFetchStarted: () => void = () => {};
  const homeResponseGate = new Promise<void>((resolve) => {
    releaseHomeResponse = resolve;
  });
  const homeFetchStarted = new Promise<void>((resolve) => {
    markHomeFetchStarted = resolve;
  });
  await page.route("**/", async (route) => {
    if (route.request().headers()["x-okawak-navigation"] === "1") {
      markHomeFetchStarted();
      await homeResponseGate;
    }
    await route.continue().catch(() => {});
  });
  let documentRequests = 0;
  page.on("request", (request) => {
    if (request.resourceType() === "document") documentRequests += 1;
  });

  await page.goBack();
  await homeFetchStarted;
  await page.getByRole("main").evaluate((main) => {
    const link = document.createElement("a");
    link.href = "#generated-content";
    link.textContent = "Article fragment";
    main.prepend(link);
  });
  const articleDocumentRequest = page.waitForRequest(
    (request) =>
      request.resourceType() === "document" &&
      new URL(request.url()).pathname === "/tech/e2e-article",
  );
  const fragmentClick = page.getByRole("link", { name: "Article fragment" }).click();
  await articleDocumentRequest;
  releaseHomeResponse();
  await fragmentClick;

  await expect(page).toHaveURL(/\/tech\/e2e-article#generated-content$/);
  await expect(page.getByText("Article fixture body")).toBeVisible();
  await expect(page.getByRole("heading", { name: "Generated content" })).toBeInViewport();
  expect(documentRequests).toBe(1);
});

test("text fragments resolve against the rendered page during popstate", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByRole("link", { name: "E2E Article" }).click();
  await expect(page).toHaveURL(/\/tech\/e2e-article$/);

  let releaseHomeResponse: () => void = () => {};
  let markHomeFetchStarted: () => void = () => {};
  const homeResponseGate = new Promise<void>((resolve) => {
    releaseHomeResponse = resolve;
  });
  const homeFetchStarted = new Promise<void>((resolve) => {
    markHomeFetchStarted = resolve;
  });
  await page.route("**/", async (route) => {
    if (route.request().headers()["x-okawak-navigation"] === "1") {
      markHomeFetchStarted();
      await homeResponseGate;
    }
    await route.continue().catch(() => {});
  });

  await page.goBack();
  await homeFetchStarted;
  await page.getByRole("main").evaluate((main) => {
    const link = document.createElement("a");
    link.href = "#:~:text=Article%20fixture%20body";
    link.textContent = "Article text fragment";
    main.prepend(link);
  });
  const articleDocumentRequest = page.waitForRequest(
    (request) =>
      request.resourceType() === "document" &&
      new URL(request.url()).pathname === "/tech/e2e-article",
  );
  const fragmentClick = page
    .getByRole("link", { name: "Article text fragment" })
    .click();
  await articleDocumentRequest;
  releaseHomeResponse();
  await fragmentClick;

  // Chromium removes the fragment directive from the document-visible URL.
  await expect(page).toHaveURL(/\/tech\/e2e-article$/);
  await expect(page.getByText("Article fixture body")).toBeVisible();
});

test("relative links resolve against the rendered page during popstate", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByRole("link", { name: "E2E Article" }).click();
  await expect(page).toHaveURL(/\/tech\/e2e-article$/);
  await page.getByRole("main").evaluate((main) => {
    const link = document.createElement("a");
    link.href = "next";
    link.textContent = "Relative next";
    main.prepend(link);
  });

  let releaseHomeResponse: () => void = () => {};
  let markHomeFetchStarted: () => void = () => {};
  const homeResponseGate = new Promise<void>((resolve) => {
    releaseHomeResponse = resolve;
  });
  const homeFetchStarted = new Promise<void>((resolve) => {
    markHomeFetchStarted = resolve;
  });
  await page.route("**/", async (route) => {
    if (route.request().headers()["x-okawak-navigation"] === "1") {
      markHomeFetchStarted();
      await homeResponseGate;
    }
    await route.continue().catch(() => {});
  });

  await page.goBack();
  await homeFetchStarted;
  const relativeRequest = page.waitForRequest(
    (request) => request.headers()["x-okawak-navigation"] === "1",
  );
  await page.getByRole("link", { name: "Relative next" }).click();
  const requestedPath = new URL((await relativeRequest).url()).pathname;
  releaseHomeResponse();

  expect(requestedPath).toBe("/tech/next");
  await expect(page).toHaveURL(/\/tech\/next$/);
});

test("relative fragment links resolve against the rendered page during popstate", async ({
  page,
}) => {
  await page.goto("/about");
  await page.getByRole("main").evaluate((main) => {
    const articleLink = document.createElement("a");
    articleLink.href = "/tech/e2e-article";
    articleLink.textContent = "Article page";
    main.prepend(articleLink);
  });
  await page.getByRole("link", { name: "Article page" }).click();
  await expect(page).toHaveURL(/\/tech\/e2e-article$/);
  await page.getByRole("main").evaluate((main) => {
    const link = document.createElement("a");
    link.href = "about#section";
    link.textContent = "Relative fragment";
    main.prepend(link);
  });

  let releaseAboutResponse: () => void = () => {};
  let markAboutFetchStarted: () => void = () => {};
  const aboutResponseGate = new Promise<void>((resolve) => {
    releaseAboutResponse = resolve;
  });
  const aboutFetchStarted = new Promise<void>((resolve) => {
    markAboutFetchStarted = resolve;
  });
  await page.route("**/about", async (route) => {
    if (route.request().headers()["x-okawak-navigation"] === "1") {
      markAboutFetchStarted();
      await aboutResponseGate;
    }
    await route.continue().catch(() => {});
  });

  await page.goBack();
  await aboutFetchStarted;
  const relativeRequest = page.waitForRequest(
    (request) =>
      request.headers()["x-okawak-navigation"] === "1" &&
      new URL(request.url()).pathname === "/tech/about",
  );
  await page.getByRole("link", { name: "Relative fragment" }).click();
  const requestedUrl = new URL((await relativeRequest).url());
  releaseAboutResponse();

  expect(requestedUrl.pathname).toBe("/tech/about");
  await expect(page).toHaveURL(/\/tech\/about#section$/);
});

test("client navigation reloads when shell asset fingerprints change", async ({ page }) => {
  await page.goto("/");
  let documentRequests = 0;
  page.on("request", (request) => {
    if (request.resourceType() === "document") documentRequests += 1;
  });
  await page.route("**/tech/e2e-article", async (route) => {
    if (route.request().headers()["x-okawak-navigation"] !== "1") {
      await route.continue();
      return;
    }

    const response = await route.fetch();
    const currentBody = await response.text();
    const body = currentBody.replace(
      /href="\/pkg\/[^"]+\.css"/,
      'href="/pkg/deployed-shell.css"',
    );
    expect(body).not.toBe(currentBody);
    await route.fulfill({ response, body });
  });

  await page.getByRole("link", { name: "E2E Article" }).click();

  await expect(page).toHaveURL(/\/tech\/e2e-article$/);
  await expect(page.getByRole("heading", { name: "E2E Article" })).toBeVisible();
  expect(documentRequests).toBe(1);
});

test("client navigation reloads when inline shell runtime changes", async ({ page }) => {
  await page.goto("/");
  let documentRequests = 0;
  page.on("request", (request) => {
    if (request.resourceType() === "document") documentRequests += 1;
  });
  await page.route("**/tech/e2e-article", async (route) => {
    if (route.request().headers()["x-okawak-navigation"] !== "1") {
      await route.continue();
      return;
    }

    const response = await route.fetch();
    const currentBody = await response.text();
    const body = currentBody.replace(
      "window.okawakScheduleCodeHighlight = function(root)",
      "window.okawakScheduleCodeHighlightV2 = function(root)",
    );
    expect(body).not.toBe(currentBody);
    await route.fulfill({ response, body });
  });

  await page.getByRole("link", { name: "E2E Article" }).click();

  await expect(page).toHaveURL(/\/tech\/e2e-article$/);
  await expect(page.getByRole("heading", { name: "E2E Article" })).toBeVisible();
  expect(documentRequests).toBe(1);
});

test("client navigation reloads when the shell version changes", async ({ page }) => {
  await page.goto("/");
  let documentRequests = 0;
  page.on("request", (request) => {
    if (request.resourceType() === "document") documentRequests += 1;
  });
  await page.route("**/tech/e2e-article", async (route) => {
    if (route.request().headers()["x-okawak-navigation"] !== "1") {
      await route.continue();
      return;
    }

    const response = await route.fetch();
    const currentBody = await response.text();
    const body = currentBody.replace(
      'name="okawak-shell-version" content="topcoat-1"',
      'name="okawak-shell-version" content="topcoat-2"',
    );
    expect(body).not.toBe(currentBody);
    await route.fulfill({ response, body });
  });

  await page.getByRole("link", { name: "E2E Article" }).click();

  await expect(page).toHaveURL(/\/tech\/e2e-article$/);
  await expect(page.getByRole("heading", { name: "E2E Article" })).toBeVisible();
  expect(documentRequests).toBe(1);
});

test("server-rendered pages remain navigable without JavaScript", async ({ browser }) => {
  const context = await browser.newContext({
    baseURL: BASE_URL,
    javaScriptEnabled: false,
  });
  const page = await context.newPage();

  try {
    const homeResponse = await page.goto("/");

    expect(homeResponse?.status()).toBe(200);
    await expect(page.getByText("Fixture home content")).toBeVisible();
    await expectMetadata(
      page,
      SITE_NAME,
      "",
      "1件の記事を1カテゴリで公開しています。",
    );

    await page.getByRole("link", { name: "E2E Article" }).click();

    await expect(page).toHaveURL(/\/tech\/e2e-article$/);
    await expect(page.getByRole("heading", { name: "E2E Article" })).toBeVisible();
    await expect(page.locator("main .content-prose")).toContainText("Article fixture body");
    await expectMetadata(
      page,
      `E2E Article | ${SITE_NAME}`,
      "/tech/e2e-article",
      "Article fixture description",
      "article",
    );
  } finally {
    await context.close();
  }
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

test("client popstate closes the retained mobile menu", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/");
  const menuButton = page.locator('button[aria-controls="site-header-nav"]');
  await menuButton.click();
  await page.getByRole("link", { name: "About", exact: true }).click();
  await expect(page).toHaveURL(/\/about$/);

  await menuButton.click();
  await expect(menuButton).toHaveAttribute("aria-expanded", "true");
  await page.goBack();

  await expect(page).toHaveURL(/\/$/);
  await expect(page.getByText("Fixture home content")).toBeVisible();
  await expect(menuButton).toHaveAttribute("aria-expanded", "false");
  await expect(page.locator("#site-header-nav")).toHaveClass(/\bhidden\b/);
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
  await expectMetadata(
    page,
    `Fixture About | ${SITE_NAME}`,
    "/about",
    "About fixture description",
  );
});

test("category renders landing content and grouped articles", async ({ page }) => {
  const browserErrors = captureBrowserErrors(page);
  const response = await page.goto("/tech");

  expect(response?.status()).toBe(200);
  await expect(page.getByText("Fixture Tech", { exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Tech landing" })).toBeVisible();
  await expect(page.locator("main .content-prose")).toContainText("Category fixture body");
  await expect(page.getByRole("heading", { name: "rust / async" })).toBeVisible();
  await expect(page.getByRole("link", { name: "E2E Article" })).toBeVisible();
  await expectFormattedFixtureDates(page);
  await expectMetadata(
    page,
    `Fixture Tech | ${SITE_NAME}`,
    "/tech",
    "Category fixture description",
  );
  expect(browserErrors).toEqual([]);
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
