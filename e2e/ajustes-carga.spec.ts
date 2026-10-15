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
    // Mientras, aún se ve el tema anterior.
    await expect(page.locator(".tema-desc")).toHaveText(antes ?? "");
    await expect(page.locator(".tema-desc")).not.toHaveText(antes ?? "", { timeout: 10_000 });
    await expect(contenido).not.toHaveAttribute("aria-busy", /.*/);
    await expect(page.locator(".tema-actualizando")).toHaveText("");

    // Volver a Internet (leído al abrir la página) y otra vez a Sistema: de
    // la caché, sin «Actualizando…» ni aria-busy en ningún momento.
    await vigilarOcupado(page);
    await cambiar(page, "internet");
    await cambiar(page, "sistema");
    expect(await ocupado(page)).toBe(false);
    expect(errores).toEqual([]);
  });
});
