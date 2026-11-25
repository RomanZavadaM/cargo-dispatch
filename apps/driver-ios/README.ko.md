# Cargo Dispatch Driver — iOS/iPadOS

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · [🇪🇸 Español](README.es.md) · **🇰🇷 한국어** · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

네이티브 Apple 운전자 클라이언트입니다. 하나의 SwiftUI target이 iPhone과 iPad를 지원하며 Driver Browser/Android 앱과 **동일한 Control API**를 사용합니다.

현재 범위: Swift 6 + SwiftUI + URLSession, iOS/iPadOS 17+, Driver URL, trip snapshot, complete/reorder, permanent point ID + local revision, optional road geometry, CoreLocation foreground one-shot 위치 전송, Control→iPhone/iPad 문서 다운로드/fileExporter 저장, fileImporter + PhotosPicker를 통한 iPhone/iPad→Control 파일/사진/동영상 업로드, 마지막 Driver URL 복구, 위치 권한 문구 현지화, uk/en/de/fr/es/ko/ja/zh-Hans, Swift protocol test 및 Simulator build/install/launch/screenshot CI.

로컬 HTTP는 Simulator/LAN Debug에서만 허용하며 Release는 HTTPS-first입니다.

다음 작업: native MapKit map, 카메라 촬영, background GPS, 실제 synthetic Control trip 연결 smoke, offline queue/retry, Keychain 보안 강화, push, production signing/TestFlight.
