import { expect, type Page, test } from "@playwright/test";
import { USUARIO } from "./instancias";
import { asentar, entrar, recogerErrores, sinDesbordamiento } from "./util";

// Menú reorganizado, a 360 y 1440 px, en español y en inglés: el nombre de
// usuario abre «Ajustes del panel», y Recomendaciones no sale en el menú
// hasta que tenga sus tarjetas (fase 4).

const textos = {
  es: {
    principal: "Principal",
    recomendaciones: "Recomendaciones",
    ajustesPanel: "Ajustes del panel",
  },
  en: {
    principal: "Main",
    recomendaciones: "Recommendations",
    ajustesPanel: "Panel settings",
  },
} as const;

/** En móvil el menú está plegado: lo abre. En escritorio ya se ve. */
async function abrirMenu(page: Page, ancho: number) {
  if (ancho > 860) return;
  const boton = page.locator("button.menu-boton");
  await boton.click();
  await expect(boton).toHaveAttribute("aria-expanded", "true");
}

for (const idioma of ["es", "en"] as const) {
  for (const ancho of [360, 1440]) {
    test.describe(`menú ${idioma} a ${ancho} px`, () => {
      test.use({
        locale: idioma === "es" ? "es-ES" : "en-GB",
        viewport: { width: ancho, height: 900 },
      });

      test("nombre de usuario y sin Recomendaciones", async ({ page }) => {
        const t = textos[idioma];
        const errores = recogerErrores(page);
        await entrar(page);
        await asentar(page);

        // 1. Sin Recomendaciones en el menú.
        await abrirMenu(page, ancho);
        const nav = page.getByRole("navigation", { name: t.principal, exact: true });
        await expect(nav.getByRole("link").first()).toBeVisible();
        await expect(nav.getByRole("link", { name: t.recomendaciones })).toHaveCount(0);
        await expect(page.locator("a[href^='/recomendaciones']")).toHaveCount(0);
        await sinDesbordamiento(page, `menú ${idioma} ${ancho}`);
        await page.screenshot({ path: `capturas/menu-${idioma}-${ancho}.png`, fullPage: true });

        // 2. El nombre de usuario abre «Ajustes del panel».
        const usuario = page.locator("a.sesion-usuario");
        await expect(usuario).toContainText(USUARIO);
        await expect(usuario).toHaveAccessibleName(`${USUARIO} ${t.ajustesPanel}`);
        await usuario.click();
        await page.waitForURL((url) => url.pathname === "/panel");
        await expect(page.locator("main h1")).toHaveText(t.ajustesPanel);
        await expect(usuario).toHaveAttribute("aria-current", "page");
        // Su subtítulo tampoco enlaza con Recomendaciones.
        await expect(page.locator("a[href^='/recomendaciones']")).toHaveCount(0);
        await sinDesbordamiento(page, `ajustes del panel ${idioma} ${ancho}`);

        expect(errores, "errores o avisos en la consola").toEqual([]);
      });
    });
  }
}

test.describe("nombre de usuario largo a 360 px", () => {
  test.use({ viewport: { width: 360, height: 800 } });

  test("se corta con puntos suspensivos sin ensanchar la página", async ({ page }) => {
    await entrar(page);
    await asentar(page);
    await abrirMenu(page, 360);
    const nombre = page.locator(".sesion-nombre");
    await nombre.evaluate((el) => {
      el.textContent = "un-usuario-con-un-nombre-larguisimo-que-no-cabe-en-el-menu";
    });
    const medidas = await nombre.evaluate((el) => ({
      corte: getComputedStyle(el).textOverflow,
      cabe: el.scrollWidth <= el.clientWidth,
    }));
    expect(medidas).toEqual({ corte: "ellipsis", cabe: false });
    await sinDesbordamiento(page, "menú con un nombre de usuario largo");
  });
});
