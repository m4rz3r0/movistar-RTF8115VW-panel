import { expect, type Locator, type Page, test } from "@playwright/test";
import { PLAZO_CAMBIO, plazoDeTest } from "./cambios";
import { CONTRASENA, USUARIO, seguridad } from "./instancias";
import { asentar, hidratada, recogerErrores, sinDesbordamiento, violacionesGraves } from "./util";

// Ajustes → Seguridad → Usuarios, en español a 360 y 1440 px, contra
// router-sim y en un panel propio (instancias.ts), en serie: la fila del
// usuario con el que entra el panel (`sim`) dice por qué no se toca y no
// tiene botones; se crea un usuario, se borra (el diálogo avisa de que el
// deshacer no le devuelve la contraseña) y se deshace el borrado. Cada
// ancho usa su propio usuario. Axe sin violaciones serias ni críticas y sin
// scroll horizontal.
//
// Cortafuegos, también a 360 y 1440 px y en el mismo panel: la tabla de
// reglas va en su caja con scroll (la página no se desborda); «No responder
// al ping desde internet» se aplica (la regla del panel aparece y el botón
// dice que ya está puesta) y se deshace (vuelven las reglas de antes).

const t = {
  aplicar: "Aplicar",
  borrar: "Borrar",
  deshacer: "Deshacer",
  crear: "Crear usuario",
  hecho: "Hecho: el router tiene ya el cambio.",
  protegidoPanel: "Es el usuario con el que entra el panel",
  avisoBorrar: "Si luego deshaces el borrado",
  avisoDeshacer: "El usuario vuelve con una contraseña nueva",
  sinPing: "No responder al ping desde internet",
  yaPuesta: "ya puesta",
};

/** Entra en el panel de Seguridad (`baseURL`), que no es el del worker. */
async function entrarEnSuPanel(page: Page) {
  await page.goto("/login");
  await hidratada(page);
  await page.waitForLoadState("networkidle");
  await page.locator("#usuario").fill(USUARIO);
  await page.locator("#contrasena").fill(CONTRASENA);
  await page.locator("form button[type=submit]").click();
  await page.waitForURL((url) => url.pathname === "/");
}

/** La fila del usuario `nombre`. */
function fila(page: Page, nombre: string): Locator {
  return page
    .locator(".usuarios-router .ajustes-usuario")
    .filter({ has: page.locator("strong", { hasText: new RegExp(`^${nombre}$`) }) });
}

/** Aplica el grupo del diálogo abierto con `boton` y espera a que acabe. */
async function aplicar(page: Page, boton: string) {
  const dialogo = page.locator("dialog[open]");
  await dialogo.getByRole("button", { name: boton, exact: true }).click();
  await expect(page.locator("dialog[open]")).toHaveCount(0);
  const aviso = page.locator(".avisos-cambio [role=status] .aviso-cambio");
  await expect(aviso).toContainText(t.hecho, { timeout: PLAZO_CAMBIO });
  return aviso;
}

test.describe("usuarios del router", () => {
  test.describe.configure({ mode: "serial" });
  test.use({ locale: "es-ES", baseURL: seguridad.url });

  for (const ancho of [360, 1440]) {
    test(`a ${ancho} px: crear, borrar con aviso y deshacer`, async ({ page }) => {
      test.setTimeout(plazoDeTest(3));
      await page.setViewportSize({ width: ancho, height: 900 });
      const errores = recogerErrores(page);
      const nuevo = `prueba${ancho}`;
      await entrarEnSuPanel(page);
      await page.goto("/ajustes/seguridad/usuarios");
      await asentar(page);
      // El del panel: su nota y ningún botón.
      const delPanel = fila(page, "sim");
      await expect(delPanel).toContainText(t.protegidoPanel);
      await expect(delPanel.getByRole("button")).toHaveCount(0);
      await sinDesbordamiento(page, `usuarios ${ancho}`);
      expect(await violacionesGraves(page), `usuarios ${ancho}`).toEqual([]);

      // Crear: nombre, contraseña con el ojo y «Usuario».
      await page.locator("#usuarios-router-nuevo-nombre").fill(nuevo);
      await page.locator("#usuarios-router-nuevo-contrasena").fill("clave-prueba-1");
      await expect(page.getByRole("radio", { name: "Usuario", exact: true }).first()).toBeChecked();
      await page.getByRole("button", { name: t.crear, exact: true }).click();
      await expect(page.locator("dialog[open] #grupo-titulo")).toHaveText("Revisa el cambio");
      await aplicar(page, t.aplicar);
      await expect(fila(page, nuevo)).toContainText("Usuario", { timeout: PLAZO_CAMBIO });

      // Borrar: el diálogo avisa antes de confirmar, con el botón de peligro.
      await fila(page, nuevo).getByRole("button", { name: `Borrar ${nuevo}`, exact: true }).click();
      const dialogo = page.locator("dialog[open]");
      await expect(dialogo).toContainText(t.avisoBorrar);
      await sinDesbordamiento(page, `borrar ${ancho}`);
      expect(await violacionesGraves(page), `diálogo de borrar ${ancho}`).toEqual([]);
      const aviso = await aplicar(page, t.borrar);
      // El detalle sigue abierto: se borra una fila, no el elemento.
      await expect(fila(page, nuevo)).toHaveCount(0, { timeout: PLAZO_CAMBIO });
      await expect(fila(page, "sim")).toBeVisible();

      // Deshacer: vuelve, con una contraseña nueva (se avisa). En el móvil,
      // el detalle ocupa la pantalla: el aviso con «Deshacer» está debajo.
      const movil = ancho < 900;
      if (movil) {
        await page.getByRole("button", { name: "‹ Seguridad" }).click();
        await page.waitForURL((u) => u.pathname === "/ajustes/seguridad");
      }
      await aviso.getByRole("button", { name: t.deshacer, exact: true }).click();
      await expect(page.locator("dialog[open]")).toContainText(t.avisoDeshacer);
      await aplicar(page, t.deshacer);
      if (movil) {
        await page.getByRole("button", { name: "Abrir Usuarios de la web del router" }).click();
        await page.waitForURL((u) => u.pathname === "/ajustes/seguridad/usuarios");
      }
      await expect(fila(page, nuevo)).toHaveCount(1, { timeout: PLAZO_CAMBIO });
      await sinDesbordamiento(page, `deshecho ${ancho}`);
      expect(errores, "errores o avisos en la consola").toEqual([]);
    });
  }
});

test.describe("cortafuegos", () => {
  test.describe.configure({ mode: "serial" });
  test.use({ locale: "es-ES", baseURL: seguridad.url });

  for (const ancho of [360, 1440]) {
    test(`a ${ancho} px: no responder al ping y deshacer`, async ({ page }) => {
      test.setTimeout(plazoDeTest(3));
      await page.setViewportSize({ width: ancho, height: 900 });
      const errores = recogerErrores(page);
      await entrarEnSuPanel(page);
      await page.goto("/ajustes/seguridad/cortafuegos");
      await asentar(page);
      const filas = page.locator(".cortafuegos-reglas .tabla-scroll tbody tr");
      await expect(filas).toHaveCount(16);
      await sinDesbordamiento(page, `cortafuegos ${ancho}`);
      expect(await violacionesGraves(page), `cortafuegos ${ancho}`).toEqual([]);

      // Aplicar: el diálogo de revisión y, hecho, la fila 17 (la regla RID 16) y el motivo.
      await page.getByRole("button", { name: t.sinPing, exact: true }).click();
      await expect(page.locator("dialog[open] #grupo-titulo")).toHaveText("Revisa el cambio");
      expect(await violacionesGraves(page), `diálogo sin ping ${ancho}`).toEqual([]);
      const aviso = await aplicar(page, t.aplicar);
      await expect(filas).toHaveCount(17, { timeout: PLAZO_CAMBIO });
      await expect(page.locator(".cortafuegos-sin-ping")).toContainText(t.yaPuesta);
      await sinDesbordamiento(page, `sin ping ${ancho}`);

      // Deshacer: quita solo su regla. En el móvil, el detalle tapa el aviso.
      const movil = ancho < 900;
      if (movil) {
        await page.getByRole("button", { name: "‹ Seguridad" }).click();
        await page.waitForURL((u) => u.pathname === "/ajustes/seguridad");
      }
      await aviso.getByRole("button", { name: t.deshacer, exact: true }).click();
      await expect(page.locator("dialog[open] #grupo-titulo")).toBeVisible();
      await aplicar(page, t.deshacer);
      if (movil) {
        await page.getByRole("button", { name: "Abrir Cortafuegos" }).click();
        await page.waitForURL((u) => u.pathname === "/ajustes/seguridad/cortafuegos");
      }
      await expect(filas).toHaveCount(16, { timeout: PLAZO_CAMBIO });
      await expect(page.locator(".cortafuegos-sin-ping")).not.toContainText(t.yaPuesta);
      expect(await violacionesGraves(page), `deshecho ${ancho}`).toEqual([]);
      expect(errores, "errores o avisos en la consola").toEqual([]);
    });
  }
});
