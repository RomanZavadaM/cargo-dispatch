# cargo-dispatch

[🇺🇦 Українська](../../README.md) · [🇬🇧 English](README.en.md) · [🇫🇷 Français](README.fr.md) · [🇩🇪 Deutsch](README.de.md) · [🇪🇸 Español](README.es.md) · **🇰🇷 한국어** · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

> **상태:** 활발히 개발 중입니다. 현재 안정 버전은 **v0.1.0**입니다. 전체 현지화가 끝나면 다음 큰 작업은 경로 Planner입니다.

## 제품 소개

**cargo-dispatch**는 화물 운송 차량군을 위한 배차 시스템입니다. 화물을 선택하고, 운행을 구성·재계획하고, 실행 단계로 전달하며, 배차 담당자가 실제 진행 상황을 관리할 수 있도록 하는 것을 목표로 합니다.

주요 개발 언어는 **Rust**입니다.

## 현재 구현된 기능

- Tauri/Leptos 기반의 별도 데스크톱 **Control** 애플리케이션;
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

`main`에는 이제 버전 관리되는 도로 경로 형상 파이프라인(schema 1.2)이 통합되어 있습니다. Planner가 파생 geometry를 전달할 수 있고 Control이 이를 저장하며, 두 지도는 현재 지점 순서와 일치할 때만 도로 선을 표시합니다. 순서를 바꾸면 이전 선은 즉시 무효화됩니다.

아직 남은 것은 **Valhalla로의 실제 HTTP transport와 geometry 자동 재계산**입니다. 계약과 표시 계층은 준비되어 있지만 외부 라우팅 엔진은 아직 product runtime에 연결되지 않았습니다.

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

- Windows, macOS, Linux용 Planner/editor 데스크톱 UI 및 릴리스 빌드;

- 로컬 브라우저 수동 검수;
- 동일한 Driver Browser를 실제 휴대폰에서 같은 LAN으로 테스트;
- HTTPS/public addressing 경로;
- HTTPS를 통한 브라우저 GPS 전체 검수;
- 서로 다른 네트워크/NAT/CGNAT 현장 테스트;
- 실제 Valhalla HTTP transport 및 순서 변경 후 geometry 재계산;


## 인터페이스 언어

현재 통합된 desktop/browser UI는 **우크라이나어, 영어, 독일어**를 구현·검증합니다. 새 Android 및 iOS/iPadOS native client는 첫 화면부터 **uk, en, de, fr, es, ko, ja, zh-Hans/zh-CN** 언어팩을 포함하며 문서도 같은 언어 집합으로 유지됩니다.

## 문서

공개 상태 진입점: [START_HERE.md](../../START_HERE.md), [HANDOFF_CURRENT.md](../../HANDOFF_CURRENT.md), [PROJECT_STATE.md](../../PROJECT_STATE.md).

상세 작업 문서는 비공개로 관리됩니다.

## 라이선스

독점 소프트웨어 © 2026 Roman Zavada. 저장소가 공개되어 있어도 오픈소스 권리가 부여되지 않습니다. [LICENSE.md](../../LICENSE.md) 및 [COPYRIGHT.md](../../COPYRIGHT.md)를 참조하세요.
