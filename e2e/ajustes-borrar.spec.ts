import { expect, type Locator, type Page, test } from "@playwright/test";
import { crearRegla, esperarFinal, plazoDeTest } from "./cambios";
import { asentar, entrar, recogerErrores } from "./util";

// Borrar dos reglas de Puertos desde la barra, en español a 1440 px,
// contra router-sim: al aplicarse, lo borrado deja de estar marcado (la
// barra se oculta, sin «2 marcadas» huérfano) y el foco va a la fila
// vecina, no al <body>. Con cambios de verdad: crea las dos reglas
// (puertos 9101 y 9102, que no usa nadie más) y las borra, así que el
// router queda como estaba.

/** Las filas de la lista de reglas. */
function filas(page: Page): Locator {
  return page
    .locator("section.lista-seccion")
    .filter({
      has: page.getByRole("heading", { level: 2, name: "Puertos abiertos", exact: true }),
    })
    .locator("tbody tr.lista-fila");
}

test.describe("listas: borrar desde la barra", () => {
  test.use({ locale: "es-ES", viewport: { width: 1440, height: 900 } });
  test("lo borrado se desmarca y el foco va a la fila vecina", async ({ page }) => {
    test.setTimeout(plazoDeTest(3));
    const errores = recogerErrores(page);
    await entrar(page);
    await esperarFinal(page, await crearRegla(page, "Borrable 1", 9101));
    await esperarFinal(page, await crearRegla(page, "Borrable 2", 9102));
    await page.goto("/ajustes/puertos");
    await asentar(page);
    await expect(filas(page)).toHaveCount(3);
    for (const nombre of ["Borrable 1", "Borrable 2"]) {
      const fila = filas(page).filter({ hasText: nombre });
      await fila.hover();
      await fila.getByRole("button", { name: `Marcar ${nombre}` }).click();
    }
    const barra = page.getByRole("toolbar");
    await expect(barra).toContainText("2 marcadas");
    await barra.getByRole("button", { name: "Borrar", exact: true }).click();
    const dialogo = page.locator("dialog[open]");
    await expect(dialogo.locator("#grupo-titulo")).toHaveText("Revisa el cambio");
    await dialogo.getByRole("button", { name: "Borrar", exact: true }).click();
    const aviso = page.locator(".avisos-cambio [role=status] .aviso-cambio");
    await expect(aviso).toContainText("Hecho: el router tiene ya el cambio.", {
      timeout: plazoDeTest(1),
    });
    // Ya no están, nada queda marcado y la barra se oculta.
    await expect(filas(page)).toHaveCount(1);
    await expect(barra).toBeHidden();
    await expect(page.locator(".marcar[aria-pressed=true]")).toHaveCount(0);
    // El foco, en la fila que queda (la vecina de las borradas).
    await expect(filas(page).filter({ hasText: "desc-1" })).toBeFocused();
    expect(errores, "errores o avisos en la consola").toEqual([]);
  });
});
