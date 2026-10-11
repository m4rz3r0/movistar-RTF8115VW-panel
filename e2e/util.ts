import { expect, type Page } from "@playwright/test";
import { CONTRASENA, USUARIO } from "./instancias";

/** Entra en el panel instalado (formulario en español). */
export async function entrar(page: Page) {
  await page.goto("/login");
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
  await page.waitForLoadState("networkidle", { timeout: 5_000 }).catch(() => {});
  await page.waitForTimeout(1_000);
}
