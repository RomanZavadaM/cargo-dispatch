# cargo-dispatch

[🇺🇦 Українська](../../README.md) · [🇬🇧 English](README.en.md) · [🇫🇷 Français](README.fr.md) · [🇩🇪 Deutsch](README.de.md) · [🇪🇸 Español](README.es.md) · **🇰🇷 한국어** · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

> **상태:** 현재 개발 중입니다. 현재 테스트 프리릴리스는 **v0.1.0-preview.3**이며, 아직 안정적인 완성 제품은 없습니다.

## 제품 소개

**cargo-dispatch**는 소형 트럭 차량군(총중량 7.5톤 이하)을 위한 배차 시스템입니다. 시장 화물을 선택하고, 운행을 구성·재계획하고, 실행 단계로 전달하며, 배차 담당자가 실제 진행 상황을 관리할 수 있도록 하는 것을 목표로 합니다.

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

## Driver Browser

Driver Browser는 전화용과 데스크톱용으로 나뉜 별도 제품이 아니라 **하나의 제품**입니다. 세 가지 기본 레이아웃을 사용합니다.

- 좁은 휴대폰;
- 태블릿;
- 넓은 데스크톱/노트북.

데이터, 경로 동작, 파일 및 backend는 동일하고 화면 크기에 따라 배치만 달라집니다.

## 지도

Control과 Driver Browser에는 운행 지점과 마지막으로 승인된 운전자 위치를 보여 주는 실제 인터랙티브 기본 지도가 있습니다.

다만 아직 **완전한 도로 경로 형상**은 아닙니다. Valhalla를 이용한 실제 경로 형상 통합은 남아 있습니다.

## 테스트 빌드

현재 프리릴리스: [v0.1.0-preview.3](https://github.com/RomanZavadaM/cargo-dispatch/releases/tag/v0.1.0-preview.3).

Windows ZIP에는 Control, 합성 활성 운행 데이터와 로컬 loopback 검사용 샘플 PDF가 포함됩니다.

이 빌드는 **테스트/프리릴리스**이며 안정판이 아닙니다.

## 앞으로 필요한 작업

- 로컬 브라우저 수동 검수;
- 동일한 Driver Browser를 실제 휴대폰에서 같은 LAN으로 테스트;
- HTTPS/public addressing 경로;
- HTTPS를 통한 브라우저 GPS 전체 검수;
- 서로 다른 네트워크/NAT/CGNAT 현장 테스트;
- 실제 Valhalla 도로 경로 형상;
- 향후 native Driver App;
- 안정판 릴리스.

## 인터페이스 언어

현재 UI에서 구현 및 검증된 언어는 **우크라이나어, 영어, 독일어**입니다. 그 밖의 README 번역은 문서용 또는 향후 목표 언어이며 현재 프로그램 UI 지원을 의미하지 않습니다.

## 문서

공개 상태 진입점: [START_HERE.md](../../START_HERE.md), [HANDOFF_CURRENT.md](../../HANDOFF_CURRENT.md), [PROJECT_STATE.md](../../PROJECT_STATE.md).

상세 작업 문서는 비공개로 관리됩니다.

## 라이선스

독점 소프트웨어 © 2026 Roman Zavada. 저장소가 공개되어 있어도 오픈소스 권리가 부여되지 않습니다. [LICENSE.md](../../LICENSE.md) 및 [COPYRIGHT.md](../../COPYRIGHT.md)를 참조하세요.
