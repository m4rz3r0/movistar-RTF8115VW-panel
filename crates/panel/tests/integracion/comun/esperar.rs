//! Esperas con plazo: repetir una comprobación hasta que se cumple.

use std::time::{Duration, Instant};

/// Cada cuánto se repite la comprobación.
const CADA: Duration = Duration::from_millis(10);

/// Comprueba `cond` cada 10 ms hasta que se cumple; si pasa `plazo`, falla
/// diciendo `que` era lo que se esperaba.
pub async fn esperar_hasta(plazo: Duration, que: &str, mut cond: impl AsyncFnMut() -> bool) {
    esperar_hasta_con(plazo, async || {
        if cond().await {
            Ok(())
        } else {
            Err(que.to_owned())
        }
    })
    .await
}

/// Repite `intento` cada 10 ms hasta que da `Ok` y devuelve su valor; si
/// pasa `plazo`, falla con el último `Err` (lo que falta y lo que hay).
pub async fn esperar_hasta_con<T>(
    plazo: Duration,
    mut intento: impl AsyncFnMut() -> Result<T, String>,
) -> T {
    let inicio = Instant::now();
    loop {
        match intento().await {
            Ok(v) => return v,
            Err(que) => assert!(inicio.elapsed() < plazo, "no llega en {plazo:?}: {que}"),
        }
        tokio::time::sleep(CADA).await;
    }
}
