import AxeBuilder from "@axe-core/playwright";
import { expect, type Page, test } from "@playwright/test";
import { CONTRASENA, USUARIO, instalado, instalados } from "./instancias";

/**
 * Espera a que la raíz de la página esté hidratada (`data-hidratado` en
 * `<body>`, que pone el wasm al terminar `hydrate_body`). Lo que va dentro de
 * un `Suspense` o un `Transition` se hidrata después, al llegar sus datos:
 * para eso, `asentar` espera además a la red. Antes de la marca, un clic o un
 * envío lo atendería el HTML del servidor (el formulario de entrada se
 * enviaría sin JavaScript).
 */
export async function hidratada(page: Page) {
  await expect(
    page.locator("body[data-hidratado]"),
    "la página no se hidrató: ¿falló el wasm? (mira la consola)",
  ).toBeAttached({ timeout: 15_000 });
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

/** Violaciones serias o críticas de axe en la página actual. */
export async function violacionesGraves(page: Page) {
  // A medio fundido, los colores aún no son los suyos.
  await quieta(page);
  const { violations, passes } = await new AxeBuilder({ page }).analyze();
  // Que axe ha mirado la página de verdad (entre otras, el contraste).
  expect(passes.map((p) => p.id)).toContain("color-contrast");
  return violations
    .filter((v) => v.impact === "serious" || v.impact === "critical")
    .map((v) => `${v.id} (${v.impact}): ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`);
}

/**
 * Espera a que acaben las animaciones y transiciones de la página (las que
 * no acaban nunca, no): para mirar los colores o hacer una captura.
 */
export async function quieta(page: Page) {
  await page.evaluate(() =>
    Promise.all(
      document
        .getAnimations()
        .filter((a) => a.effect?.getComputedTiming().iterations !== Infinity)
        .map((a) => a.finished.catch(() => undefined)),
    ),
  );
}

/**
 * Captura de pantalla solo con `E2E_CAPTURAS=1`: en la batería normal no se
 * mira ninguna (ninguna se compara) y las de página entera a 1440 px son
 * caras. `objetivo` es una página o un localizador.
 */
export async function capturaOpcional(
  objetivo: { screenshot(opciones: object): Promise<unknown> },
  opciones: object,
): Promise<void> {
  if (process.env.E2E_CAPTURAS) await objetivo.screenshot(opciones);
}
