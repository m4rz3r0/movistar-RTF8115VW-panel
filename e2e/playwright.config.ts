import { defineConfig } from "@playwright/test";
import { instalado, todas } from "./instancias";

// Las capturas del README (capturas-readme.spec.ts) sobrescriben
// docs/capturas/, que está versionado: solo se generan cuando se piden por
// su nombre (`npx playwright test capturas-readme`) o con
// E2E_CAPTURAS_README=1, nunca en la pasada completa. La variable llega
// también a los procesos de las pruebas.
if (process.argv.some((a) => a.includes("capturas-readme"))) {
  process.env.E2E_CAPTURAS_README = "1";
}

export default defineConfig({
  testDir: ".",
  testIgnore: process.env.E2E_CAPTURAS_README ? [] : ["**/capturas-readme.spec.ts"],
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
