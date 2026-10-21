import { expect, type Page } from "@playwright/test";

// Cambios en el router (el simulador) por las server functions, para las
// pruebas que necesitan uno en curso o recién aplicado sin pasar por la
// interfaz.

/** Llama a la server function `nombre` con la sesión de la página. */
async function api(page: Page, nombre: string, cuerpo: unknown) {
  const r = await page.request.post(`/api/${nombre}`, { data: cuerpo });
  expect(r.status(), `${nombre}: ${await r.text()}`).toBe(200);
  return r.json();
}

/**
 * Cambia la apertura automática de puertos (UPnP) al valor contrario: un
 * cambio de riesgo bajo, que se vigila RP_E2E_VIGILANCIA_S. Devuelve su id.
 * Otra llamada lo deja como estaba.
 */
export async function cambiarUpnp(page: Page): Promise<number> {
  const tema = await api(page, "leer_tema", { tema: "puertos" });
  const upnp = tema.ajustes.find((a: { id: string }) => a.id === "puertos.upnp");
  const activo = upnp.campos.find((c: { id: string }) => c.id === "activo").valor.Booleano;
  const grupo = await api(page, "preparar_ajuste", {
    p: { ajuste: "puertos.upnp", instancia: null, borrar: false, valores: { activo: String(!activo) } },
  });
  return api(page, "confirmar_ajuste", { token: grupo.token, desmarcados: [] });
}

/** Estado del cambio `id` («Vigilando», «Aplicado»…). */
export async function estadoCambio(page: Page, id: number): Promise<string> {
  const r = await page.request.post("/api/estado_cambio", { data: { id } });
  return (await r.json()).estado;
}

/** Espera a que el cambio `id` termine. */
export async function esperarFinal(page: Page, id: number) {
  await expect
    .poll(() => estadoCambio(page, id), { timeout: 60_000, intervals: [1_000] })
    .toBe("Aplicado");
}
