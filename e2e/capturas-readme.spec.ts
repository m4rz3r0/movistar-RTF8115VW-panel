import { expect, test } from "@playwright/test";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { asentar, entrar } from "./util";

// Capturas del README (docs/capturas/), contra router-sim con las fixtures
// neutras. Solo se ejecuta cuando se pide (ver playwright.config.ts):
//
//   npx playwright test capturas-readme
//
// Las PNG se guardan sin metadatos (solo los bloques de imagen y color), y
// después hay que revisarlas a mano y poner su sha256 en
// tools/publicacion-permitidos.txt (`binario`).

const DESTINO = join(__dirname, "..", "docs", "capturas");

const capturas = [
  {
    fichero: "estado-escritorio.png",
    ruta: "/",
    listo: "main h1, main h2",
    viewport: { width: 1440, height: 900 },
    escala: 1,
  },
  {
    fichero: "estado-movil.png",
    ruta: "/",
    listo: "main h1, main h2",
    viewport: { width: 360, height: 800 },
    escala: 2,
  },
  {
    // Lo que hoy es la configuración del router en el panel; el nombre no
    // depende de dónde esté en el menú.
    fichero: "configuracion-escritorio.png",
    ruta: "/router/conexion",
    listo: "#panel-router-url",
    viewport: { width: 1440, height: 900 },
    escala: 1,
  },
];

/** Bloques de PNG que se conservan: la imagen y su espacio de color. */
const BLOQUES_PNG = new Set(["IHDR", "PLTE", "tRNS", "sRGB", "gAMA", "cHRM", "IDAT", "IEND"]);

/** Quita de una PNG todo bloque que no sea de imagen (texto, EXIF, fecha…). */
function sinMetadatos(png: Buffer): Buffer {
  const firma = png.subarray(0, 8);
  const partes = [firma];
  let i = 8;
  while (i + 8 <= png.length) {
    const largo = png.readUInt32BE(i);
    const tipo = png.toString("latin1", i + 4, i + 8);
    const fin = i + 12 + largo;
    if (BLOQUES_PNG.has(tipo)) partes.push(png.subarray(i, fin));
    i = fin;
    if (tipo === "IEND") break;
  }
  return Buffer.concat(partes);
}

for (const c of capturas) {
  test.describe(c.fichero, () => {
    test.use({ viewport: c.viewport, deviceScaleFactor: c.escala });

    test(`captura ${c.fichero}`, async ({ page }) => {
      await entrar(page);
      await page.goto(c.ruta);
      await page.locator(c.listo).first().waitFor();
      await expect(page.getByText("Esperando el primer dato")).toHaveCount(0, {
        timeout: 20_000,
      });
      await asentar(page);
      const png = await page.screenshot({ fullPage: false });
      mkdirSync(DESTINO, { recursive: true });
      writeFileSync(join(DESTINO, c.fichero), sinMetadatos(png));
    });
  });
}
