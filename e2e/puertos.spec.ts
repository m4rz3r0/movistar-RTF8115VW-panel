import { expect, type Locator, type Page, test } from "@playwright/test";
import { asentar, entrar, recogerErrores, sinDesbordamiento } from "./util";

// Puertos abiertos (/ajustes/puertos) a 360 y 1440 px, en español y en
// inglés, contra router-sim: la lista de reglas (apilada en móvil) con el
// aviso de la regla cuyo dispositivo de destino ya no está en la red, y el
// formulario de «Abrir un puerto», que dice junto al campo por qué no vale
// (el 8080 TCP ya lo abre esa regla) sin enviar nada.
//
// Aplicar se hace en una sola combinación (inglés, 1440 px, la última):
// cada aplicación se vigila 30 s. Se borra primero la regla sin destino
// (lo que pide su aviso), se abre el 8080 TCP a host-2, aparece en la lista
// y se borra. Las demás abren el resumen y lo cancelan.

const textos = {
  es: {
    titulo: "Puertos abiertos",
    abrir: "Abrir un puerto",
    nombre: "Nombre",
    protocolo: "Protocolo",
    puerto: "Puerto de internet",
    dispositivo: "Dispositivo",
    guardar: "Guardar",
    cancelar: "Cancelar",
    aplicar: "Aplicar",
    cerrar: "Cerrar",
    revisar: "Revisa el cambio",
    hecho: "Hecho: el router tiene ya el cambio.",
    perdido: "el dispositivo de destino ya no está en la red",
    solapa: "Ese puerto ya está abierto en otra regla con el mismo protocolo",
    borrarDesc1: "Borrar la regla desc-1",
    borrarWeb: "Borrar la regla Servidor web",
  },
  en: {
    titulo: "Open ports",
    abrir: "Open a port",
    nombre: "Name",
    protocolo: "Protocol",
    puerto: "Internet port",
    dispositivo: "Device",
    guardar: "Save",
    cancelar: "Cancel",
    aplicar: "Apply",
    cerrar: "Close",
    revisar: "Review the change",
    hecho: "Done: the router now has the change.",
    perdido: "the destination device is no longer on the network",
    solapa: "That port is already open in another rule with the same protocol",
    borrarDesc1: "Delete the rule desc-1",
    borrarWeb: "Delete the rule Web server",
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
  expect(texto).not.toMatch(/l4_protocol|ext_l4|int_l4|int_ip_addr|adm_state|nat_port/);
}

/** Aplica el diálogo abierto, espera «hecho» y lo cierra. */
async function aplicar(page: Page, t: (typeof textos)["es" | "en"]) {
  const dialogo = page.locator("dialog[open]");
  await dialogo.getByRole("button", { name: t.aplicar }).click();
  await expect(dialogo.getByText(t.hecho)).toBeVisible({ timeout: 120_000 });
  await dialogo.getByRole("button", { name: t.cerrar }).click();
  await expect(page.locator("dialog[open]")).toHaveCount(0);
}

for (const idioma of ["es", "en"] as const) {
  for (const ancho of [360, 1440]) {
    const aplicaDeVerdad = idioma === "en" && ancho === 1440;
    test.describe(`puertos ${idioma} a ${ancho} px`, () => {
      test.use({
        locale: idioma === "es" ? "es-ES" : "en-GB",
        viewport: { width: ancho, height: 900 },
      });

      test("puertos abiertos", async ({ page }) => {
        if (aplicaDeVerdad) test.setTimeout(400_000);
        const t = textos[idioma];
        const errores = recogerErrores(page);
        await entrar(page);
        await page.goto("/ajustes/puertos");
        await asentar(page);
        const reglas = tarjeta(page, t.titulo);
        await expect(reglas).toBeVisible();
        // La regla de las fixtures va a una IP que no es de nadie.
        const desc1 = reglas.locator("li.regla").filter({ hasText: "desc-1" });
        await expect(desc1.getByText(t.perdido)).toBeVisible();
        await expect(desc1).toContainText("8080");
        await expect(desc1).toContainText("TCP");
        await sinDesbordamiento(page, `puertos ${idioma} ${ancho}`);
        sinApi(await reglas.innerText());
        await page.screenshot({ path: `capturas/puertos-${idioma}-${ancho}.png`, fullPage: true });

        // 8080 TCP a host-2: ya lo abre desc-1. El error, junto al puerto,
        // sin enviar.
        const abrir = reglas.getByRole("button", { name: t.abrir });
        await abrir.click();
        const form = reglas.locator("form.regla-formulario");
        const nombre = form.getByLabel(t.nombre, { exact: true });
        await expect(nombre).toBeFocused();
        await nombre.fill(idioma === "es" ? "Servidor web" : "Web server");
        await form.getByLabel(t.protocolo, { exact: true }).selectOption("tcp");
        const puerto = form.getByLabel(t.puerto, { exact: true });
        await puerto.fill("8080");
        await form.getByLabel(t.dispositivo, { exact: true }).selectOption("192.168.1.62");
        let preparado = false;
        page.on("request", (r) => {
          if (r.url().endsWith("/api/preparar_ajuste")) preparado = true;
        });
        await form.getByRole("button", { name: t.guardar }).click();
        await expect(form.getByText(t.solapa)).toBeVisible();
        await expect(puerto).toBeFocused();
        await expect(puerto).toHaveAttribute("aria-invalid", "true");
        expect(preparado).toBe(false);
        await sinDesbordamiento(page, `formulario ${idioma} ${ancho}`);
        sinApi(await form.innerText());

        if (aplicaDeVerdad) {
          // Primero se borra la regla sin destino, como pide su aviso.
          await form.getByRole("button", { name: t.cancelar }).click();
          await expect(abrir).toBeFocused();
          await reglas.getByRole("button", { name: t.borrarDesc1 }).click();
          await expect(page.locator("dialog[open]")).toBeVisible();
          await expect(page.locator("dialog[open]")).toContainText(`${t.titulo}: desc-1`);
          await aplicar(page, t);
          await expect(reglas.locator("li.regla").filter({ hasText: "desc-1" })).toHaveCount(0);
          // Ahora sí: 8080 TCP a host-2.
          await reglas.getByRole("button", { name: t.abrir }).click();
          await form.getByLabel(t.nombre, { exact: true }).fill("Web server");
          await form.getByLabel(t.puerto, { exact: true }).fill("8080");
          await form.getByLabel(t.dispositivo, { exact: true }).selectOption("192.168.1.62");
        } else {
          await puerto.fill("9000");
        }
        await form.getByRole("button", { name: t.guardar }).click();
        const dialogo = page.locator("dialog[open]");
        await expect(dialogo).toBeVisible();
        await expect(dialogo.locator("#grupo-titulo")).toHaveText(t.revisar);
        await expect(dialogo).toContainText(`${t.titulo}: ${idioma === "es" ? "Servidor web" : "Web server"}`);
        await expect(dialogo.getByText("192.168.1.62")).toBeVisible();
        sinApi(await dialogo.innerText());
        await sinDesbordamiento(page, `diálogo ${idioma} ${ancho}`);
        await page.screenshot({ path: `capturas/puertos-dialogo-${idioma}-${ancho}.png` });

        if (aplicaDeVerdad) {
          await aplicar(page, t);
          // La regla nueva, en la lista, hacia host-2 y sin aviso.
          const nueva = reglas.locator("li.regla").filter({ hasText: "Web server" });
          await expect(nueva).toHaveCount(1);
          await expect(nueva).toContainText("8080");
          await expect(nueva).toContainText("host-2");
          await expect(nueva.getByText(t.perdido)).toHaveCount(0);
          await expect(reglas.getByRole("heading", { level: 2 })).toBeFocused();
          // Borrarla.
          await reglas.getByRole("button", { name: t.borrarWeb }).click();
          await expect(page.locator("dialog[open]")).toBeVisible();
          await aplicar(page, t);
          await expect(reglas.locator("li.regla").filter({ hasText: "Web server" })).toHaveCount(0);
        } else {
          await dialogo.getByRole("button", { name: t.cancelar }).click();
          await expect(page.locator("dialog[open]")).toHaveCount(0);
        }
        expect(errores).toEqual([]);
      });
    });
  }
}
