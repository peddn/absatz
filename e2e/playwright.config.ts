import { defineConfig, devices } from "@playwright/test";

// Serves the release build (just build) under /absatz/, like GitHub Pages.
export default defineConfig({
  testDir: "tests",
  timeout: 60_000,
  reporter: [["list"]],
  use: {
    baseURL: "http://localhost:4173/absatz/",
    trace: "retain-on-failure",
  },
  projects: [
    {
      name: "chromium",
      use: {
        ...devices["Desktop Chrome"],
        // Use a locally installed Chrome when given (dev container), else Playwright's.
        launchOptions: process.env.CHROME_PATH ? { executablePath: process.env.CHROME_PATH } : {},
      },
    },
  ],
  webServer: {
    command:
      "mkdir -p .site && ln -sfn ../../crates/absatz-web/dist .site/absatz && python3 -m http.server 4173 --directory .site",
    url: "http://localhost:4173/absatz/",
    reuseExistingServer: !process.env.CI,
    stderr: "ignore",
  },
});
