# cargo-dispatch

[🇺🇦 Українська](../../README.md) · [🇬🇧 English](README.en.md) · [🇫🇷 Français](README.fr.md) · [🇩🇪 Deutsch](README.de.md) · **🇪🇸 Español** · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

> **Estado:** desarrollo activo. La versión estable actual es **v0.1.0**. El prerelease de prueba actual es **v0.1.0-preview.8**, que ya incluye un Planner separado para Windows, macOS y Linux. La localización completa en 8 idiomas está integrada.

## Acerca del producto

**cargo-dispatch** es un sistema de despacho para flotas de carga. Está pensado para seleccionar cargas, construir y recalcular viajes, entregarlos para su ejecución y permitir al despachador controlar el progreso real.

El lenguaje principal de desarrollo es **Rust**.

## Funciones ya disponibles

- aplicación de escritorio **Control** separada basada en Tauri/Leptos; Control puede importar un ZIP de Planner o crear manualmente un nuevo viaje activo sin ZIP de Planner usando el mismo estado persistente del viaje;
- una aplicación desktop **Planner** separada basada en Tauri/Leptos: borrador manual de ruta, añadir/eliminar/reordenar puntos, coordenadas de navegación y revisión gestionada por Planner core; el borrador se guarda en un journal append-only versionado dentro de los datos de la aplicación y se recupera tras reiniciar;
- almacenamiento local persistente del viaje activo y recuperación tras reinicio;
- paquete de viaje versionado entre Planner y Control;
- un único **Driver Browser** adaptable para teléfono, tableta, portátil y navegador de escritorio;
- cambio del orden de los puntos abiertos en Control y Driver Browser;
- PDF y archivos **Control → Driver Browser**;
- archivos, fotos y vídeos **Driver Browser → Control** con guardado local;
- dos clientes de navegador simultáneos para un mismo viaje y reconexión sin una base de datos separada del navegador;
- mapa interactivo OpenStreetMap en Control y Driver Browser;
- modos loopback y LAN explícito para pruebas y trabajo local;
- la aceptación automatizada de Driver Browser verifica una sesión real de Control por separado mediante loopback y un bind LAN privado explícito;
- la aceptación automatizada del Gateway HTTPS público verifica la cadena real `Control ↔ public HTTPS Gateway ↔ Driver Browser`: Control solo saliente, shell/snapshot/ubicación del navegador y archivos bidireccionales;

## Driver Apps nativas

Los dos clientes nativos ya están integrados en `main`:

- **Android** — Jetpack Compose, ruta/completar/reordenar, archivos/fotos/vídeos, GPS en primer plano, recuperación de sesión, MapLibre + OpenStreetMap nativo, geometría vial actual, puntos/última posición y captura de foto/vídeo del sistema; build/install/launch/screenshot en Emulator PASS.
- **iPhone/iPad** — SwiftUI, ruta/completar/reordenar, archivos/fotos/vídeos, CoreLocation en primer plano, recuperación de sesión, MapKit nativo, geometría actual, puntos/última posición y captura de foto/vídeo del sistema; build/install/launch/screenshot en Simulator PASS.

Ambos usan la misma API de Control y no crean una base de viaje separada.

Desde r4/r5, ambos clientes nativos también tienen una cola persistente de reintentos para ubicación y multimedia. Android r5 añade seguimiento en segundo plano Start/Stop explícito mediante un servicio de ubicación en primer plano, una notificación Stop persistente y una política de 15 s / 25 m. Apple r5 añade seguimiento CoreLocation en segundo plano Start/Stop explícito, `UIBackgroundModes=location`, filtro de 25 m y no solicita permiso `Always` sin una necesidad separada.

## Driver Browser

Driver Browser es **un solo producto**, no versiones separadas para teléfono y escritorio. Tiene tres diseños normales:

- teléfono estrecho;
- tableta;
- escritorio/portátil ancho.

Los datos, las acciones de ruta, los archivos y el backend son comunes; solo cambia la disposición según el tamaño de pantalla.

## Mapa

Control y Driver Browser ya incluyen un mapa base interactivo real con los puntos del viaje y la última posición aceptada del conductor.

El `main` actual usa schema 1.4 con una base de planificación inmutable; los schemas anteriores siguen siendo legibles. Un perfil de routing de Planner solo es la entrada de un cálculo de planificación concreto y no crea una vinculación permanente entre el viaje y un vehículo. Tras la importación, Control mantiene por separado el contexto real y mutable de routing de la sesión activa.

Control conserva la geometría derivada y recalcula la línea vial mediante el servicio Valhalla runtime configurado después de un cambio de orden. Una respuesta tardía de routing no puede sobrescribir un estado de viaje más nuevo. Si Valhalla no está configurado o está temporalmente no disponible, el nuevo orden de puntos se guarda igualmente y la geometría derivada puede quedar ausente.

Planner desktop dispone ahora de su propio preview Valhalla runtime: el operador indica la dirección de Valhalla, el ID del conjunto de mapas y los tiempos de espera, comprueba `/status`, y `/route` se calcula solo con un perfil de planificación proporcionado explícitamente. No se inventan restricciones numéricas desconocidas. Un resultado correcto dibuja la polyline real de carretera sobre OSM; sin resultado no se dibuja una línea recta ficticia. El preview se deriva de ruta/revisión/perfil: cambiar el borrador o el perfil de planificación elimina la línea anterior y una respuesta tardía se rechaza. Un fallo de routing no modifica el borrador de Planner.

Planner desktop ahora también exporta el package de viaje versionado existente como schema 1.4 mediante un diálogo nativo «Guardar como». El package incluye una planning basis explícita (tipo/clase de vehículo y cargas), el orden de puntos y, cuando existe, solo el snapshot coincidente del planning routing profile y la geometría Valhalla. TripId se mantiene estable para un borrador, TripVersion continúa tras reinicios, CargoId no depende de la posición de la fila y un ZIP existente nunca se sobrescribe.

## Test v0.1.0-preview.8

[Abrir Test v0.1.0-preview.8](https://github.com/RomanZavadaM/cargo-dispatch/releases/tag/v0.1.0-preview.8).

Es un **prerelease**, no Stable. Es una compilación de prueba integrada del producto actual desde una única fuente exacta.

El release incluye:
- Windows Control x64 ZIP;
- macOS Control universal ZIP — Intel + Apple Silicon;
- Linux Control x64 tar.gz;
- Windows Planner x64 ZIP;
- macOS Planner universal ZIP — Intel + Apple Silicon;
- Linux Planner x64 tar.gz;
- Android Driver debug APK;
- iOS/iPadOS Driver Simulator ZIP;
- Linux Addressing Gateway x64 package;
- SHA256SUMS.

preview.8 ya incluye el borrador durable de Planner, la vista previa real de carretera con Valhalla, la exportación ZIP versionada schema 1.4 con planning basis y la creación manual de un viaje activo en Control sin ZIP de Planner. El perfil de Planner no es una vinculación permanente viaje→vehículo; el contexto real de vehículo/dispositivo se mantiene por separado en Control.

## Stable v0.1.0

[Abrir Stable v0.1.0](https://github.com/RomanZavadaM/cargo-dispatch/releases/tag/v0.1.0).

Stable se reconstruye desde una única fuente exacta y publica:
- Windows Control x64 ZIP;
- macOS Control universal ZIP — Intel + Apple Silicon;
- Linux Control x64 tar.gz;
- Android Driver debug APK para instalación de prueba;
- iOS/iPadOS Driver Simulator ZIP;
- paquete Linux Addressing Gateway x64 con Docker/Compose;
- SHA256SUMS.

**Linux Control** es la aplicación de escritorio del operador. **Linux Addressing Gateway** es el módulo relay de servidor independiente; son paquetes distintos.

Los paquetes Android y Apple aún no están firmados para las tiendas. App Store/TestFlight/Google Play y la firma de producción se tratarán por separado más adelante.

## Qué falta

- creación de un nuevo viaje directamente en Control cuando no haya un ZIP de Planner;

- validación visual manual del navegador local para narrow phone / tablet / wide desktop, incluido pan/zoom/fit del mapa;
- prueba del mismo Driver Browser en un teléfono real dentro de la misma LAN;
- aceptación HTTPS en dispositivo real desde una red separada y NAT/CGNAT móvil;
- validación completa del GPS del navegador mediante HTTPS en un dispositivo real;


## Idiomas de la interfaz

Control, Planner, Driver Browser, Android Driver e iOS/iPadOS Driver están implementados y verificados con el mismo **conjunto de 8 idiomas: uk, en, de, fr, es, ko, ja, zh-Hans/zh-CN**. Control y Driver Browser usan el idioma del sistema más una selección manual persistente, y el CI verifica la cobertura completa de las 8 localizaciones.

## Documentación

Puntos de entrada públicos: [START_HERE.md](../../START_HERE.md), [HANDOFF_CURRENT.md](../../HANDOFF_CURRENT.md) y [PROJECT_STATE.md](../../PROJECT_STATE.md).

La documentación de trabajo detallada se mantiene de forma privada.

## Licencia

Software propietario © 2026 Roman Zavada. La visibilidad pública del repositorio no concede derechos de código abierto. Véanse [LICENSE.md](../../LICENSE.md) y [COPYRIGHT.md](../../COPYRIGHT.md).
