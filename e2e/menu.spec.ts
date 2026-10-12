import { expect, type Page, test } from "@playwright/test";
import { USUARIO } from "./instancias";
import { asentar, entrar, recogerErrores, sinDesbordamiento } from "./util";

// Menú reorganizado, a 360 y 1440 px, en español y en inglés: el nombre de
// usuario abre «Ajustes del panel»; Recomendaciones lleva a «Tus
// preferencias», donde una sugerencia de DNS se guarda y sigue tras recargar.

const textos = {
  es: {
    principal: "Principal",
    recomendaciones: "Recomendaciones",
    ajustesPanel: "Ajustes del panel",
    preferencias: "Tus preferencias",
    usarQuad9: "Usar Quad9",
    guardar: "Guardar",
    guardado: "Guardado.",
  },
  en: {
    principal: "Main",
    recomendaciones: "Recommendations",
    ajustesPanel: "Panel settings",
    preferencias: "Your preferences",
    usarQuad9: "Use Quad9",
    guardar: "Save",
    guardado: "Saved.",
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

      test("nombre de usuario, Recomendaciones y Tus preferencias", async ({ page }) => {
        const t = textos[idioma];
        const errores = recogerErrores(page);
        await entrar(page);
        await asentar(page);

        // 1-2. El nombre de usuario abre «Ajustes del panel».
        await abrirMenu(page, ancho);
        const usuario = page.locator("a.sesion-usuario");
        await expect(usuario).toContainText(USUARIO);
        await expect(usuario).toHaveAccessibleName(`${USUARIO} ${t.ajustesPanel}`);
        await usuario.click();
        await page.waitForURL((url) => url.pathname === "/panel");
        await expect(page.locator("main h1")).toHaveText(t.ajustesPanel);
        await expect(usuario).toHaveAttribute("aria-current", "page");

        // 3. Recomendaciones lleva a «Tus preferencias».
        await abrirMenu(page, ancho);
        const nav = page.getByRole("navigation", { name: t.principal, exact: true });
        const recomendaciones = nav.getByRole("link", { name: t.recomendaciones });
        await recomendaciones.click();
        await page.waitForURL((url) => url.pathname === "/recomendaciones/preferencias");
        await expect(page.locator("main h1")).toHaveText(t.preferencias);
        // En móvil el menú se pliega al navegar: el enlace sigue en la página.
        await expect(page.locator("nav.nav a[href='/recomendaciones/preferencias']")).toHaveAttribute(
          "aria-current",
          "page",
        );
        await expect(usuario).not.toHaveAttribute("aria-current", "page");

        // 4. Una sugerencia de DNS, guardada.
        const dns = page.locator("#pref-dns");
        const guardar = page.getByRole("button", { name: t.guardar, exact: true });
        await page.getByRole("button", { name: t.usarQuad9 }).click();
        await expect(dns).toHaveValue("9.9.9.9");
        await guardar.click();
        await expect(page.locator(".aviso-formulario")).toHaveText(t.guardado);
        await sinDesbordamiento(page, `tus preferencias ${idioma} ${ancho}`);
        await page.screenshot({
          path: `capturas/preferencias-${idioma}-${ancho}.png`,
          fullPage: true,
        });

        // 5. Tras recargar, sigue guardada. Se deja como estaba (vacía).
        await page.reload();
        await asentar(page);
        await expect(dns).toHaveValue("9.9.9.9");
        await dns.fill("");
        await guardar.click();
        await expect(page.locator(".aviso-formulario")).toHaveText(t.guardado);
        await page.reload();
        await asentar(page);
        await expect(dns).toHaveValue("");

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
