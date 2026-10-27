import { readFile } from "node:fs/promises";
import { expect, test } from "@playwright/test";
import { asentar, entrar, recogerErrores, sinDesbordamiento, violacionesGraves } from "./util";

// «Copia de la configuración», al pie de Sistema, contra router-sim (con
// todo habilitado para las pruebas): el aviso de las contraseñas, la
// descarga con el nombre del router (config_<fecha>.tar) y su contenido
// (inventado por el simulador), y que la sección no desborde en el móvil.

for (const ancho of [360, 1440]) {
  test(`copia de la configuración a ${ancho} px`, async ({ page }) => {
    await page.setViewportSize({ width: ancho, height: 900 });
    const errores = recogerErrores(page);
    await entrar(page);
    await page.goto("/ajustes/sistema");
    await asentar(page);
    const seccion = page.locator("section.copia-router");
    await expect(seccion.getByRole("heading", { name: "Copia de la configuración" })).toBeVisible();
    await expect(seccion).toContainText("todas las contraseñas del router");
    const boton = seccion.getByRole("button", { name: "Descargar copia" });
    await expect(boton).toBeEnabled();

    const [descarga] = await Promise.all([page.waitForEvent("download"), boton.click()]);
    expect(descarga.suggestedFilename()).toMatch(/^config_\d+\.tar$/);
    const ruta = await descarga.path();
    const datos = await readFile(ruta);
    expect(datos.subarray(0, 6).toString("latin1")).toBe("ASP_CM");
    await expect(seccion.locator(".copia-router-hecha")).toContainText(
      descarga.suggestedFilename(),
    );
    // El contenido no se queda en la página.
    await expect(page.locator("body")).not.toContainText("copia simulada");

    await sinDesbordamiento(page, `copia a ${ancho} px`);
    expect(await violacionesGraves(page)).toEqual([]);
    expect(errores).toEqual([]);
  });
}
