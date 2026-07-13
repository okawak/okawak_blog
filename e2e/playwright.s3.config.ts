import path from "node:path";
import { devices, defineConfig } from "@playwright/test";

const baseURL = "http://127.0.0.1:8008";
const repoRoot = path.resolve(__dirname, "..");
const bucket = process.env.OKAWAK_BLOG_ARTIFACT_BUCKET;

if (!bucket) {
  throw new Error(
    "OKAWAK_BLOG_ARTIFACT_BUCKET is required for the S3 browser smoke test.",
  );
}

export default defineConfig({
  testDir: "./tests/s3",
  outputDir: "test-results/s3",
  timeout: 30_000,
  expect: {
    timeout: 5_000,
  },
  fullyParallel: false,
  forbidOnly: Boolean(process.env.CI),
  retries: 0,
  workers: 1,
  reporter: "list",
  use: {
    baseURL,
    trace: "retain-on-failure",
  },
  projects: [
    {
      name: "chromium-s3",
      use: devices["Desktop Chrome"],
    },
  ],
  webServer: {
    command: "sh e2e/run-server.sh",
    cwd: repoRoot,
    env: {
      ...process.env,
      OKAWAK_BLOG_ARTIFACT_SOURCE: "s3",
      OKAWAK_BLOG_ARTIFACT_BUCKET: bucket,
      OKAWAK_BLOG_SITE_ORIGIN: baseURL,
    },
    url: `${baseURL}/api/health`,
    reuseExistingServer: false,
    timeout: 180_000,
  },
});
