# cargo-dispatch

[🇺🇦 Українська](../../README.md) · [🇬🇧 English](README.en.md) · [🇫🇷 Français](README.fr.md) · [🇩🇪 Deutsch](README.de.md) · **🇪🇸 Español** · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

> **Estado:** desarrollo activo. La versión preliminar de prueba actual es **v0.1.0-preview.3**. Todavía no existe un producto estable y terminado.

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

Esto **todavía no es la geometría vial completa de la ruta**. La geometría real mediante Valhalla sigue pendiente.

## Versión de prueba

Prerelease actual: [v0.1.0-preview.3](https://github.com/RomanZavadaM/cargo-dispatch/releases/tag/v0.1.0-preview.3).

El ZIP de Windows contiene Control, un viaje activo sintético y un PDF de prueba para la validación local por loopback.

Es una **compilación de prueba/prerelease**, no estable.

## Qué falta

- validación manual local en navegador;
- prueba del mismo Driver Browser en un teléfono real dentro de la misma LAN;
- ruta HTTPS/direccionamiento público;
- validación completa del GPS del navegador mediante HTTPS;
- pruebas de campo en distintas redes/NAT/CGNAT;
- geometría vial real de Valhalla;
- futura Driver App nativa;
- versión estable.

## Idiomas de la interfaz

La interfaz actual implementa y verifica **ucraniano, inglés y alemán**. Las demás traducciones del README son idiomas de documentación u objetivos y todavía no significan que la interfaz del programa los admita.

## Documentación

Puntos de entrada públicos: [START_HERE.md](../../START_HERE.md), [HANDOFF_CURRENT.md](../../HANDOFF_CURRENT.md) y [PROJECT_STATE.md](../../PROJECT_STATE.md).

La documentación de trabajo detallada se mantiene de forma privada.

## Licencia

Software propietario © 2026 Roman Zavada. La visibilidad pública del repositorio no concede derechos de código abierto. Véanse [LICENSE.md](../../LICENSE.md) y [COPYRIGHT.md](../../COPYRIGHT.md).
