import { defineConfig } from "@playwright/test";

// Puertos propios: no chocan con una vista previa en 3000/3100/8091.
const port = Number(process.env.E2E_PORT ?? 3195);

export default defineConfig({
  testDir: ".",
  outputDir: "test-results",
  fullyParallel: false,
  workers: 1,
  reporter: "list",
  use: {
    // localhost (no 127.0.0.1): Chromium acepta allí la cookie Secure
    // __Host-rp_session por http, y RP_ORIGIN coincide.
    baseURL: `http://localhost:${port}`,
    locale: "es-ES",
    timezoneId: "Europe/Madrid",
    colorScheme: "dark",
    launchOptions: process.env.E2E_CHROMIUM
      ? { executablePath: process.env.E2E_CHROMIUM }
      : undefined,
  },
  webServer: {
    command: "sh ./arrancar.sh",
    url: `http://localhost:${port}/login`,
    env: { E2E_PORT: String(port) },
    reuseExistingServer: false,
    timeout: 60_000,
    stdout: "ignore",
    stderr: "pipe",
  },
});
