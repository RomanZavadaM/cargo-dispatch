# cargo-dispatch

[🇺🇦 Українська](../../README.md) · [🇬🇧 English](README.en.md) · [🇫🇷 Français](README.fr.md) · [🇩🇪 Deutsch](README.de.md) · [🇪🇸 Español](README.es.md) · **🇰🇷 한국어** · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

> **상태:** 활발히 개발 중입니다. 현재 안정 버전은 **v0.1.0**입니다. 현재 테스트 prerelease는 **v0.1.0-preview.7**이며 Windows, macOS, Linux용 별도 Planner가 포함됩니다. 8개 언어 전체 현지화가 통합되어 있습니다.

## 제품 소개

**cargo-dispatch**는 화물 운송 차량군을 위한 배차 시스템입니다. 화물을 선택하고, 운행을 구성·재계획하고, 실행 단계로 전달하며, 배차 담당자가 실제 진행 상황을 관리할 수 있도록 하는 것을 목표로 합니다.

주요 개발 언어는 **Rust**입니다.

## 현재 구현된 기능

- Tauri/Leptos 기반의 별도 데스크톱 **Control** 애플리케이션;
- Tauri/Leptos 기반의 별도 데스크톱 **Planner**: 수동 경로 초안, 지점 추가/삭제/재정렬, 내비게이션 좌표, Planner core revision 관리;
- 활성 운행의 지속적 로컬 저장 및 재시작 후 복구;
- Planner와 Control 사이의 버전 관리된 trip package;
- 휴대폰·태블릿·노트북·데스크톱 브라우저용 하나의 적응형 **Driver Browser**;
- Control과 Driver Browser 양쪽에서 열린 지점 순서 변경;
- PDF 및 일반 파일 **Control → Driver Browser**;
- 파일·사진·동영상 **Driver Browser → Control** 및 로컬 저장;
- 하나의 운행에 대해 두 개의 동시 브라우저 클라이언트와 별도 브라우저 DB 없는 재연결;
- Control과 Driver Browser의 인터랙티브 OpenStreetMap 지도;
- 테스트 및 로컬 작업을 위한 loopback 및 명시적 LAN 모드.

## 네이티브 Driver App

두 네이티브 클라이언트가 이미 `main`에 통합되어 있습니다.

- **Android** — Jetpack Compose, 경로/완료/재정렬, 파일/사진/동영상, foreground GPS, 세션 복구, native MapLibre + OpenStreetMap, 현재 road geometry, 지점/최신 위치, 시스템 사진·동영상 촬영; Emulator build/install/launch/screenshot PASS.
- **iPhone/iPad** — SwiftUI, 경로/완료/재정렬, 파일/사진/동영상, foreground CoreLocation, 세션 복구, native MapKit, 현재 geometry, 지점/최신 위치, 시스템 사진·동영상 촬영; Simulator build/install/launch/screenshot PASS.

두 앱 모두 동일한 Control API를 사용하며 별도 운행 데이터베이스를 만들지 않습니다.

r4/r5 이후 두 native client 모두 위치와 미디어용 durable retry queue를 갖습니다. Android r5는 foreground location service, 지속적인 Stop 알림, 15초 / 25m 정책을 사용하는 명시적 Start/Stop 백그라운드 운행 추적을 추가합니다. Apple r5는 명시적 Start/Stop CoreLocation 백그라운드 추적, `UIBackgroundModes=location`, 25m 필터를 추가하며 별도 필요가 없으면 `Always` 권한을 요청하지 않습니다.

## Driver Browser

Driver Browser는 전화용과 데스크톱용으로 나뉜 별도 제품이 아니라 **하나의 제품**입니다. 세 가지 기본 레이아웃을 사용합니다.

- 좁은 휴대폰;
- 태블릿;
- 넓은 데스크톱/노트북.

데이터, 경로 동작, 파일 및 backend는 동일하고 화면 크기에 따라 배치만 달라집니다.

## 지도

Control과 Driver Browser에는 운행 지점과 마지막으로 승인된 운전자 위치를 보여 주는 실제 인터랙티브 기본 지도가 있습니다.

`main`에는 이제 버전 관리되는 도로 geometry 파이프라인(schema 1.3)이 통합되어 있습니다. Planner는 파생 geometry와 실제 차량 routing profile을 함께 전달하며 schema 1.0–1.2 패키지도 계속 읽을 수 있습니다. Control은 geometry를 저장하고 지점 순서가 바뀌면 설정된 runtime Valhalla를 통해 도로 선을 다시 계산합니다. 늦게 도착한 routing 응답은 더 최신 운행 상태를 덮어쓸 수 없습니다.

Valhalla 주소, 지도 데이터셋 ID, timeout은 Desktop Control에서 직접 설정하며 `/status`로 사용 가능 여부를 확인하거나 routing을 명시적으로 끌 수 있습니다. Valhalla가 설정되지 않았거나 일시적으로 사용할 수 없어도 새 지점 순서는 Control에 저장되고 파생 geometry만 없을 수 있습니다.

## Test v0.1.0-preview.7

[Test v0.1.0-preview.7 열기](https://github.com/RomanZavadaM/cargo-dispatch/releases/tag/v0.1.0-preview.7).

이 버전은 Stable이 아닌 **prerelease**입니다. 하나의 정확한 소스에서 다른 제품 구성요소와 함께 별도 데스크톱 Planner를 게시하는 첫 통합 preview입니다.

Release 구성:
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

첫 Planner slice는 수동 경로 초안과 지점 편집을 지원합니다. preview.7에서는 Planner 초안이 아직 재시작 후 유지되지 않으며, 이것이 다음 단계입니다.

## Stable v0.1.0

[Stable v0.1.0 열기](https://github.com/RomanZavadaM/cargo-dispatch/releases/tag/v0.1.0).

Stable은 하나의 정확한 소스에서 다시 빌드되며 다음을 게시합니다.
- Windows Control x64 ZIP;
- macOS Control universal ZIP — Intel + Apple Silicon;
- Linux Control x64 tar.gz;
- 테스트 설치용 Android Driver debug APK;
- iOS/iPadOS Driver Simulator ZIP;
- Docker/Compose가 포함된 Linux Addressing Gateway x64 패키지;
- SHA256SUMS.

**Linux Control**은 운영자용 데스크톱 애플리케이션입니다. **Linux Addressing Gateway**는 별도의 서버 relay 모듈이며 서로 다른 패키지입니다.

Android와 Apple 패키지는 아직 스토어용 서명이 되어 있지 않습니다. App Store/TestFlight/Google Play와 production signing은 이후 별도로 처리합니다.

## 앞으로 필요한 작업

- Planner 초안의 재시작 간 영구 저장, 이후 지도/routing preview와 package export 준비;

- 로컬 브라우저 수동 검수;
- 동일한 Driver Browser를 실제 휴대폰에서 같은 LAN으로 테스트;
- HTTPS/public addressing 경로;
- HTTPS를 통한 브라우저 GPS 전체 검수;
- 서로 다른 네트워크/NAT/CGNAT 현장 테스트;


## 인터페이스 언어

Control, Planner, Driver Browser, Android Driver, iOS/iPadOS Driver는 모두 동일한 **8개 언어 세트: uk, en, de, fr, es, ko, ja, zh-Hans/zh-CN**로 구현·검증되었습니다. Control과 Driver Browser는 시스템 언어와 저장되는 수동 선택을 지원하며 CI가 8개 현지화의 완전성을 검사합니다.

## 문서

공개 상태 진입점: [START_HERE.md](../../START_HERE.md), [HANDOFF_CURRENT.md](../../HANDOFF_CURRENT.md), [PROJECT_STATE.md](../../PROJECT_STATE.md).

상세 작업 문서는 비공개로 관리됩니다.

## 라이선스

독점 소프트웨어 © 2026 Roman Zavada. 저장소가 공개되어 있어도 오픈소스 권리가 부여되지 않습니다. [LICENSE.md](../../LICENSE.md) 및 [COPYRIGHT.md](../../COPYRIGHT.md)를 참조하세요.
