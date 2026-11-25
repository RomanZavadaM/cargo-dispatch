# Cargo Dispatch Driver — Android 기본 구조

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · [🇪🇸 Español](README.es.md) · **🇰🇷 한국어** · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

첫 네이티브 Android 단계입니다. **Driver Browser를 대체하지 않으며** 별도의 서버 프로토콜을 만들지 않습니다. 동일한 Control API를 사용합니다.

현재 범위: Kotlin + Jetpack Compose, Android 17/API 37, Driver URL 연결, active trip snapshot, complete/reorder, route geometry 모델, reconnect/error 상태, protocol/unit test, uk/en/de/fr/es/ko/ja/zh-CN 현지화.

Android Emulator에서는 호스트의 `127.0.0.1`/localhost가 `10.0.2.2`로 매핑됩니다. 평문 HTTP는 로컬 debug에서만 허용하며 공개 경로는 HTTPS를 사용합니다.

아직 미구현: 카메라/사진/동영상, 문서 열기/공유, 백그라운드 GPS, 오프라인 큐, 안전한 세션 저장, push, production signing.
