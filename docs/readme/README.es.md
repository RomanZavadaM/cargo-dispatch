# cargo-dispatch

[🇺🇦 Українська](../../README.md) · [🇬🇧 English](README.en.md) · [🇫🇷 Français](README.fr.md) · [🇩🇪 Deutsch](README.de.md) · **🇪🇸 Español** · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

> **Estado:** desarrollo activo. La versión preliminar de prueba actual es **v0.1.0-preview.4**. Todavía no existe un producto estable y terminado.

## Acerca del producto

**cargo-dispatch** es un sistema de despacho para una flota de camiones pequeños (masa total de hasta 7,5 t). Está pensado para seleccionar cargas, construir y recalcular viajes, entregarlos para su ejecución y permitir al despachador controlar el progreso real.

El lenguaje principal de desarrollo es **Rust**.

## Funciones ya disponibles

- aplicación de escritorio **Control** separada basada en Tauri/Leptos;
- almacenamiento local persistente del viaje activo y recuperación tras reinicio;
- paquete de viaje versionado entre Planner y Control;
- un único **Driver Browser** adaptable para teléfono, tableta, portátil y navegador de escritorio;
- cambio del orden de los puntos abiertos en Control y Driver Browser;
- PDF y archivos **Control → Driver Browser**;
- archivos, fotos y vídeos **Driver Browser → Control** con guardado local;
- dos clientes de navegador simultáneos para un mismo viaje y reconexión sin una base de datos separada del navegador;
- mapa interactivo OpenStreetMap en Control y Driver Browser;
- modos loopback y LAN explícito para pruebas y trabajo local.

## Driver Browser

Driver Browser es **un solo producto**, no versiones separadas para teléfono y escritorio. Tiene tres diseños normales:

- teléfono estrecho;
- tableta;
- escritorio/portátil ancho.

Los datos, las acciones de ruta, los archivos y el backend son comunes; solo cambia la disposición según el tamaño de pantalla.

## Mapa

Control y Driver Browser ya incluyen un mapa base interactivo real con los puntos del viaje y la última posición aceptada del conductor.

La rama `main` ya contiene una canalización versionada de geometría vial (schema 1.2): Planner puede transportar geometría derivada, Control la guarda y ambos mapas dibujan la línea solo mientras coincide con el orden actual de las paradas. Un reorder invalida inmediatamente la línea anterior.

Todavía falta el **transporte HTTP real hacia Valhalla y el recálculo automático de la geometría**. El contrato y el renderizado ya existen; el motor de rutas externo aún no está conectado al runtime del producto.

## Versión de prueba

Prerelease actual: [v0.1.0-preview.4](https://github.com/RomanZavadaM/cargo-dispatch/releases/tag/v0.1.0-preview.4).

El ZIP de Windows contiene Control, un viaje activo sintético y un PDF de prueba para la validación local por loopback.

Es una **compilación de prueba/prerelease**, no estable. Preview.4 ya contiene el adaptador Valhalla y la canalización de geometría schema 1.2. `main` vuelve a estar por delante: Driver r2 nativo para Android e iOS/iPadOS se integró después de preview.4.

## Qué falta

- validación manual local en navegador;
- prueba del mismo Driver Browser en un teléfono real dentro de la misma LAN;
- ruta HTTPS/direccionamiento público;
- validación completa del GPS del navegador mediante HTTPS;
- pruebas de campo en distintas redes/NAT/CGNAT;
- transporte HTTP real de Valhalla y recálculo de geometría tras reorder;
- mapas nativos: MapLibre en Android y MapKit en iOS/iPadOS;\n- captura de cámara, GPS en segundo plano, smoke conectado en dispositivos virtuales, cola offline y almacenamiento seguro de sesión;
- versión estable.

## Idiomas de la interfaz

La interfaz desktop/browser integrada implementa actualmente **ucraniano, inglés y alemán**. Los nuevos clientes nativos Android e iOS/iPadOS incorporan desde el primer ekran los paquetes **uk, en, de, fr, es, ko, ja, zh-Hans/zh-CN**, y la documentación usa el mismo conjunto de idiomas.

## Documentación

Puntos de entrada públicos: [START_HERE.md](../../START_HERE.md), [HANDOFF_CURRENT.md](../../HANDOFF_CURRENT.md) y [PROJECT_STATE.md](../../PROJECT_STATE.md).

La documentación de trabajo detallada se mantiene de forma privada.

## Licencia

Software propietario © 2026 Roman Zavada. La visibilidad pública del repositorio no concede derechos de código abierto. Véanse [LICENSE.md](../../LICENSE.md) y [COPYRIGHT.md](../../COPYRIGHT.md).
