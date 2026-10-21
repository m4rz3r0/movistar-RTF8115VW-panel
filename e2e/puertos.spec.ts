import { expect, type Locator, type Page, test } from "@playwright/test";
import { asentar, entrar, recogerErrores, sinDesbordamiento } from "./util";

// Puertos abiertos (/ajustes/puertos) a 360, 768, 1024, 1280 y 1440 px, en
// español y en inglés, contra router-sim:
// - la lista de reglas con el aviso de la regla cuyo dispositivo de destino
//   ya no está en la red, sin que el nombre ni el aviso se queden sin sitio
//   a ningún ancho;
// - «Abrir un puerto», que dice junto al campo por qué no vale (el 8080 TCP
//   ya lo abre esa regla; un puerto del dispositivo que no existe, con su
//   plegable abierto) sin enviar nada;
// - «TCP y UDP»: el resumen con un paso «TCP» y otro «UDP», que solo enseña
//   lo que cambia.
//
// Aplicar se hace en una sola combinación (inglés, 1440 px, la última):
// cada aplicación se vigila (30 s en producción; en el e2e,
// RP_E2E_VIGILANCIA_S). Se edita la regla de las fixtures (al
// puerto 8081), se abre el 8080 TCP a host-2, aparece en la lista, se
// comprueba «Activa» con dos reglas, se borra y la regla de las fixtures
// vuelve a su puerto. Las demás combinaciones abren el resumen y lo cancelan.

const textos = {
  es: {
    titulo: "Puertos abiertos",
    abrir: "Abrir un puerto",
    nombre: "Nombre",
    protocolo: "Protocolo",
    puerto: "Puerto de internet",
    dispositivo: "Dispositivo",
    otroPuerto: "El dispositivo usa otro puerto",
    interno: "Puerto en el dispositivo",
    guardar: "Guardar",
    cancelar: "Cancelar",
    aplicar: "Aplicar",
    cerrar: "Cerrar",
    revisar: "Revisa el cambio",
    hecho: "Hecho: el router tiene ya el cambio.",
    perdido: "El dispositivo de destino ya no está en la red",
    solapa: "Ese puerto ya está abierto en otra regla con el mismo protocolo",
    pasoIgual: "Igual que la regla TCP, pero con el protocolo UDP.",
    nombreNuevo: "Servidor web",
    editarDesc1: "Editar la regla desc-1",
    activaDesc1: "Regla desc-1 activa",
    activaWeb: "Regla Servidor web activa",
    borrarWeb: "Borrar la regla Servidor web",
  },
  en: {
    titulo: "Open ports",
    abrir: "Open a port",
    nombre: "Name",
    protocolo: "Protocol",
    puerto: "Internet port",
    dispositivo: "Device",
    otroPuerto: "The device uses a different port",
    interno: "Port on the device",
    guardar: "Save",
    cancelar: "Cancel",
    aplicar: "Apply",
    cerrar: "Close",
    revisar: "Review the change",
    hecho: "Done: the router now has the change.",
    perdido: "The destination device is no longer on the network",
    solapa: "That port is already open in another rule with the same protocol",
    pasoIgual: "Same as the TCP rule, but with the UDP protocol.",
    nombreNuevo: "Web server",
    editarDesc1: "Edit the rule desc-1",
    activaDesc1: "Rule desc-1 active",
    activaWeb: "Rule Web server active",
    borrarWeb: "Delete the rule Web server",
  },
} as const;

type Textos = (typeof textos)["es" | "en"];

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
async function aplicar(page: Page, t: Textos) {
  const dialogo = page.locator("dialog[open]");
  await dialogo.getByRole("button", { name: t.aplicar }).click();
  await expect(dialogo.getByText(t.hecho)).toBeVisible({ timeout: 60_000 });
  await dialogo.getByRole("button", { name: t.cerrar }).click();
  await expect(page.locator("dialog[open]")).toHaveCount(0);
}

/** Edita el puerto de internet de desc-1 y lo aplica. */
async function puertoDeDesc1(page: Page, reglas: Locator, t: Textos, puerto: string) {
  await reglas.getByRole("button", { name: t.editarDesc1 }).click();
  const form = reglas.locator("form.regla-formulario");
  await form.getByLabel(t.puerto, { exact: true }).fill(puerto);
  await form.getByRole("button", { name: t.guardar }).click();
  const dialogo = page.locator("dialog[open]");
  await expect(dialogo).toContainText(`${t.titulo}: desc-1`);
  // Solo cambian los puertos de internet (y los del dispositivo, que los
  // siguen): ni el nombre ni el destino.
  await expect(dialogo).not.toContainText("192.168.1.75");
  await aplicar(page, t);
  await expect(reglas.locator("li.regla").filter({ hasText: "desc-1" })).toContainText(puerto);
}

for (const idioma of ["es", "en"] as const) {
  for (const ancho of [360, 768, 1024, 1280, 1440]) {
    const aplicaDeVerdad = idioma === "en" && ancho === 1440;
    test.describe(`puertos ${idioma} a ${ancho} px`, () => {
      test.use({
        locale: idioma === "es" ? "es-ES" : "en-GB",
        viewport: { width: ancho, height: 900 },
      });

      test("puertos abiertos", async ({ page }) => {
        if (aplicaDeVerdad) test.setTimeout(180_000);
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
        // El nombre y el aviso tienen sitio: nada se parte letra a letra.
        const quien = await desc1.locator(".regla-quien").boundingBox();
        expect(quien?.width ?? 0, `nombre y aviso a ${ancho} px`).toBeGreaterThan(150);
        const aviso = await desc1.locator(".insignia").boundingBox();
        expect(aviso?.height ?? 0, `aviso a ${ancho} px`).toBeLessThan(80);
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
        await nombre.fill(t.nombreNuevo);
        await form.getByLabel(t.protocolo, { exact: true }).selectOption("tcp");
        const puerto = form.getByLabel(t.puerto, { exact: true });
        await puerto.fill("8080");
        await form.getByLabel(t.dispositivo, { exact: true }).selectOption("192.168.1.62");
        let preparado = false;
        page.on("request", (r) => {
          if (r.url().endsWith("/api/preparar_ajuste")) preparado = true;
        });
        const guardar = form.getByRole("button", { name: t.guardar });
        await guardar.click();
        await expect(form.getByText(t.solapa)).toBeVisible();
        await expect(puerto).toBeFocused();
        await expect(puerto).toHaveAttribute("aria-invalid", "true");
        expect(preparado).toBe(false);
        await sinDesbordamiento(page, `formulario ${idioma} ${ancho}`);
        sinApi(await form.innerText());

        if (aplicaDeVerdad) {
          await form.getByRole("button", { name: t.cancelar }).click();
          await expect(abrir).toBeFocused();
          // «Editar»: desc-1 pasa al 8081 (deja libre el 8080).
          await puertoDeDesc1(page, reglas, t, "8081");
          // Ahora sí: 8080 TCP a host-2.
          await reglas.getByRole("button", { name: t.abrir }).click();
          await form.getByLabel(t.nombre, { exact: true }).fill(t.nombreNuevo);
          await form.getByLabel(t.puerto, { exact: true }).fill("8080");
          await form.getByLabel(t.dispositivo, { exact: true }).selectOption("192.168.1.62");
          await form.getByRole("button", { name: t.guardar }).click();
          const dialogo = page.locator("dialog[open]");
          await expect(dialogo.locator("#grupo-titulo")).toHaveText(t.revisar);
          await expect(dialogo).toContainText(`${t.titulo}: ${t.nombreNuevo}`);
          await expect(dialogo.getByText("192.168.1.62")).toBeVisible();
          sinApi(await dialogo.innerText());
          await aplicar(page, t);
          // La regla nueva, en la lista, hacia host-2 y sin aviso.
          const nueva = reglas.locator("li.regla").filter({ hasText: t.nombreNuevo });
          await expect(nueva).toHaveCount(1);
          await expect(nueva).toContainText("8080");
          await expect(nueva).toContainText("host-2");
          await expect(nueva.getByText(t.perdido)).toHaveCount(0);
          await expect(reglas.getByRole("heading", { level: 2 })).toBeFocused();

          // «Activa» con dos reglas: mientras se prepara una, el interruptor
          // de la otra no se puede tocar; al cancelar, todo vuelve a estar
          // como en el router.
          await page.route("**/api/preparar_ajuste", async (ruta) => {
            await new Promise((r) => setTimeout(r, 1_500));
            await ruta.continue();
          });
          const activaWeb = reglas.getByRole("switch", { name: t.activaWeb });
          const activaDesc1 = reglas.getByRole("switch", { name: t.activaDesc1 });
          await activaWeb.click();
          await expect(activaDesc1).toBeDisabled();
          await expect(page.locator("dialog[open]")).toBeVisible();
          await page.unroute("**/api/preparar_ajuste");
          await page.locator("dialog[open]").getByRole("button", { name: t.cancelar }).click();
          await expect(activaWeb).toBeChecked();
          await expect(activaDesc1).toBeChecked();
          await expect(activaDesc1).toBeEnabled();

          // Borrarla y dejar desc-1 como estaba.
          await reglas.getByRole("button", { name: t.borrarWeb }).click();
          await expect(page.locator("dialog[open]")).toBeVisible();
          await aplicar(page, t);
          await expect(reglas.locator("li.regla").filter({ hasText: t.nombreNuevo })).toHaveCount(0);
          await puertoDeDesc1(page, reglas, t, "8080");
        } else {
          // Un puerto del dispositivo que no existe, con su plegable
          // cerrado: el plegable se abre para enseñar el error.
          await puerto.fill("9000");
          const plegable = form.locator("details.regla-interno");
          await plegable.locator("summary").click();
          const interno = form.getByLabel(t.interno, { exact: true });
          await interno.fill("70000");
          await plegable.locator("summary").click();
          await expect(plegable).not.toHaveAttribute("open", "");
          await guardar.click();
          await expect(plegable).toHaveAttribute("open", "");
          await expect(interno).toBeFocused();
          await expect(interno).toHaveAttribute("aria-invalid", "true");
          expect(preparado).toBe(false);
          await interno.fill("");

          // «TCP y UDP»: un paso «TCP» y otro «UDP» que solo enseña lo
          // que cambia.
          await form.getByLabel(t.protocolo, { exact: true }).selectOption("ambos");
          await guardar.click();
          const dialogo = page.locator("dialog[open]");
          await expect(dialogo).toBeVisible();
          await expect(dialogo.locator("#grupo-titulo")).toHaveText(t.revisar);
          await expect(dialogo).toContainText(`${t.titulo}: ${t.nombreNuevo}`);
          const pasos = dialogo.locator("fieldset.grupo-paso");
          await expect(pasos).toHaveCount(2);
          await expect(pasos.nth(0).locator("legend label")).toHaveText("TCP");
          await expect(pasos.nth(1).locator("legend label")).toHaveText("UDP");
          await expect(pasos.nth(1)).toContainText(t.pasoIgual);
          await expect(pasos.nth(1).locator("tbody tr")).toHaveCount(1);
          await expect(pasos.nth(0).getByText("192.168.1.62")).toBeVisible();
          sinApi(await dialogo.innerText());
          await sinDesbordamiento(page, `diálogo ${idioma} ${ancho}`);
          await page.screenshot({ path: `capturas/puertos-dialogo-${idioma}-${ancho}.png` });
          await dialogo.getByRole("button", { name: t.cancelar }).click();
          await expect(page.locator("dialog[open]")).toHaveCount(0);
        }
        expect(errores).toEqual([]);
      });
    });
  }
}
