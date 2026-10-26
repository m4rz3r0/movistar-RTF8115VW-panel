import { expect, type Locator, type Page, test } from "@playwright/test";
import { PLAZO_CAMBIO, plazoDeTest } from "./cambios";
import { asentar, entrar, recogerErrores, sinDesbordamiento } from "./util";
import { capturaOpcional } from "./util";

// Ajustes → WiFi dice si cada red está encendida o apagada: la insignia de
// la tarjeta de cada red y, en la radio encendida sin ninguna red
// encendida, una nota en su pie. En las fixtures, como en el router real,
// todas las redes están apagadas y las radios, encendidas.
//
// En español a 1440 px se enciende la red de 2,4 GHz con el interruptor de
// su tarjeta (sin preguntar: encender no corta nada) y se vuelve a apagar
// (apagar sí corta: lo pregunta antes); dos cambios de verdad contra
// router-sim, que dejan el panel como estaba. En las demás combinaciones
// solo se mira.

const textos = {
  es: {
    red24: "Red WiFi 2,4 GHz",
    radio24: "Radio de 2,4 GHz",
    encendida: "Encendida",
    apagada: "Apagada",
    nota: "La radio está encendida, pero ninguna de sus redes emite: en esta banda no hay WiFi.",
    aplicar: "Aplicar",
    hecho: "Hecho: el router tiene ya el cambio.",
  },
  en: {
    red24: "2.4 GHz WiFi network",
    radio24: "2.4 GHz radio",
    encendida: "On",
    apagada: "Off",
    nota: "The radio is on, but none of its networks is broadcasting: there is no WiFi on this band.",
    aplicar: "Apply",
    hecho: "Done: the router now has the change.",
  },
} as const;

type Textos = (typeof textos)["es" | "en"];

/** La tarjeta cuyo título es exactamente `titulo`. */
function tarjeta(page: Page, titulo: string): Locator {
  return page.locator("article.tarjeta-el").filter({
    has: page.getByRole("heading", { level: 2, name: titulo, exact: true }),
  });
}

/**
 * Enciende (`encender`) o apaga la red con el interruptor de su tarjeta:
 * apagarla pregunta antes; encenderla, no. Espera a que el router lo tenga.
 */
async function ponerRed(page: Page, t: Textos, red: Locator, encender: boolean) {
  await red.getByRole("switch").click();
  const dialogo = page.locator("dialog[open]");
  if (encender) {
    await expect(dialogo).toHaveCount(0);
  } else {
    const fila = dialogo.locator("tbody tr");
    await expect(fila.locator("td").nth(0)).toHaveText(t.encendida);
    await expect(fila.locator("td").nth(1)).toHaveText(t.apagada);
    await dialogo.getByRole("button", { name: t.aplicar }).click();
  }
  // El aviso es el de este cambio (el anterior ya decía «hecho»): lo sigue
  // y lo dice al terminar (tras la vigilancia).
  const aviso = page.locator(".avisos-cambio [role=status] .aviso-cambio");
  await expect(aviso).not.toContainText(t.hecho);
  await expect(aviso).toContainText(t.hecho, { timeout: PLAZO_CAMBIO });
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
        const insignia = red24.locator(".tarjeta-insignia .ins");
        const radio24 = tarjeta(page, t.radio24);
        const nota = radio24.getByText(t.nota, { exact: true });
        await expect(insignia).toHaveText(t.apagada);
        await expect(nota).toBeVisible();
        await sinDesbordamiento(page, `wifi ${idioma} ${ancho}`);
        if (idioma === "es") {
          await capturaOpcional(red24, { path: `capturas/w1-red-apagada-${idioma}-${ancho}.png` });
          await capturaOpcional(radio24, { path: `capturas/w1-radio-sin-redes-${idioma}-${ancho}.png` });
        }

        if (cambia) {
          await ponerRed(page, t, red24, true);
          await expect(insignia).toHaveText(t.encendida);
          await expect(nota).toHaveCount(0);
          await ponerRed(page, t, red24, false);
          await expect(insignia).toHaveText(t.apagada);
          await expect(nota).toBeVisible();
        }
        expect(errores, "errores o avisos en la consola").toEqual([]);
      });
    });
  }
}
