import { expect, type Page, test } from "@playwright/test";
import { CONTRASENA, USUARIO, instalado, instalados } from "./instancias";

/**
 * Espera a que la página esté hidratada (`data-hidratado` en `<body>`, que
 * pone el wasm al terminar). Antes, un clic o un envío lo atendería el HTML
 * del servidor: el formulario de entrada, por ejemplo, se enviaría sin
 * JavaScript y la página se quedaría en /login.
 */
export async function hidratada(page: Page) {
  await page.locator("body[data-hidratado]").waitFor({ state: "attached", timeout: 15_000 });
}

/**
 * Cada worker usa su panel: el del índice que le dio Playwright
 * (instancias.ts) y, en la página, el puerto de ese panel (`baseURL` de
 * playwright.config.ts, que se evalúa otra vez en cada worker). Si un día se
 * evaluase una sola vez, todos irían al primero y casi todo seguiría en
 * verde.
 */
export function suPanel(page: Page) {
  const { info } = test;
  const indice = info().parallelIndex;
  expect(instalado, `el worker ${indice} usa instalados[${indice}]`).toBe(instalados[indice]);
  expect(new URL(page.url()).port, `el worker ${indice} entra en su panel`).toBe(
    String(instalado.puerto),
  );
}

/** Entra en el panel instalado (formulario en español). */
export async function entrar(page: Page) {
  await page.goto("/login");
  suPanel(page);
  await hidratada(page);
  await page.waitForLoadState("networkidle");
  await page.locator("#usuario").fill(USUARIO);
  await page.locator("#contrasena").fill(CONTRASENA);
  await page.locator("form button[type=submit]").click();
  await page.waitForURL((url) => url.pathname === "/");
}

/**
 * Errores y avisos de la consola y del JavaScript de la página (la
 * hidratación avisa por la consola si el servidor y el navegador no pintan
 * lo mismo). Se miran al final con `expect(errores).toEqual([])`.
 */
export function recogerErrores(page: Page): string[] {
  const errores: string[] = [];
  page.on("console", (m) => {
    // El panel no tiene favicon: el 404 que pide Chromium no es de la página.
    if (m.location().url.endsWith("/favicon.ico")) return;
    if (m.type() === "error" || m.type() === "warning") errores.push(m.text());
  });
  page.on("pageerror", (e) => errores.push(String(e)));
  return errores;
}

/** Elementos que sobresalen por la derecha (para el mensaje de error). */
async function desbordados(page: Page): Promise<string[]> {
  return page.evaluate(() => {
    const ancho = document.documentElement.clientWidth;
    const fuera: string[] = [];
    for (const el of document.body.querySelectorAll("*")) {
      const r = el.getBoundingClientRect();
      if (r.width > 0 && r.right > ancho + 0.5) {
        const id = el.id ? `#${el.id}` : "";
        const clase = el.className && typeof el.className === "string"
          ? `.${el.className.trim().split(/\s+/).join(".")}`
          : "";
        fuera.push(`${el.tagName.toLowerCase()}${id}${clase} → ${Math.round(r.right)}px`);
      }
    }
    return fuera.slice(0, 15);
  });
}

/** Sin desbordamiento horizontal (scrollWidth ≤ ancho de la ventana). */
export async function sinDesbordamiento(page: Page, donde: string) {
  const { scroll, inner } = await page.evaluate(() => ({
    scroll: document.documentElement.scrollWidth,
    inner: window.innerWidth,
  }));
  expect(
    scroll,
    `${donde}: scrollWidth ${scroll} > innerWidth ${inner}; sobresalen: ${(await desbordados(page)).join(", ")}`,
  ).toBeLessThanOrEqual(inner);
}

/** Hidratación y primeras cargas (las páginas sondean: la red nunca queda del todo quieta). */
export async function asentar(page: Page) {
  await hidratada(page);
  await page.waitForLoadState("networkidle", { timeout: 5_000 }).catch(() => {});
}
