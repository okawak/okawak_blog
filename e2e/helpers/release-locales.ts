import { expect, type Page } from "@playwright/test";

export type LocaleCatalog = {
  schema_version: number;
  routes: Record<string, Array<"ja" | "en">>;
};

// Sample each published language's home, optional About, and one category/article.
// Local validation checks every declared artifact before upload.
export async function verifyReleaseLocales(page: Page, catalog: LocaleCatalog, baseURL: string) {
  expect(catalog.schema_version).toBe(1);
  expect(catalog.routes["/"]).toContain("ja");
  for (const locale of catalog.routes["/"]) {
    expect(["ja", "en"]).toContain(locale);
    const paths = Object.keys(catalog.routes).filter(path => catalog.routes[path].includes(locale));
    const category = paths.find(path => /^\/[^/]+$/.test(path) && path !== "/about");
    const article = paths.find(path => /^\/[^/]+\/[^/]+$/.test(path));
    const sample = ["/", paths.includes("/about") ? "/about" : undefined, category, article].filter((path): path is string => Boolean(path));
    for (const japanesePath of sample) {
      const route = locale === "en" ? (japanesePath === "/" ? "/en" : `/en${japanesePath}`) : japanesePath;
      const response = await page.goto(route === "/" ? "/?lang=ja" : route);
      expect(response?.status(), `${locale}: ${route}`).toBe(200);
      await expect(page.locator("html")).toHaveAttribute("lang", locale);
      const canonical = `${baseURL}${route === "/" ? "" : route}`;
      await expect(page.locator('link[rel="canonical"]')).toHaveAttribute("href", canonical);
      await expect(page.locator('meta[property="og:url"]')).toHaveAttribute("content", canonical);
      await expect(page.locator("main")).toBeVisible();
      for (const other of catalog.routes[japanesePath]) {
        await expect(page.locator(`link[rel="alternate"][hreflang="${other}"]`)).toHaveCount(1);
      }
    }
  }
}
