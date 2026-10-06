import { expect, test } from "@playwright/test";

test("intent prefetch is reused without a document reload", async ({ page }) => {
  await page.goto("/tech");
  let renders = 0;
  let documents = 0;
  page.on("request", request => {
    if (new URL(request.url()).pathname !== "/tech/e2e-article") return;
    if (request.method() === "POST") renders += 1;
    if (request.resourceType() === "document") documents += 1;
  });
  const article = page.getByRole("link", { name: "E2E Article", exact: true });
  const prefetched = page.waitForResponse(response =>
    response.request().method() === "POST" &&
    new URL(response.url()).pathname === "/tech/e2e-article",
  );
  await article.hover();
  expect((await prefetched).ok()).toBe(true);
  await article.click();
  await expect(page.getByRole("heading", { name: "E2E Article", exact: true })).toBeVisible();
  expect(renders).toBe(1);
  expect(documents).toBe(0);
});

test("language choices never prefetch or change cookies before a click", async ({ page, context }) => {
  await page.goto("/tech/e2e-article");
  const english = page.locator("header").getByRole("link", { name: "English", exact: true });
  const home = page.locator("#site-header-nav").getByRole("link", { name: "ホーム", exact: true });
  const choices: string[] = [];
  page.on("request", request => {
    if (new URL(request.url()).searchParams.has("lang")) choices.push(request.url());
  });
  for (const link of [english, home]) {
    await expect(link).toHaveAttribute("data-topcoat-link", "never");
    await link.hover();
    await link.focus();
  }
  // Cross the framework's 80 ms hover debounce to catch unwanted side effects.
  await page.waitForTimeout(200);
  expect(choices).toEqual([]);
  expect((await context.cookies()).find(cookie => cookie.name === "okawak_locale")).toBeUndefined();
  await english.click();
  await expect(page).toHaveURL(/\/en\/tech\/e2e-article$/);
  await expect(page.locator("html")).toHaveAttribute("lang", "en");
  expect((await context.cookies()).find(cookie => cookie.name === "okawak_locale")).toMatchObject({ value: "en", httpOnly: true });
});

test("article enhancements run after runtime navigation and again after history", async ({ page }) => {
  // Model already-loaded CDN APIs so this regression does not depend on a CDN.
  await page.route(/cdn\.jsdelivr\.net|cdnjs\.cloudflare\.com/, route => route.abort());
  await page.goto("/");
  await page.evaluate(() => {
    (window as any).__enhancementDocument = true;
    (window as any).katex = {
      render(expression: string, element: Element) {
        const rendered = document.createElement("span");
        rendered.className = "katex";
        rendered.textContent = expression;
        element.replaceChildren(rendered);
      },
    };
    (window as any).hljs = {
      highlightElement(element: HTMLElement) {
        element.dataset.highlighted = "yes";
        element.classList.add("hljs");
      },
    };
  });
  await page.getByRole("link", { name: "E2E Article", exact: true }).click();
  await expect(page.locator('[data-testid="article-katex"] .katex')).toBeVisible();
  await expect(page.locator(".content-prose pre code")).toHaveAttribute("data-highlighted", "yes");
  await page.goBack();
  await expect(page.getByText("Fixture home content")).toBeVisible();
  await page.goForward();
  await expect(page.locator('[data-testid="article-katex"] .katex')).toBeVisible();
  await expect(page.locator(".content-prose pre code")).toHaveAttribute("data-highlighted", "yes");
  expect(await page.evaluate(() => (window as any).__enhancementDocument)).toBe(true);
});

test("category search still rerenders its shard after runtime navigation", async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => { (window as any).__categoryDocument = true; });
  await page.locator('main a[href="/tech"]').click();
  await expect(page).toHaveURL(/\/tech$/);
  const query = page.getByRole("searchbox", { name: "記事を絞り込む" });
  await query.fill("missing article");
  await expect(page.getByRole("status")).toContainText("0");
  await query.fill("rust");
  await expect(page.getByRole("status")).toContainText("1");
  expect(await page.evaluate(() => (window as any).__categoryDocument)).toBe(true);
});
