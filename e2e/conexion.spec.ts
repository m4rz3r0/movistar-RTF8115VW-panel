import { expect, test } from "@playwright/test";
import { asentar, entrar, recogerErrores, sinDesbordamiento, violacionesGraves } from "./util";

// Conexión a internet (/ajustes/internet/conexion), en español a 360 y
// 1440 px, contra router-sim y solo de lectura: la tarjeta en la rejilla con
// su estado; en el detalle, la conexión de internet primero y las del
// operador marcadas, la contraseña PPP censurada que el ojo muestra, la
// fibra «Operativa» dentro de la misma columna (bloque «Fibra», con su
// número de serie censurado que el ojo muestra) y «Cómo pasar a
// monopuesto», que se abre (pasos y avisos, sin botón). Axe sin violaciones serias ni críticas y sin scroll
// horizontal.

const t = {
  tarjeta: "Conexión a internet",
  estado: "Conectada (PPPoE)",
  internet: "Tu conexión a internet",
  operador: "Del operador (no es tu internet)",
  ojo: "Mostrar Contraseña",
  operativa: "Operativa (O5)",
  fibra: "Fibra",
  ojoSerie: "Mostrar Número de serie de la fibra",
  monopuesto: "Cómo pasar a monopuesto",
  // Las fixtures saneadas.
  clave: "X-REDACTED-X",
};

test.describe("conexión a internet", () => {
  test.use({ locale: "es-ES" });

  for (const ancho of [360, 1440]) {
    test(`a ${ancho} px: conexiones, contraseña con el ojo y monopuesto`, async ({ page }) => {
      await page.setViewportSize({ width: ancho, height: 900 });
      const errores = recogerErrores(page);
      await entrar(page);
      await page.goto("/ajustes/internet");
      await asentar(page);
      const tarjeta = page.locator("article.tarjeta-el").filter({
        has: page.getByRole("heading", { level: 2, name: t.tarjeta, exact: true }),
      });
      await expect(tarjeta).toContainText(t.estado);

      await page.goto("/ajustes/internet/conexion");
      await asentar(page);
      const bloques = page.locator(".conexiones .conexion-bloque");
      await expect(bloques).toHaveCount(3);
      await expect(bloques.first().getByRole("heading", { level: 4 })).toHaveText(t.internet);
      await expect(page.locator(".conexion-operador h4")).toHaveText([t.operador, t.operador]);
      // La fibra, en un bloque «Fibra» de la misma columna (no «Avanzado»).
      const fibra = page.locator(".conexion-anexo");
      await expect(fibra.getByRole("heading", { level: 4 })).toHaveText(t.fibra);
      await expect(fibra).toContainText(t.operativa);
      await expect(page.locator(".detalle-columna")).toHaveCount(1);
      await sinDesbordamiento(page, `conexión ${ancho}`);
      expect(await violacionesGraves(page), `conexión ${ancho}`).toEqual([]);

      // La contraseña PPP: censurada, el ojo la muestra.
      const internet = bloques.first();
      const campo = internet.locator("input.secreto-valor");
      await expect(campo).toHaveAttribute("type", "password");
      await expect(campo).toHaveValue(t.clave);
      await internet.getByRole("button", { name: t.ojo, exact: true }).click();
      await expect(campo).toHaveAttribute("type", "text");
      // El número de serie de la fibra: otro camino, el mismo ojo.
      const serie = fibra.locator("input.secreto-valor").first();
      await expect(serie).toHaveAttribute("type", "password");
      await expect(serie).toHaveValue(t.clave);
      await fibra.getByRole("button", { name: t.ojoSerie, exact: true }).click();
      await expect(serie).toHaveAttribute("type", "text");
      await expect(serie).toHaveValue(t.clave);

      // «Cómo pasar a monopuesto» se abre: pasos y avisos, sin botón.
      const monopuesto = page.locator("details.conexion-monopuesto");
      await expect(monopuesto).not.toHaveAttribute("open", "");
      await monopuesto.getByText(t.monopuesto, { exact: true }).click();
      await expect(monopuesto).toHaveAttribute("open", "");
      await expect(monopuesto.locator(".conexion-pasos li")).toHaveCount(5);
      await expect(monopuesto.locator(".conexion-avisos li").first()).toBeVisible();
      await expect(monopuesto.getByRole("button")).toHaveCount(0);
      // Ni «Guardar cambios»: todo es de solo lectura.
      await expect(page.locator(".detalle-pie")).toHaveCount(0);
      await sinDesbordamiento(page, `monopuesto ${ancho}`);
      expect(await violacionesGraves(page), `monopuesto ${ancho}`).toEqual([]);
      expect(errores, "errores o avisos en la consola").toEqual([]);
    });
  }
});
