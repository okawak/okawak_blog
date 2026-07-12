import path from "node:path";
import { devices, defineConfig } from "@playwright/test";

const baseURL = "http://127.0.0.1:8008";
const repoRoot = path.resolve(__dirname, "../../../..");
const fixtureRoot = path.resolve(__dirname, "fixtures/site");

export default defineConfig({
  testDir: "./tests",
  outputDir: "test-results",
  timeout: 30_000,
  expect: {
    timeout: 5_000,
  },
  fullyParallel: false,
  forbidOnly: Boolean(process.env.CI),
  retries: process.env.CI ? 1 : 0,
  workers: 1,
  reporter: "list",
  use: {
    baseURL,
    trace: "retain-on-failure",
  },
  projects: [
    {
      name: "chromium",
      use: devices["Desktop Chrome"],
    },
  ],
  webServer: {
    command: "sh crates/site/web/end2end/run-server.sh",
    cwd: repoRoot,
    env: {
      ...process.env,
      OKAWAK_BLOG_ARTIFACT_SOURCE: "local",
      OKAWAK_BLOG_ARTIFACT_LOCAL_ROOT: fixtureRoot,
      OKAWAK_BLOG_SITE_ORIGIN: baseURL,
    },
    url: `${baseURL}/api/health`,
    reuseExistingServer: false,
    timeout: 180_000,
  },
});
