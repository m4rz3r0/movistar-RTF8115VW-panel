import { expect, type Locator, type Page, test } from "@playwright/test";
import { asentar, entrar, recogerErrores, sinDesbordamiento } from "./util";

// Páginas de Ajustes (/ajustes/<tema>) a 360 y 1440 px, en español y en
// inglés, contra router-sim: abrir WiFi, cambiar el nombre de la red de
// 5 GHz, ver el resumen «antes → después» sin nombres de la API, «Más
// opciones» y, en móvil, el selector de temas. El diálogo: el foco entra en
// su título y vuelve a «Guardar»; Escape no lo cierra mientras confirma.
//
// Aplicar, esperar «aplicado» y deshacer se hace en una sola combinación
// (inglés, 1440 px, la última, para que un fallo no deje un cambio vigilándose
// a las demás): la WiFi es de riesgo medio y cada aplicación se vigila
// 2 min, así que repetirlo en las cuatro alargaría la pasada unos 16 min.
// Las demás llegan hasta el resumen y lo cancelan.

const textos = {
  es: {
    red5: "Red WiFi 5 GHz",
    nombre: "Nombre de la red",
    editar: "Editar",
    guardar: "Guardar",
    cancelar: "Cancelar",
    aplicar: "Aplicar",
    deshacer: "Deshacer",
    cerrar: "Cerrar",
    revisar: "Revisa el cambio",
    volver: "Volver a como estaba",
    hecho: "Hecho: el router tiene ya el cambio.",
    mas: "Más opciones",
    red5Avanzada: "Red WiFi 5 GHz: más opciones",
    puertos: "Puertos",
  },
  en: {
    red5: "5 GHz WiFi network",
    nombre: "Network name",
    editar: "Edit",
    guardar: "Save",
    cancelar: "Cancel",
    aplicar: "Apply",
    deshacer: "Undo",
    cerrar: "Close",
    revisar: "Review the change",
    volver: "Put it back as it was",
    hecho: "Done: the router now has the change.",
    mas: "More options",
    red5Avanzada: "5 GHz WiFi network: more options",
    puertos: "Ports",
  },
} as const;

/** La tarjeta cuyo título es exactamente `titulo`. */
function tarjeta(page: Page, titulo: string): Locator {
  return page.locator("article.ajuste").filter({
    has: page.getByRole("heading", { level: 2, name: titulo, exact: true }),
  });
}

/** Sin funciones ni parámetros de la API en un texto visible. */
function sinApi(texto: string) {
  expect(texto).not.toMatch(/\bcb[A-Z]\w+/);
  expect(texto).not.toMatch(/\bssid\b|key_phrase|authentication/);
}

for (const idioma of ["es", "en"] as const) {
  for (const ancho of [360, 1440]) {
    const aplicaDeVerdad = idioma === "en" && ancho === 1440;
    test.describe(`ajustes ${idioma} a ${ancho} px`, () => {
      test.use({
        locale: idioma === "es" ? "es-ES" : "en-GB",
        viewport: { width: ancho, height: 900 },
      });

      test("WiFi: cambiar el nombre de la red de 5 GHz", async ({ page }) => {
        if (aplicaDeVerdad) test.setTimeout(420_000);
        const t = textos[idioma];
        const errores = recogerErrores(page);
        await entrar(page);

        // 1. Abrir WiFi (en móvil, con el selector de temas).
        if (ancho < 640) {
          await page.goto("/ajustes/puertos");
          await asentar(page);
          await expect(page.locator("nav.temas")).toBeHidden();
          await page.locator("#ajustes-tema").selectOption("wifi");
          await page.waitForURL((u) => u.pathname === "/ajustes/wifi");
        } else {
          await page.goto("/ajustes/wifi");
          await asentar(page);
          await expect(page.locator("#ajustes-tema")).toBeHidden();
          await expect(page.locator("nav.temas a[aria-current=page]")).toHaveAttribute(
            "href",
            "/ajustes/wifi",
          );
        }
        const red5 = tarjeta(page, t.red5);
        await expect(red5).toBeVisible();
        await asentar(page);
        await sinDesbordamiento(page, `wifi ${idioma} ${ancho}`);
        const antes = (await red5.locator(".ajuste-fila dd").first().innerText()).trim();
        expect(antes.length).toBeGreaterThan(0);

        // 2. Cambiar el nombre.
        await red5.getByRole("button", { name: t.editar }).click();
        const nombre = red5.getByLabel(t.nombre, { exact: true });
        await expect(nombre).toBeFocused();
        const nuevo = `prueba-e2e-${idioma}-${ancho}`;
        await nombre.fill(nuevo);
        const guardar = red5.getByRole("button", { name: t.guardar });
        await guardar.click();

        // 3. El resumen: «antes → después», sin nombres de la API; el foco
        // en su título.
        const dialogo = page.locator("dialog[open]");
        await expect(dialogo).toBeVisible();
        await expect(dialogo.locator("#grupo-titulo")).toHaveText(t.revisar);
        await expect(dialogo.locator("#grupo-titulo")).toBeFocused();
        const fila = dialogo.locator("tbody tr");
        await expect(fila).toHaveCount(1);
        await expect(fila.locator("th")).toHaveText(t.nombre);
        await expect(fila.locator("td").nth(0)).toHaveText(antes);
        await expect(fila.locator("td").nth(1)).toHaveText(nuevo);
        sinApi(await dialogo.innerText());
        await sinDesbordamiento(page, `diálogo ${idioma} ${ancho}`);
        await page.screenshot({ path: `capturas/ajustes-dialogo-${idioma}-${ancho}.png` });

        if (aplicaDeVerdad) {
          // 4. Aplicar: mientras confirma, Escape no cierra; luego «aplicado»
          // y deshacer.
          await page.route(
            "**/api/confirmar_ajuste",
            async (r) => {
              await new Promise((ok) => setTimeout(ok, 1_500));
              await r.continue();
            },
            { times: 1 },
          );
          await dialogo.getByRole("button", { name: t.aplicar }).click();
          await page.keyboard.press("Escape");
          await expect(dialogo).toBeVisible();
          await expect(dialogo.getByText(t.hecho)).toBeVisible({ timeout: 200_000 });
          await dialogo.getByRole("button", { name: t.deshacer }).click();
          await expect(dialogo.locator("#grupo-titulo")).toHaveText(t.volver);
          await expect(dialogo.locator("#grupo-titulo")).toBeFocused();
          await expect(dialogo.locator("tbody td").nth(0)).toHaveText(nuevo);
          await expect(dialogo.locator("tbody td").nth(1)).toHaveText(antes);
          await dialogo.getByRole("button", { name: t.deshacer }).click();
          await expect(dialogo.getByText(t.hecho)).toBeVisible({ timeout: 200_000 });
          await dialogo.getByRole("button", { name: t.cerrar }).click();
          await expect(page.locator("dialog[open]")).toHaveCount(0);
          // La página relee el tema: vuelve el nombre de antes y el foco va
          // al título de la tarjeta.
          await expect(red5.locator(".ajuste-fila dd").first()).toHaveText(antes);
          await expect(red5.getByRole("heading", { level: 2 })).toBeFocused();
        } else {
          // Cancelar: el foco vuelve a «Guardar» y nada se escribe.
          await dialogo.getByRole("button", { name: t.cancelar }).click();
          await expect(page.locator("dialog[open]")).toHaveCount(0);
          await expect(guardar).toBeFocused();
          await red5.getByRole("button", { name: t.cancelar }).click();
          await expect(red5.locator(".ajuste-fila dd").first()).toHaveText(antes);
        }

        // 5. «Más opciones» abre y cierra.
        const mas = page.locator("details.mas-opciones");
        const avanzada = tarjeta(page, t.red5Avanzada);
        await expect(avanzada).toBeHidden();
        await mas.locator(":scope > summary").click();
        await expect(avanzada).toBeVisible();
        await sinDesbordamiento(page, `más opciones ${idioma} ${ancho}`);
        await page.screenshot({
          path: `capturas/ajustes-wifi-${idioma}-${ancho}.png`,
          fullPage: true,
        });
        await mas.locator(":scope > summary").click();
        await expect(avanzada).toBeHidden();

        // 6. Las pestañas (o el selector en móvil) llevan a otro tema.
        if (ancho < 640) {
          await page.locator("#ajustes-tema").selectOption("puertos");
        } else {
          await page.locator("nav.temas").getByRole("link", { name: t.puertos }).click();
        }
        await page.waitForURL((u) => u.pathname === "/ajustes/puertos");
        await expect(page.locator("article.ajuste").first()).toBeVisible();
        await sinDesbordamiento(page, `puertos ${idioma} ${ancho}`);
        expect(errores, "errores o avisos en la consola").toEqual([]);
      });
    });
  }
}
