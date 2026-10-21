import { expect, type Locator, type Page, test } from "@playwright/test";
import { PLAZO_CAMBIO, plazoDeTest } from "./cambios";
import { asentar, entrar, recogerErrores, sinDesbordamiento } from "./util";

// Red local (/ajustes/red_local) a 360 y 1440 px, en español y en inglés,
// contra router-sim: los dispositivos conectados con «Fijar esta IP» y las IP
// fijas con su tabla (apilada en móvil) y el formulario de añadir, que dice
// junto al campo por qué no vale una IP (la del router).
//
// «Fijar esta IP» → aplicado → aparece en IP fijas se hace en una sola
// combinación (inglés, 1440 px, la última): cada aplicación se vigila (30 s
// en producción; en el e2e, RP_E2E_VIGILANCIA_S) y el dispositivo queda con
// su IP fija, así que después se borra (otra aplicación) para dejar el
// simulador como estaba. Las demás abren el resumen y lo cancelan.

const textos = {
  es: {
    dispositivos: "Dispositivos conectados",
    fijas: "IP fijas",
    fijar: "Fijar esta IP",
    fijarHost2: "Fijar esta IP: 192.168.1.62 (host-2)",
    yaFija: "Ya tiene IP fija",
    anadir: "Añadir IP fija",
    mac: "Dirección física (MAC)",
    ip: "Dirección IP",
    guardar: "Guardar",
    cancelar: "Cancelar",
    aplicar: "Aplicar",
    cerrar: "Cerrar",
    borrarHost2: "Borrar la IP fija de host-2",
    revisar: "Revisa el cambio",
    hecho: "Hecho: el router tiene ya el cambio.",
    ipRouter: "Esa es la IP del router: elige otra.",
  },
  en: {
    dispositivos: "Connected devices",
    fijas: "Fixed IPs",
    fijar: "Fix this IP",
    fijarHost2: "Fix this IP: 192.168.1.62 (host-2)",
    yaFija: "Already has a fixed IP",
    anadir: "Add fixed IP",
    mac: "Physical address (MAC)",
    ip: "IP address",
    guardar: "Save",
    cancelar: "Cancel",
    aplicar: "Apply",
    cerrar: "Close",
    borrarHost2: "Delete the fixed IP of host-2",
    revisar: "Review the change",
    hecho: "Done: the router now has the change.",
    ipRouter: "That's the router's IP: choose another one.",
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
  expect(texto).not.toMatch(/mac_addr|ipv4_addr|adm_state|static_id/);
}

/** Aplica el diálogo abierto, espera «hecho» y lo cierra. */
async function aplicar(page: Page, t: (typeof textos)["es" | "en"]) {
  const dialogo = page.locator("dialog[open]");
  await dialogo.getByRole("button", { name: t.aplicar }).click();
  await expect(dialogo.getByText(t.hecho)).toBeVisible({ timeout: PLAZO_CAMBIO });
  await dialogo.getByRole("button", { name: t.cerrar }).click();
  await expect(page.locator("dialog[open]")).toHaveCount(0);
}

for (const idioma of ["es", "en"] as const) {
  for (const ancho of [360, 1440]) {
    const aplicaDeVerdad = idioma === "en" && ancho === 1440;
    test.describe(`red local ${idioma} a ${ancho} px`, () => {
      test.use({
        locale: idioma === "es" ? "es-ES" : "en-GB",
        viewport: { width: ancho, height: 900 },
      });

      test("dispositivos e IP fijas", async ({ page }) => {
        // Fijar la IP y borrarla: dos cambios.
        if (aplicaDeVerdad) test.setTimeout(plazoDeTest(2));
        const t = textos[idioma];
        const errores = recogerErrores(page);
        await entrar(page);
        await page.goto("/ajustes/red_local");
        await asentar(page);
        const dispositivos = tarjeta(page, t.dispositivos);
        const fijas = tarjeta(page, t.fijas);
        await expect(dispositivos).toBeVisible();
        await expect(fijas).toBeVisible();
        await sinDesbordamiento(page, `red local ${idioma} ${ancho}`);
        // Un dispositivo con reserva no se puede fijar otra vez: lo dice una
        // etiqueta, sin botón.
        await expect(dispositivos.locator(".chip-ya-fija").first()).toContainText(t.yaFija);
        await expect(dispositivos.getByRole("button", { name: new RegExp(`^${t.yaFija}`) })).toHaveCount(0);
        sinApi(await dispositivos.innerText());
        sinApi(await fijas.innerText());
        await page.screenshot({
          path: `capturas/red_local-${idioma}-${ancho}.png`,
          fullPage: true,
        });

        // Añadir con la IP del router: el error, junto a la IP, sin enviar.
        const anadir = fijas.getByRole("button", { name: t.anadir });
        await anadir.click();
        const form = fijas.locator("form.reserva-formulario");
        const mac = form.getByLabel(t.mac, { exact: true });
        await expect(mac).toBeFocused();
        await mac.fill("02:00:00:00:00:0b");
        const ip = form.getByLabel(t.ip, { exact: true });
        await ip.fill("192.168.1.1");
        let preparado = false;
        page.on("request", (r) => {
          if (r.url().endsWith("/api/preparar_ajuste")) preparado = true;
        });
        await form.getByRole("button", { name: t.guardar }).click();
        await expect(form.getByText(t.ipRouter)).toBeVisible();
        await expect(ip).toBeFocused();
        await expect(ip).toHaveAttribute("aria-invalid", "true");
        expect(preparado).toBe(false);
        await sinDesbordamiento(page, `formulario ${idioma} ${ancho}`);
        await form.getByRole("button", { name: t.cancelar }).click();
        await expect(form).toHaveCount(0);
        await expect(anadir).toBeFocused();

        // «Fijar esta IP»: el resumen con la MAC y la IP actuales.
        // El nombre accesible empieza por lo que se ve (WCAG 2.5.3).
        const fijar = dispositivos.getByRole("button", { name: t.fijarHost2, exact: true });
        await expect(fijar).toHaveText(t.fijar);
        await fijar.click();
        const dialogo = page.locator("dialog[open]");
        await expect(dialogo).toBeVisible();
        await expect(dialogo.locator("#grupo-titulo")).toHaveText(t.revisar);
        // Nombra el dispositivo.
        await expect(dialogo).toContainText(`${t.fijas}: host-2`);
        await expect(dialogo.getByText("02:00:00:00:00:03")).toBeVisible();
        await expect(dialogo.getByText("192.168.1.62")).toBeVisible();
        sinApi(await dialogo.innerText());
        await sinDesbordamiento(page, `diálogo ${idioma} ${ancho}`);
        await page.screenshot({ path: `capturas/red_local-dialogo-${idioma}-${ancho}.png` });

        if (aplicaDeVerdad) {
          await aplicar(page, t);
          // La reserva nueva, en IP fijas; el dispositivo ya no se puede fijar.
          const fila = fijas.locator("tbody tr").filter({ hasText: "192.168.1.62" });
          await expect(fila).toHaveCount(1);
          await expect(fila).toContainText("host-2");
          await expect(fila).toContainText("02:00:00:00:00:03");
          await expect(dispositivos.getByRole("button", { name: t.fijarHost2 })).toHaveCount(0);
          await expect(
            dispositivos.locator(".chip-ya-fija").filter({ hasText: `${t.yaFija}: 192.168.1.62` }),
          ).toBeVisible();
          await expect(dispositivos.getByRole("heading", { level: 2 })).toBeFocused();
          // Borrarla deja el simulador como estaba.
          await fijas.getByRole("button", { name: t.borrarHost2 }).click();
          await expect(page.locator("dialog[open]")).toBeVisible();
          await aplicar(page, t);
          await expect(fijas.locator("tbody tr").filter({ hasText: "192.168.1.62" })).toHaveCount(0);
          await expect(dispositivos.getByRole("button", { name: t.fijarHost2 })).toBeVisible();
        } else {
          await dialogo.getByRole("button", { name: t.cancelar }).click();
          await expect(page.locator("dialog[open]")).toHaveCount(0);
          await expect(fijar).toBeFocused();
        }
        expect(errores).toEqual([]);
      });
    });
  }
}
