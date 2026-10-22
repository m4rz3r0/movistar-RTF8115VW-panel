import { expect, type Page, test } from "@playwright/test";
import { asentar, entrar, quieta, sinDesbordamiento } from "./util";

// El diálogo de revisión de Ajustes a 360 px con valores largos: una MAC
// («Fijar esta IP») y la dirección web del servidor de la gestión remota.
// Los valores se parten solo tras «:», «.», «/», «-» o un espacio, nunca en
// mitad de un número o de una palabra, y los títulos de las columnas no se
// parten. Se cancela sin aplicar: el simulador queda como estaba.

test.describe.configure({ mode: "parallel" });
test.use({ viewport: { width: 360, height: 800 } });

const URL_LARGA = "https://gestion-remota-del-operador.example/cwmp/servicio/acs";

/**
 * Cortes de línea feos en las celdas de la tabla del diálogo abierto: cada
 * uno, con el texto de la celda y lo que queda a cada lado del corte.
 */
async function cortesFeos(page: Page): Promise<string[]> {
  return page.locator("dialog[open] .tabla").evaluate((tabla) => {
    const feos: string[] = [];
    for (const celda of tabla.querySelectorAll("th, td")) {
      // Los caracteres de la celda, cada uno con la altura de su línea.
      const letras: { c: string; top: number }[] = [];
      const recorrido = document.createTreeWalker(celda, NodeFilter.SHOW_TEXT);
      for (let n = recorrido.nextNode(); n; n = recorrido.nextNode()) {
        const texto = n.textContent ?? "";
        for (let i = 0; i < texto.length; i++) {
          const r = document.createRange();
          r.setStart(n, i);
          r.setEnd(n, i + 1);
          const caja = r.getBoundingClientRect();
          if (caja.width === 0 && caja.height === 0) continue;
          letras.push({ c: texto[i], top: caja.top });
        }
      }
      for (let i = 1; i < letras.length; i++) {
        const corte = letras[i].top > letras[i - 1].top + 2;
        if (corte && !/[\s:./-]/.test(letras[i - 1].c)) {
          const todo = letras.map((l) => l.c).join("");
          feos.push(`«${todo}»: «${todo.slice(0, i)}» | «${todo.slice(i)}»`);
        }
      }
    }
    return feos;
  });
}

test("una MAC en el diálogo, entera o partida por «:»", async ({ page }) => {
  await entrar(page);
  await page.goto("/ajustes/red_local");
  await asentar(page);
  const dispositivos = page.locator("section.lista-seccion").filter({
    has: page.getByRole("heading", { level: 2, name: "Dispositivos conectados", exact: true }),
  });
  await dispositivos
    .locator("tbody tr.lista-fila")
    .filter({ hasText: "host-2" })
    .locator(".lista-nombre")
    .click();
  await page
    .locator(".detalle-lateral")
    .getByRole("button", { name: /^Fijar esta IP/ })
    .click();
  const dialogo = page.locator("dialog[open]");
  await expect(dialogo.getByText("02:00:00:00:00:03")).toBeVisible();
  await quieta(page);
  expect(await cortesFeos(page)).toEqual([]);
  await sinDesbordamiento(page, "diálogo con una MAC");
  await page.screenshot({ path: "capturas/dialogo-valores-mac-360.png" });
  await dialogo.getByRole("button", { name: "Cancelar", exact: true }).click();
  await expect(dialogo).toHaveCount(0);
});

test("una dirección web en el diálogo, partida por «/», «.» o «-»", async ({ page }) => {
  await entrar(page);
  await page.goto("/ajustes/seguridad");
  await asentar(page);
  await page
    .getByRole("button", { name: "Abrir Gestión remota del operador", exact: true })
    .click();
  const detalle = page.locator(".detalle");
  await detalle.getByLabel("Dirección del servidor del operador", { exact: true }).fill(URL_LARGA);
  await detalle.getByRole("button", { name: "Guardar cambios", exact: true }).click();
  const dialogo = page.locator("dialog[open]");
  await expect(dialogo.getByText(URL_LARGA)).toBeVisible();
  await quieta(page);
  expect(await cortesFeos(page)).toEqual([]);
  await sinDesbordamiento(page, "diálogo con una dirección web");
  await page.screenshot({ path: "capturas/dialogo-valores-url-360.png" });
  await dialogo.getByRole("button", { name: "Cancelar", exact: true }).click();
  await expect(dialogo).toHaveCount(0);
});
