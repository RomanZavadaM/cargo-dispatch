# cargo-dispatch

[🇺🇦 Українська](../../README.md) · [🇬🇧 English](README.en.md) · [🇫🇷 Français](README.fr.md) · [🇩🇪 Deutsch](README.de.md) · **🇪🇸 Español** · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

> **Estado:** desarrollo activo. La versión estable actual es **v0.1.0**. El prerelease de prueba actual es **v0.1.0-preview.7**, que ya incluye un Planner separado para Windows, macOS y Linux. La localización completa en 8 idiomas está integrada.

## Acerca del producto

**cargo-dispatch** es un sistema de despacho para flotas de carga. Está pensado para seleccionar cargas, construir y recalcular viajes, entregarlos para su ejecución y permitir al despachador controlar el progreso real.

El lenguaje principal de desarrollo es **Rust**.

## Funciones ya disponibles

- aplicación de escritorio **Control** separada basada en Tauri/Leptos;
- una aplicación desktop **Planner** separada basada en Tauri/Leptos: borrador manual de ruta, añadir/eliminar/reordenar puntos, coordenadas de navegación y revisión gestionada por Planner core;
- almacenamiento local persistente del viaje activo y recuperación tras reinicio;
- paquete de viaje versionado entre Planner y Control;
- un único **Driver Browser** adaptable para teléfono, tableta, portátil y navegador de escritorio;
- cambio del orden de los puntos abiertos en Control y Driver Browser;
- PDF y archivos **Control → Driver Browser**;
- archivos, fotos y vídeos **Driver Browser → Control** con guardado local;
- dos clientes de navegador simultáneos para un mismo viaje y reconexión sin una base de datos separada del navegador;
- mapa interactivo OpenStreetMap en Control y Driver Browser;
- modos loopback y LAN explícito para pruebas y trabajo local.

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

La rama `main` ya contiene una canalización versionada de geometría vial (schema 1.3): Planner transporta la geometría derivada junto con el perfil real de rutas del vehículo, mientras que los paquetes schema 1.0–1.2 siguen siendo legibles. Control guarda la geometría y, tras un reorder, recalcula la línea vial mediante el servicio Valhalla configurado en runtime. Una respuesta tardía del motor de rutas no puede sobrescribir un estado de viaje más nuevo.

La dirección de Valhalla, el ID del conjunto de mapas y los tiempos de espera se configuran directamente en Desktop Control; la disponibilidad puede comprobarse mediante `/status` o se puede desactivar el routing de forma explícita. Si Valhalla no está configurado o está temporalmente no disponible, Control conserva igualmente el nuevo orden de puntos y la geometría derivada puede quedar ausente.

## Test v0.1.0-preview.7

[Abrir Test v0.1.0-preview.7](https://github.com/RomanZavadaM/cargo-dispatch/releases/tag/v0.1.0-preview.7).

Es un **prerelease**, no Stable. Es el primer preview integrado que publica un Planner desktop separado junto con el resto del producto desde una misma fuente exacta.

El release contiene:
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

El primer slice de Planner permite trabajar con un borrador manual de ruta y editar puntos. En preview.7, el borrador de Planner todavía no se conserva entre reinicios; ese es el siguiente paso.

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

- persistencia duradera del borrador de Planner entre reinicios, seguida de vista previa de mapa/routing y preparación del package export;

- validación manual local en navegador;
- prueba del mismo Driver Browser en un teléfono real dentro de la misma LAN;
- ruta HTTPS/direccionamiento público;
- validación completa del GPS del navegador mediante HTTPS;
- pruebas de campo en distintas redes/NAT/CGNAT;


## Idiomas de la interfaz

Control, Planner, Driver Browser, Android Driver e iOS/iPadOS Driver están implementados y verificados con el mismo **conjunto de 8 idiomas: uk, en, de, fr, es, ko, ja, zh-Hans/zh-CN**. Control y Driver Browser usan el idioma del sistema más una selección manual persistente, y el CI verifica la cobertura completa de las 8 localizaciones.

## Documentación

Puntos de entrada públicos: [START_HERE.md](../../START_HERE.md), [HANDOFF_CURRENT.md](../../HANDOFF_CURRENT.md) y [PROJECT_STATE.md](../../PROJECT_STATE.md).

La documentación de trabajo detallada se mantiene de forma privada.

## Licencia

Software propietario © 2026 Roman Zavada. La visibilidad pública del repositorio no concede derechos de código abierto. Véanse [LICENSE.md](../../LICENSE.md) y [COPYRIGHT.md](../../COPYRIGHT.md).
