# Cargo Dispatch Driver — iOS/iPadOS 기본 구조

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · [🇪🇸 Español](README.es.md) · **🇰🇷 한국어** · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

첫 네이티브 Apple 단계입니다. 하나의 SwiftUI 앱 타깃이 iPhone과 iPad를 지원하며 Driver Browser 및 Android 앱과 **동일한 Control API**를 사용합니다.

현재 범위: Swift 6 + SwiftUI + URLSession, iOS/iPadOS 17+, Driver URL, trip snapshot, complete/reorder, 영구 point ID + local revision, optional route geometry, 순수 Swift core test, uk/en/de/fr/es/ko/ja/zh-Hans 현지화.

로컬 HTTP는 Simulator/LAN debug에서만 허용하며 Release는 HTTPS-first입니다.

아직 미구현: 네이티브 지도, 카메라/사진/동영상, 문서 열기/공유, 백그라운드 GPS, 오프라인 큐, 안전한 세션 저장, push, production signing/TestFlight.
