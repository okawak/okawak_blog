import { expect, test } from "@playwright/test";

test("native popover exposes its expanded state to assistive technology", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/tech");
  const session = await page.context().newCDPSession(page);
  try {
    for (const [step, expanded] of [false, true, false].entries()) {
      await expect.poll(async () => {
        const { nodes } = await session.send("Accessibility.getFullAXTree");
        const button = nodes.find((node) => node.role?.value === "button" &&
          node.name?.value === (expanded ? "ナビゲーションメニューを閉じる" : "ナビゲーションメニューを開く"));
        return button?.properties?.find((property) => property.name === "expanded")?.value.value;
      }).toBe(expanded);
      if (step === 0) {
        await page.getByRole("button", { name: "ナビゲーションメニューを開く" }).click();
      } else if (step === 1) {
        await page.keyboard.press("Escape");
      }
    }
  } finally {
    await session.detach();
  }
});

test("mobile menu dismisses on Escape and outside click", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/tech");
  const trigger = page.getByRole("button", { name: "ナビゲーションメニューを開く" });
  const navigation = page.getByRole("navigation", { name: "メインナビゲーション" });
  await trigger.click();
  await expect(navigation).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(navigation).toBeHidden();
  await expect(trigger).toBeFocused();
  await trigger.click();
  // The panel can cover the heading; the page's left gutter is always outside it.
  await page.mouse.click(5, 400);
  await expect(navigation).toBeHidden();
  await expect(trigger).toBeVisible();
});

test("mobile popover closes across desktop and mobile layouts", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/tech");
  await page.getByRole("button", { name: "ナビゲーションメニューを開く" }).click();
  await page.setViewportSize({ width: 1280, height: 844 });
  await expect(page.getByRole("navigation", { name: "メインナビゲーション" })).toBeVisible();
  await expect(page.locator("[popover]:popover-open")).toHaveCount(0);
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(page.getByRole("navigation", { name: "メインナビゲーション" })).toBeHidden();
  await expect(page.getByRole("button", { name: "ナビゲーションメニューを開く" })).toBeVisible();
});

test("mobile navigation works without JavaScript in both languages", async ({ browser, baseURL }, testInfo) => {
  const context = await browser.newContext({
    baseURL,
    javaScriptEnabled: false,
    viewport: { width: 320, height: 700 },
  });
  try {
    const page = await context.newPage();
    for (const [prefix, open, close] of [
      ["", "ナビゲーションメニューを開く", "ナビゲーションメニューを閉じる"],
      ["/en", "Open navigation menu", "Close navigation menu"],
    ]) {
      await page.goto(`${prefix}/tech`);
      await page.getByRole("button", { name: open }).click();
      await expect(page.getByRole("button", { name: close })).toBeVisible();
      const panel = page.locator("[popover]:popover-open");
      await expect(panel).toHaveCSS("opacity", "1");
      const bounds = await panel.boundingBox();
      expect(bounds).not.toBeNull();
      expect(bounds!.x).toBeGreaterThanOrEqual(0);
      expect(bounds!.x + bounds!.width).toBeLessThanOrEqual(320);
      await page.screenshot({
        path: testInfo.outputPath(`menu-${prefix ? "en" : "ja"}.png`),
        animations: "disabled",
      });
      await page.getByRole("link", { name: "About", exact: true }).click();
      await expect(page).toHaveURL(`${baseURL}${prefix}/about`);
      await expect(page.getByRole("button", { name: open })).toBeVisible();
    }
  } finally {
    await context.close();
  }
});
