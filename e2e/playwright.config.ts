import { defineConfig } from "@playwright/test";
import { instalado, todas } from "./instancias";

export default defineConfig({
  testDir: ".",
  outputDir: "test-results",
  fullyParallel: false,
  workers: 1,
  reporter: "list",
  use: {
    // Las páginas del panel instalado; el asistente usa el suyo.
    baseURL: instalado.url,
    locale: "es-ES",
    timezoneId: "Europe/Madrid",
    colorScheme: "dark",
    launchOptions: process.env.E2E_CHROMIUM
      ? { executablePath: process.env.E2E_CHROMIUM }
      : undefined,
  },
  webServer: todas.map((i) => ({
    command: "sh ./arrancar.sh",
    url: `${i.url}/salud`,
    env: {
      E2E_MODO: i.modo,
      E2E_PORT: String(i.puerto),
      E2E_SIM_PORT: String(i.puertoSim),
      E2E_DIR: i.dir,
    },
    reuseExistingServer: false,
    // SIGTERM (no SIGKILL): arrancar.sh borra su carpeta al salir.
    gracefulShutdown: { signal: "SIGTERM" as const, timeout: 5_000 },
    timeout: 60_000,
    stdout: "ignore" as const,
    stderr: "pipe" as const,
  })),
});
