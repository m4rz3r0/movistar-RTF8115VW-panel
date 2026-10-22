import { availableParallelism, tmpdir } from "node:os";
import { join } from "node:path";

// Paneles que arranca el e2e (cada uno con su simulador y su base de datos),
// en puertos propios: no chocan con una vista previa en 3000/3100/8091.
//
// - «instalado-k»: uno por worker de Playwright, con el usuario e2e y las
//   credenciales del router importadas del entorno (RP_ROUTER_*), como un
//   servidor que se actualiza; además, un grupo de dos pasos ya aplicado en
//   el Historial. Cada worker usa solo el suyo (el idioma de serie, la zona,
//   las reglas, el retardo del sim y el cambio en curso del gestor no se
//   comparten entre workers); dentro del worker los tests van en serie.
// - «nuevo-*»: recién instalados (sin usuarios ni router), uno por cada
//   pasada del asistente, que es de un solo uso. Solo los usa
//   instalar.spec.ts, que corre entero en un worker.
// - «recorrido-*»: instalados como los primeros, uno por idioma del
//   recorrido del criterio de éxito 1 (ajustes-recorrido.spec.ts). Cambia
//   cinco ajustes y no los deshace: con su propio panel, no estorba a las
//   demás pruebas (que esperan las fixtures, o el cambio de e2e_grupo en la
//   primera página del Historial). Su vigilancia es más corta
//   (VIGILANCIA_RECORRIDO_S): el recorrido no prueba la vigilancia y sus
//   cinco cambios irían, si no, en el camino crítico de la pasada.
//
// Puertos: los instalados en E2E_PORT + 0..WORKERS-1, los nuevos a
// continuación y, detrás, los del recorrido (igual con E2E_SIM_PORT para
// los simuladores).

const base = Number(process.env.E2E_PORT ?? 3195);
const baseSim = Number(process.env.E2E_SIM_PORT ?? 8195);

/** Paneles del asistente (instalar.spec.ts), detrás de los instalados. */
const NUEVOS = 4;

/** Paneles del recorrido (ajustes-recorrido.spec.ts), detrás de los del asistente. */
const RECORRIDOS = 2;

/**
 * Paneles de la rejilla de WiFi (ajustes-wifi.spec.ts), detrás de los del
 * recorrido: los cambios de verdad (un lote de dos pasos, interruptores y
 * sus deshacer) no ensucian el Historial de los instalados, que otras
 * pruebas cuentan.
 */
const REJILLAS = 2;

/**
 * Panel de las otras pestañas en rejilla (ajustes-temas.spec.ts): sus
 * guardados de verdad van en serie en él, con la vigilancia corta del
 * recorrido, sin ensuciar el Historial de los instalados.
 */
const TEMAS = 1;

/**
 * Tope de E2E_WORKERS: los instalados, los nuevos, los del recorrido, los de
 * la rejilla y el de los temas ocupan WORKERS + 9 puertos seguidos desde
 * E2E_PORT (y desde E2E_SIM_PORT), y la ventana reservada a cada prueba es
 * de 20 (p. ej. 3710-3729 y 8710-8729).
 */
const VENTANA_DE_PUERTOS = 20;
const MAX_WORKERS = VENTANA_DE_PUERTOS - NUEVOS - RECORRIDOS - REJILLAS - TEMAS;

/**
 * Workers de Playwright (E2E_WORKERS; 1, en serie). Por defecto, uno por
 * cada 3 hilos de la CPU, entre 1 y 2: cada worker mueve un Chromium, un
 * panel y un simulador, y lo que limita en un portátil (memoria, calor, lo
 * que corre a la vez) no se ve en los hilos. Con 2, el portátil tarda unos
 * 5,3 min; en una máquina dedicada, E2E_WORKERS=4 (unos 3,2 min en el
 * sobremesa). La CI (4 vCPU) sigue con 1.
 */
export const WORKERS = workers(process.env.E2E_WORKERS);

function workers(valor: string | undefined): number {
  if (valor === undefined || valor === "") {
    return Math.min(2, Math.max(1, Math.floor(availableParallelism() / 3)));
  }
  const n = Number(valor);
  if (!Number.isInteger(n) || n < 1 || n > MAX_WORKERS) {
    throw new Error(
      `E2E_WORKERS debe ser un entero entre 1 y ${MAX_WORKERS} (con ${NUEVOS} paneles del ` +
        `asistente, ${RECORRIDOS} del recorrido, ${REJILLAS} de la rejilla y ${TEMAS} de los temas, ` +
        `${VENTANA_DE_PUERTOS} puertos seguidos ` +
        `desde E2E_PORT): ${valor}`,
    );
  }
  return n;
}

/**
 * Vigilancia de cada cambio en el e2e, en segundos (RP_E2E_VIGILANCIA_S; 10
 * por defecto): la de riesgo bajo y medio; la de alto, el doble (main.rs).
 * Los plazos de los specs salen de aquí (cambios.ts).
 */
export const VIGILANCIA_S = vigilancia(process.env.RP_E2E_VIGILANCIA_S);

function vigilancia(valor: string | undefined): number {
  if (valor === undefined || valor === "") return 10;
  const n = Number(valor);
  if (!Number.isInteger(n) || n < 1) {
    throw new Error(`RP_E2E_VIGILANCIA_S debe ser un entero mayor que cero: ${valor}`);
  }
  return n;
}

/**
 * El worker de este proceso: Playwright pone TEST_PARALLEL_INDEX (0 ..
 * WORKERS-1; el worker que sustituye a uno caído hereda su índice) y vuelve
 * a evaluar la configuración en cada worker. En el proceso principal no
 * está: 0.
 */
const indice = Number(process.env.TEST_PARALLEL_INDEX ?? 0);

export interface Instancia {
  nombre: string;
  modo: "instalado" | "nuevo";
  puerto: number;
  puertoSim: number;
  /** Carpeta de la base de datos (y del token del asistente). */
  dir: string;
  /** Vigilancia de cada cambio en segundos (RP_E2E_VIGILANCIA_S). */
  vigilanciaS: number;
  /** Origen del panel (localhost: Chromium acepta allí la cookie Secure). */
  url: string;
  urlSim: string;
}

function instancia(
  nombre: string,
  modo: Instancia["modo"],
  n: number,
  vigilanciaS = VIGILANCIA_S,
): Instancia {
  const puerto = base + n;
  const puertoSim = baseSim + n;
  return {
    nombre,
    modo,
    puerto,
    puertoSim,
    dir: join(tmpdir(), `routerpanel-e2e-${puerto}`),
    vigilanciaS,
    url: `http://localhost:${puerto}`,
    urlSim: `http://127.0.0.1:${puertoSim}`,
  };
}

/** Un panel instalado por worker. */
export const instalados: Instancia[] = Array.from({ length: WORKERS }, (_, k) =>
  instancia(`instalado-${k}`, "instalado", k),
);

/** El panel instalado de este worker. */
export const instalado = instalados[indice];
if (instalado === undefined) {
  throw new Error(
    `TEST_PARALLEL_INDEX ${indice} fuera de los ${WORKERS} workers: elige cuántos con E2E_WORKERS, no con --workers`,
  );
}

/** Un panel recién instalado por cada ancho e idioma del asistente. */
export const nuevos = {
  "es-360": instancia("nuevo-es-360", "nuevo", WORKERS + 0),
  "es-1440": instancia("nuevo-es-1440", "nuevo", WORKERS + 1),
  "en-360": instancia("nuevo-en-360", "nuevo", WORKERS + 2),
  "en-1440": instancia("nuevo-en-1440", "nuevo", WORKERS + 3),
};

/** Vigilancia de los paneles del recorrido, en segundos (la de alto, el doble). */
const VIGILANCIA_RECORRIDO_S = Math.min(1, VIGILANCIA_S);

/** Un panel instalado por idioma del recorrido del criterio de éxito 1. */
export const recorridos = {
  es: instancia("recorrido-es", "instalado", WORKERS + NUEVOS + 0, VIGILANCIA_RECORRIDO_S),
  en: instancia("recorrido-en", "instalado", WORKERS + NUEVOS + 1, VIGILANCIA_RECORRIDO_S),
};

/** Un panel instalado por combinación de la rejilla que cambia de verdad. */
export const rejillas = {
  "1440-oscuro": instancia("rejilla-1440-oscuro", "instalado", WORKERS + NUEVOS + RECORRIDOS + 0),
  "360-claro": instancia("rejilla-360-claro", "instalado", WORKERS + NUEVOS + RECORRIDOS + 1),
};

/** El panel de los guardados de verdad de Internet, Seguridad, Teléfono y Sistema. */
export const temas = instancia(
  "temas",
  "instalado",
  WORKERS + NUEVOS + RECORRIDOS + REJILLAS,
  VIGILANCIA_RECORRIDO_S,
);

export const todas: Instancia[] = [
  ...instalados,
  ...Object.values(nuevos),
  ...Object.values(recorridos),
  ...Object.values(rejillas),
  temas,
];

/** Usuario del panel «instalado». */
export const USUARIO = "e2e";
export const CONTRASENA = "e2e-contrasena";
