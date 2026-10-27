import { expect, test } from "@playwright/test";
import { asentar, entrar, recogerErrores, sinDesbordamiento, violacionesGraves } from "./util";

// «Registro del router», al pie de Sistema, contra router-sim: leerlo
// (exporta y lee /syslog.log), filtrar sus líneas, descargarlo con su
// nombre de fichero y que el bloque no desborde la página en el móvil.

for (const ancho of [360, 1440]) {
  test(`registro del router a ${ancho} px`, async ({ page }) => {
    await page.setViewportSize({ width: ancho, height: 900 });
    const errores = recogerErrores(page);
    await entrar(page);
    await page.goto("/ajustes/sistema");
    await asentar(page);
    const seccion = page.locator("section.registro-router");
    await expect(seccion.getByRole("heading", { name: "Registro del router" })).toBeVisible();
    const descargar = seccion.getByRole("button", { name: "Descargar" });
    await expect(descargar).toBeDisabled();

    await seccion.getByRole("button", { name: "Leer el registro" }).click();
    const texto = seccion.locator("pre.registro-router-texto");
    await expect(texto).toContainText("syslogd started");
    await expect(seccion.getByRole("button", { name: "Volver a leer" })).toBeVisible();
    await expect(seccion.locator(".registro-router-leido")).toContainText("Leído");

    await seccion.getByLabel("Filtrar líneas").fill("dnsmasq");
    await expect(texto).toContainText("dnsmasq");
    await expect(texto).not.toContainText("syslogd started");

    const [descarga] = await Promise.all([page.waitForEvent("download"), descargar.click()]);
    expect(descarga.suggestedFilename()).toMatch(/^registro-router-\d{8}-\d{4}\.log$/);

    await sinDesbordamiento(page, `registro a ${ancho} px`);
    expect(await violacionesGraves(page)).toEqual([]);
    expect(errores).toEqual([]);
  });
}
