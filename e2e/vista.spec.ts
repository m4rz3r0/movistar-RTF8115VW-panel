import { expect, test, type Page } from "@playwright/test";
import { instalado } from "./instancias";
import { asentar, entrar, recogerErrores, sinDesbordamiento } from "./util";

// Modos de visualización (texto, gráfica, mixto): se eligen en la cabecera
// de Estado o en «Ajustes del panel → Visualización», se guardan por usuario
// y sobreviven a recargar. El servidor pone el modo en <html data-vista> y
// la hidratación pinta lo mismo (sin errores en la consola). Al terminar se
// vuelve a «mixto», el de por defecto.

/** Elige `modo` en el selector de la cabecera de Estado. */
async function elegirEnEstado(page: Page, modo: string) {
  const boton = page.locator(`.selector-vista button[value=${modo}]`);
  const guardado = page.waitForResponse((r) => r.url().endsWith("/api/guardar_vista"));
  await boton.click();
  expect((await guardado).status()).toBe(200);
  await expect(boton).toHaveAttribute("aria-pressed", "true");
}

/** Lo que pinta Estado en cada modo (ya con el primer dato del router). */
async function comprobarEstado(page: Page, modo: string) {
  await expect(page.locator("main .estado-espera")).toHaveCount(0, { timeout: 20_000 });
  await expect(page.locator("html")).toHaveAttribute("data-vista", modo);
  await expect(page.locator(".selector-vista button[aria-pressed=true]")).toHaveAttribute(
    "value",
    modo,
  );
  const main = page.locator("main");
  if (modo === "texto") {
    await expect(main.locator(".historico-texto table")).toHaveCount(2);
    await expect(main.locator(".historico-texto th[scope=row]")).toHaveCount(5);
    await expect(main.locator("dl.indicadores-texto")).toBeVisible();
    await expect(main.locator("svg")).toHaveCount(0);
  } else if (modo === "grafica") {
    await expect(main.locator("figure.grafica").first()).toBeVisible();
    await expect(main.locator(".rejilla-indicadores.compactos")).toBeVisible();
    await expect(main.locator(".rejilla-indicadores.compactos .indicador-sub").first()).toBeVisible();
    await expect(main.locator(".minigrafica")).toHaveCount(0);
  } else {
    await expect(main.locator(".minigrafica").first()).toBeVisible();
    await expect(main.locator("figure.grafica").first()).toBeVisible();
  }
}

test.describe("modos de visualización", () => {
  test.use({ viewport: { width: 1440, height: 900 } });

  test("desde Estado y desde Ajustes del panel, y se conserva al recargar", async ({ page }) => {
    const errores = recogerErrores(page);
    await entrar(page);
    await asentar(page);
    await comprobarEstado(page, "mixto");
    try {
      // Selector rápido de Estado: cambia sin recargar y se guarda.
      await elegirEnEstado(page, "texto");
      await comprobarEstado(page, "texto");
      await page.reload();
      await asentar(page);
      await comprobarEstado(page, "texto");

      // Ajustes del panel: la misma preferencia, con las tres opciones.
      await page.goto("/panel");
      await asentar(page);
      const seccion = page.locator("section[aria-labelledby=panel-vista]");
      await expect(seccion.locator("input[name=vista]")).toHaveCount(3);
      await expect(seccion.locator("input[value=texto]")).toBeChecked();
      await seccion.getByLabel("Gráfica").check();
      await seccion.locator("button[type=submit]").click();
      await expect(seccion.getByRole("status")).toHaveText("Guardado.");
      await page.reload();
      await asentar(page);
      await expect(page.locator("html")).toHaveAttribute("data-vista", "grafica");
      await expect(seccion.locator("input[value=grafica]")).toBeChecked();
      await page.goto("/");
      await asentar(page);
      await comprobarEstado(page, "grafica");
    } finally {
      await page.goto("/");
      await asentar(page);
      await elegirEnEstado(page, "mixto");
    }
    await page.reload();
    await asentar(page);
    await comprobarEstado(page, "mixto");
    expect(errores, "errores o avisos en la consola").toEqual([]);
  });

  for (const idioma of ["es", "en"]) {
    for (const ancho of [360, 1440]) {
      test(`los tres modos sin errores de hidratación (${idioma}, ${ancho} px)`, async ({
        page,
        context,
      }) => {
        await page.setViewportSize({ width: ancho, height: 900 });
        await context.addCookies([{ name: "rp_lang", value: idioma, url: instalado.url }]);
        const errores = recogerErrores(page);
        await entrar(page);
        await asentar(page);
        try {
          for (const modo of ["texto", "grafica", "mixto"]) {
            await elegirEnEstado(page, modo);
            await page.reload();
            await asentar(page);
            await expect(page.locator("html")).toHaveAttribute("lang", idioma);
            await comprobarEstado(page, modo);
            await sinDesbordamiento(page, `Estado en «${modo}» (${idioma}, ${ancho} px)`);
          }
          await page.goto("/panel");
          await asentar(page);
          await sinDesbordamiento(page, `Ajustes del panel (${idioma}, ${ancho} px)`);
        } finally {
          await page.goto("/");
          await asentar(page);
          if ((await page.locator("html").getAttribute("data-vista")) !== "mixto") {
            await elegirEnEstado(page, "mixto");
          }
        }
        expect(errores, "errores o avisos en la consola").toEqual([]);
      });
    }
  }
});
