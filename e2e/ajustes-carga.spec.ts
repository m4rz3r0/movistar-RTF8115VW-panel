import { expect, type Page, test } from "@playwright/test";
import { instalado } from "./instancias";
import { entrar, recogerErrores } from "./util";

// Cambio de tema en Ajustes con un router lento (300 ms por respuesta en el
// simulador): mientras se lee el tema nuevo, lo anterior queda atenuado con
// aria-busy y «Actualizando…»; al volver a un tema leído hace poco (la caché
// del servidor), ni lo uno ni lo otro.

async function retardo(ms: number) {
  const r = await fetch(`${instalado.urlSim}/__sim/retardo`, { method: "POST", body: String(ms) });
  expect(r.status).toBe(204);
}

/** Pulsa la pestaña de `tema` y espera a ver su contenido, ya sin aria-busy. */
async function cambiar(page: Page, tema: string) {
  const antes = await page.locator(".tema-desc").textContent();
  await page.locator(`nav.temas a[href="/ajustes/${tema}"]`).click();
  await expect(page.locator(".tema-desc")).not.toHaveText(antes ?? "");
  await expect(page.locator(".tema-contenido")).not.toHaveAttribute("aria-busy", /.*/);
  await expect(page.locator(".tema-actualizando")).toHaveText("");
}

/** Desde ahora, ¿se ha visto aria-busy o «Actualizando…»? (`window.__ocupado`) */
async function vigilarOcupado(page: Page) {
  await page.evaluate(() => {
    const w = window as unknown as { __ocupado: boolean; __vigia?: MutationObserver };
    w.__ocupado = false;
    w.__vigia?.disconnect();
    const mirar = () => {
      if (
        document.querySelector(".tema-contenido[aria-busy=true]") ||
        document.querySelector(".tema-actualizando")?.textContent
      ) {
        w.__ocupado = true;
      }
    };
    w.__vigia = new MutationObserver(mirar);
    w.__vigia.observe(document.body, {
      subtree: true,
      childList: true,
      attributes: true,
      characterData: true,
    });
  });
}

const ocupado = (page: Page) =>
  page.evaluate(() => (window as unknown as { __ocupado: boolean }).__ocupado);

test.describe("ajustes: carga al cambiar de tema", () => {
  test.afterEach(async () => {
    await retardo(0);
  });

  test("«Actualizando…» mientras lee y no al volver a un tema", async ({ page }) => {
    const errores = recogerErrores(page);
    await entrar(page);
    await page.goto("/ajustes/internet");
    await page.waitForLoadState("networkidle");
    await retardo(300);

    // Sistema: dos lecturas lentas. Lo anterior, ocupado mientras tanto.
    const antes = await page.locator(".tema-desc").textContent();
    await page.locator('nav.temas a[href="/ajustes/sistema"]').click();
    const contenido = page.locator(".tema-contenido");
    await expect(contenido).toHaveAttribute("aria-busy", "true");
    await expect(page.locator(".tema-actualizando")).toHaveText("Actualizando…");
    await expect(page.locator(".tema-actualizando")).toHaveAttribute("role", "status");
    // El aviso, fuera de la zona ocupada (aria-busy aplaza sus avisos), y lo
    // anterior, inerte (ni ratón ni teclado).
    await expect(page.locator("[aria-busy=true] [role=status]")).toHaveCount(0);
    await expect(contenido).toHaveAttribute("inert", "");
    // Mientras, aún se ve el tema anterior.
    await expect(page.locator(".tema-desc")).toHaveText(antes ?? "");
    await expect(page.locator(".tema-desc")).not.toHaveText(antes ?? "", { timeout: 10_000 });
    await expect(contenido).not.toHaveAttribute("aria-busy", /.*/);
    await expect(contenido).not.toHaveAttribute("inert", /.*/);
    await expect(page.locator(".tema-actualizando")).toHaveText("");

    // Volver a Internet (leído al abrir la página) y otra vez a Sistema: de
    // la caché, sin «Actualizando…» ni aria-busy en ningún momento.
    await vigilarOcupado(page);
    await cambiar(page, "internet");
    await cambiar(page, "sistema");
    expect(await ocupado(page)).toBe(false);
    expect(errores).toEqual([]);
  });

  test("una vez a la vista, «Actualizando…» no parpadea", async ({ page }) => {
    const errores = recogerErrores(page);
    await entrar(page);
    await page.goto("/ajustes/internet");
    await page.waitForLoadState("networkidle");
    // Seguridad (dos lecturas, sin leer aún) tarda unos 350 ms: un poco más
    // que el umbral.
    await retardo(100);
    await page.evaluate(() => {
      const w = window as unknown as { __visto?: number; __oculto?: number };
      const aviso = document.querySelector(".tema-actualizando")!;
      new MutationObserver(() => {
        const t = performance.now();
        if (aviso.textContent && w.__visto === undefined) w.__visto = t;
        if (!aviso.textContent && w.__visto !== undefined && w.__oculto === undefined) {
          w.__oculto = t;
        }
      }).observe(aviso, { childList: true, subtree: true, characterData: true });
    });
    await cambiar(page, "seguridad");
    const { visto, oculto } = await page.evaluate(() => {
      const w = window as unknown as { __visto?: number; __oculto?: number };
      return { visto: w.__visto, oculto: w.__oculto };
    });
    expect(visto, "no llegó a verse: la prueba no prueba nada").toBeDefined();
    expect(oculto! - visto!).toBeGreaterThanOrEqual(280);
    expect(errores).toEqual([]);
  });

  test("salir de la página antes del umbral no rompe nada", async ({ page }) => {
    const errores = recogerErrores(page);
    await entrar(page);
    await page.goto("/ajustes/internet");
    await page.waitForLoadState("networkidle");
    await retardo(300);
    // Pulsar un tema lento y, enseguida, otra sección del menú…
    await page.locator('nav.temas a[href="/ajustes/sistema"]').click();
    await page.locator('nav.nav a[href="/"]').click();
    await page.waitForURL((u) => u.pathname === "/");
    await page.waitForTimeout(600);
    // …y con el foco en una pestaña (la precarga programada), volver atrás.
    await page.goto("/ajustes/internet");
    await page.waitForLoadState("networkidle");
    await page.locator('nav.temas a[href="/ajustes/puertos"]').focus();
    await page.locator('nav.nav a[href="/"]').click();
    await page.waitForURL((u) => u.pathname === "/");
    await page.waitForTimeout(600);
    // La aplicación sigue viva: el menú navega en el navegador.
    await page.locator('nav.nav a[href^="/ajustes"]').click();
    await expect(page.locator("nav.temas")).toBeVisible();
    expect(errores).toEqual([]);
  });

  test("la precarga de Red local trae también sus dispositivos", async ({ page }) => {
    const errores = recogerErrores(page);
    await entrar(page);
    await page.goto("/ajustes/internet");
    await page.waitForLoadState("networkidle");
    await retardo(300);
    const pestana = page.locator('nav.temas a[href="/ajustes/red_local"]');
    const red = page.waitForResponse((r) => r.url().endsWith("/api/leer_red_casa"), {
      timeout: 15_000,
    });
    await pestana.hover();
    await red;
    await pestana.click();
    // De la caché: sin esperar al router (cada lectura tarda 300 ms).
    await expect(page.locator("li.dispositivo").first()).toBeVisible({ timeout: 500 });
    expect(errores).toEqual([]);
  });
});
