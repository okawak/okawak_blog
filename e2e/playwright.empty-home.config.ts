import path from "node:path";
import { devices, defineConfig } from "@playwright/test";

const baseURL = "http://127.0.0.1:8008";
const repoRoot = path.resolve(__dirname, "..");
const fixtureRoot = path.resolve(__dirname, "fixtures/empty-site");

export default defineConfig({
  testDir: "./tests/empty-home",
  outputDir: "test-results/empty-home",
  timeout: 30_000,
  expect: {
    timeout: 5_000,
  },
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
      name: "chromium-empty-home",
      use: devices["Desktop Chrome"],
    },
  ],
  webServer: {
    command: "sh e2e/run-server.sh",
    cwd: repoRoot,
    env: {
      ...process.env,
      OKAWAK_BLOG_ARTIFACT_SOURCE: "local",
      OKAWAK_BLOG_ARTIFACT_LOCAL_ROOT: fixtureRoot,
      OKAWAK_BLOG_SITE_ORIGIN: baseURL,
    },
    url: `${baseURL}/api/health`,
    reuseExistingServer: false,
    timeout: 600_000,
  },
});
