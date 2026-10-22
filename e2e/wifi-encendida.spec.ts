import { expect, type Locator, type Page, test } from "@playwright/test";
import { PLAZO_CAMBIO, plazoDeTest } from "./cambios";
import { asentar, entrar, recogerErrores, sinDesbordamiento } from "./util";

// Ajustes → WiFi dice si cada red está encendida o apagada: la insignia de
// la cabecera de cada red y, en la radio encendida sin ninguna red
// encendida, una nota. En las fixtures, como en el router real, todas las
// redes están apagadas y las radios, encendidas.
//
// En español a 1440 px se enciende la red de 2,4 GHz desde su tarjeta
// básica y se vuelve a apagar (dos cambios de verdad contra router-sim, que
// dejan el panel como estaba); en las demás combinaciones solo se mira.

const textos = {
  es: {
    red24: "Red WiFi 2,4 GHz",
    radio24: "Radio de 2,4 GHz",
    red: "Red encendida",
    si: "Sí",
    no: "No",
    encendida: "Encendida",
    apagada: "Apagada",
    nota: "La radio está encendida, pero ninguna de sus redes emite: en esta banda no hay WiFi.",
    editar: "Editar",
    guardar: "Guardar",
    aplicar: "Aplicar",
    cerrar: "Cerrar",
    hecho: "Hecho: el router tiene ya el cambio.",
  },
  en: {
    red24: "2.4 GHz WiFi network",
    radio24: "2.4 GHz radio",
    red: "Network on",
    si: "Yes",
    no: "No",
    encendida: "On",
    apagada: "Off",
    nota: "The radio is on, but none of its networks is broadcasting: there is no WiFi on this band.",
    editar: "Edit",
    guardar: "Save",
    aplicar: "Apply",
    cerrar: "Close",
    hecho: "Done: the router now has the change.",
  },
} as const;

type Textos = (typeof textos)["es" | "en"];

/** La tarjeta cuyo título es exactamente `titulo`. */
function tarjeta(page: Page, titulo: string): Locator {
  return page.locator("article.ajuste").filter({
    has: page.getByRole("heading", { level: 2, name: titulo, exact: true }),
  });
}

/** Desde la tarjeta básica, enciende (`encender`) o apaga la red con su interruptor. */
async function ponerRed(page: Page, t: Textos, red: Locator, encender: boolean, de: string, a: string) {
  await red.getByRole("button", { name: t.editar }).click();
  const control = red.getByRole("switch", { name: t.red, exact: true });
  await expect(control).toBeFocused();
  await control.setChecked(encender);
  await red.getByRole("button", { name: t.guardar }).click();
  const dialogo = page.locator("dialog[open]");
  const fila = dialogo.locator("tbody tr");
  await expect(fila).toHaveCount(1);
  await expect(fila.locator("th")).toHaveText(t.red);
  await expect(fila.locator("td").nth(0)).toHaveText(de);
  await expect(fila.locator("td").nth(1)).toHaveText(a);
  await dialogo.getByRole("button", { name: t.aplicar }).click();
  await expect(dialogo.getByText(t.hecho)).toBeVisible({ timeout: PLAZO_CAMBIO });
  await dialogo.getByRole("button", { name: t.cerrar }).click();
  await expect(page.locator("dialog[open]")).toHaveCount(0);
  // La página relee el tema y lleva el foco al título de la tarjeta.
  await expect(red.getByRole("heading", { level: 2 })).toBeFocused();
}

test.describe.configure({ mode: "parallel" });

for (const idioma of ["es", "en"] as const) {
  for (const ancho of [360, 1440]) {
    const cambia = idioma === "es" && ancho === 1440;
    test.describe(`WiFi encendida o apagada ${idioma} a ${ancho} px`, () => {
      test.use({
        locale: idioma === "es" ? "es-ES" : "en-GB",
        viewport: { width: ancho, height: 900 },
      });

      test("cada red dice si está encendida", async ({ page }) => {
        if (cambia) test.setTimeout(plazoDeTest(2));
        const t = textos[idioma];
        const errores = recogerErrores(page);
        await entrar(page);
        await page.goto("/ajustes/wifi");
        await asentar(page);

        const red24 = tarjeta(page, t.red24);
        const insignia = red24.locator(".ajuste-cabecera .emision-red");
        const radio24 = tarjeta(page, t.radio24);
        const nota = radio24.getByText(t.nota, { exact: true });
        await expect(insignia).toHaveText(t.apagada);
        await expect(nota).toBeVisible();
        // Encender o apagar la red es la primera fila de la tarjeta.
        await expect(red24.locator(".ajuste-resumen .ajuste-fila").first().locator("dt")).toHaveText(
          t.red,
        );
        await sinDesbordamiento(page, `wifi ${idioma} ${ancho}`);
        if (idioma === "es") {
          await red24.screenshot({ path: `capturas/w1-red-apagada-${idioma}-${ancho}.png` });
          await radio24.screenshot({ path: `capturas/w1-radio-sin-redes-${idioma}-${ancho}.png` });
        }

        if (cambia) {
          await ponerRed(page, t, red24, true, t.no, t.si);
          await expect(insignia).toHaveText(t.encendida);
          await expect(nota).toHaveCount(0);
          await ponerRed(page, t, red24, false, t.si, t.no);
          await expect(insignia).toHaveText(t.apagada);
          await expect(nota).toBeVisible();
        }
        expect(errores, "errores o avisos en la consola").toEqual([]);
      });
    });
  }
}
