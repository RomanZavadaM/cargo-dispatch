# Cargo Dispatch Driver — Android

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · [🇪🇸 Español](README.es.md) · **🇰🇷 한국어** · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

네이티브 Android 운전자 클라이언트입니다. **Driver Browser를 대체하지 않으며** 별도의 서버 프로토콜을 만들지 않습니다. 동일한 Control API와 운행 규칙을 사용합니다.

현재 범위: Kotlin + Jetpack Compose, Android 16/API 36(minSdk 26), Driver URL, Emulator의 `10.0.2.2` host loopback, active-trip snapshot, complete/reorder, 선택적 road geometry, foreground one-shot 위치 전송, Control→Android 문서 다운로드/네이티브 저장, 시스템 picker를 통한 Android→Control 파일/사진/동영상 업로드, 마지막 Driver URL/Emulator 모드 복구, uk/en/de/fr/es/ko/ja/zh-CN 현지화, protocol/unit test와 Emulator install/launch/screenshot CI.

평문 HTTP는 로컬 debug에서만 허용하며 공개 경로는 HTTPS-first입니다.

다음 작업: native map, 카메라 촬영, background GPS, 실제 synthetic Control trip 연결 smoke, offline queue/retry, Keystore 보안 강화, push, production signing.
