import { expect, test } from "@playwright/test";
import { cambiarUpnp, esperarFinal, plazoDeTest } from "./cambios";
import { instalado } from "./instancias";
import { entrar, hidratada, recogerErrores } from "./util";

// La Biblioteca (una función de lectura con un parámetro) abierta justo
// después de aplicar un cambio, con el router lento: el servidor pinta la
// página mientras relee lo que el cambio dejó viejo. Tiene que hidratarse sin
// errores y seguir respondiendo («Leer de nuevo» lee). Antes, el resultado de
// la lectura llegaba en el HTML del servidor y, según lo que tardase el
// router, el navegador no lo encontraba al hidratar y el wasm moría
// (pánico de hidratación en explorador.rs).

async function retardo(ms: number) {
  const r = await fetch(`${instalado.urlSim}/__sim/retardo`, { method: "POST", body: String(ms) });
  expect(r.status).toBe(204);
}

test.describe("Biblioteca tras un cambio", () => {
  test.use({ viewport: { width: 1440, height: 900 } });
  test.afterEach(async () => {
    await retardo(0);
  });

  test("se hidrata y lee justo después de aplicar un cambio", async ({ page }) => {
    // El cambio y el que lo deshace.
    test.setTimeout(plazoDeTest(2));
    const errores = recogerErrores(page);
    await entrar(page);
    const id = await cambiarUpnp(page);
    try {
      await esperarFinal(page, id);
      await retardo(150);
      for (let i = 0; i < 3; i++) {
        await page.goto("/explorador/wifi/cbGetWifiAccPt");
        await hidratada(page);
        expect(errores, `errores al abrir la Biblioteca (${i + 1})`).toEqual([]);
      }
      // Sigue respondiendo: «Leer de nuevo» con el parámetro lee la red 1.
      await page.locator("#lectura-1").fill("1");
      await page.getByRole("button", { name: "Leer de nuevo" }).click();
      await expect(page.locator(".resultado-lectura input.secreto-valor").first()).not.toHaveValue("", {
        timeout: 15_000,
      });
    } finally {
      // Lo deja como estaba.
      await retardo(0);
      await esperarFinal(page, await cambiarUpnp(page));
    }
    expect(errores, "errores o avisos en la consola").toEqual([]);
  });
});
