# cargo-dispatch

[🇺🇦 Українська](../../README.md) · [🇬🇧 English](README.en.md) · [🇫🇷 Français](README.fr.md) · [🇩🇪 Deutsch](README.de.md) · **🇪🇸 Español** · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

Despacho de una flota de carga: planificar un viaje, entregarlo para su ejecución, mantener contacto con el conductor y registrar lo que realmente ocurrió. El lenguaje principal de desarrollo es **Rust**.

**Versión estable: [v0.2.0](https://github.com/RomanZavadaM/cargo-dispatch/releases/tag/v0.2.0).** El desarrollo continúa; las versiones de prueba están en [Releases](https://github.com/RomanZavadaM/cargo-dispatch/releases).

## Componentes

| Programa | Quién lo usa | Plataformas |
|---|---|---|
| **Control** | despachador: lleva los viajes activos | Windows, macOS, Linux |
| **Planner** | planificador: prepara la ruta y el paquete del viaje | Windows, macOS, Linux |
| **Página del conductor** | conductor, en cualquier navegador, sin instalar nada | teléfono, tableta, ordenador |
| **Driver para Android** | conductor | Android 8.0 o posterior |
| **Driver para iPhone y iPad** | conductor | iOS / iPadOS 17 o posterior |
| **Addressing Gateway** | conexión por internet | servidor Linux |

Control es la única fuente de verdad de un viaje activo. Entrega al conductor la página, la ruta y los archivos, y recibe las marcas, las posiciones y los adjuntos. No hay base de datos central de viajes.

## Primeros pasos

1. Descargue un archivo desde la página de la versión, descomprímalo e inicie **Control**. El paquete de Control incluye datos de prueba inventados (`local-browser-test-data`).
2. **«Añadir viaje»** → «Abrir un viaje existente» (carpeta de viaje o paquete de Planner) o «Crear un viaje nuevo manualmente».
3. **«Conectar conductores»** → «Este equipo», «La misma red Wi-Fi» o «Desde cualquier lugar, por internet».
4. Entregue al conductor el enlace del bloque **«Qué hacer ahora»**: «Abrir» o «Copiar».
5. El conductor abre el enlace en un navegador o lo pega en la aplicación Driver.

## Funciones

**Control**
- varios viajes a la vez, una ventana por viaje;
- viaje de un paquete de Planner o creado a mano; el estado se guarda localmente y se recupera tras reiniciar;
- orden de los puntos abiertos, mapa, última posición del conductor, comparación entre plan y realidad;
- archivos para el conductor; archivos, fotos y vídeos del conductor;
- averigua por sí mismo la dirección del equipo en la red local; el modo de conexión cambia sin apagarlo a mano;
- mensajes de error en el idioma de la interfaz; hora de la posición, tamaño del archivo y nombre del punto en lenguaje claro.

**Planner**
- borrador de ruta: añadir, quitar y reordenar puntos por coordenadas; el borrador sobrevive a un reinicio;
- vista previa de la ruta y línea de carretera mediante su servidor Valhalla con parámetros del vehículo introducidos expresamente (lo desconocido no se inventa);
- exportación del paquete de viaje (ZIP, esquema 1.4) con la carga y la base de planificación; nunca se sobrescribe un archivo existente.

**Conductor: navegador, Android, iPhone y iPad**
- ruta y mapa, marcar un punto como hecho, reordenar puntos abiertos;
- recibir documentos; enviar archivos, fotos y vídeos;
- compartir la posición; en las aplicaciones también en segundo plano, activado y detenido expresamente, con cola de reenvío;
- marcar un punto como hecho pide confirmación; las aplicaciones ofrecen además un botón de navegador, asociar un archivo a un punto, una pantalla que se actualiza sola, la conservación del recorrido sin conexión (hasta 500 posiciones) y los archivos rechazados visibles.

## Conexión y seguridad

- Los modos **Este equipo** y **red local** no van cifrados (HTTP): úselos solo en una red de confianza. El acceso desde la red nunca se activa solo y no se abren puertos del enrutador.
- Un enlace con código secreto da acceso al viaje: compártalo solo con el conductor de ese viaje.
- Los navegadores de teléfono suelen permitir la geolocalización solo por HTTPS, es decir, a través del Gateway.
- Para que un teléfono llegue a Control en la red, permita el programa en el cortafuegos del equipo.
- **Por internet:** su propio Gateway detrás de un proxy inverso HTTPS. Control se conecta a él por sí mismo, así que no hace falta un puerto entrante en el equipo. El Gateway no guarda viajes, documentos, medios ni posiciones. Límite predeterminado: 64 MiB por solicitud (configurable hasta 512 MiB).

## Mapas y rutas

El mapa de Control, Planner, el navegador y Android es [OpenStreetMap](https://www.openstreetmap.org/copyright) (© OpenStreetMap contributors); en iPhone y iPad es Apple MapKit. El proveedor de mapas ve la dirección IP del dispositivo y la zona mostrada. La línea de carretera requiere un servidor Valhalla propio; sin él el viaje funciona, solo falta la línea.

## Idiomas

Todos los programas están en українська, English, Français, Deutsch, Español, 한국어, 日本語 y 简体中文. La CI comprueba que las traducciones estén completas; los detalles técnicos del sistema en los errores siguen en inglés.

## Aún sin verificar, y limitaciones

- Las aplicaciones para Android y iPhone/iPad no se han probado en dispositivos físicos: solo en emulador, simulador y pruebas automáticas.
- Internet móvil y el Gateway en una red real solo se han verificado automáticamente.
- Los paquetes no están firmados; Android es un APK de depuración e iOS solo una compilación para el simulador de Apple.
- Tras reiniciar Control, hay que volver a activar a mano los viajes y la conexión.
- El despachador todavía no puede editar puntos ni marcarlos como hechos.

## Compilación

Las compilaciones, pruebas y versiones se ejecutan en GitHub Actions; véase `.github/workflows`.

## Documentación

Estado del proyecto (en ucraniano): [START_HERE.md](../../START_HERE.md), [HANDOFF_CURRENT.md](../../HANDOFF_CURRENT.md), [PROJECT_STATE.md](../../PROJECT_STATE.md). La documentación de trabajo detallada se mantiene en privado.

## Licencia

Software propietario © 2026 Roman Zavada (Роман Завада). Que el repositorio sea público no concede derechos de código abierto. Véanse [LICENSE.md](../../LICENSE.md), [COPYRIGHT.md](../../COPYRIGHT.md) y [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md).
