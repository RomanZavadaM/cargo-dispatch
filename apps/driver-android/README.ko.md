# Cargo Dispatch Driver — Android

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · [🇪🇸 Español](README.es.md) · **🇰🇷 한국어** · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

네이티브 Android 운전자 클라이언트입니다. **Driver Browser를 대체하지 않으며** 별도의 서버 프로토콜을 만들지 않습니다. 동일한 Control API와 운행 규칙을 사용합니다.

현재 범위: Kotlin + Jetpack Compose, Android 16/API 36(minSdk 26), Driver URL, active-trip snapshot, complete/reorder, 현재 road geometry·지점·최신 위치를 보여 주는 native MapLibre 지도, foreground one-shot 위치, 파일/사진/동영상 전송과 시스템 카메라, 위치/미디어용 durable retry queue, 15초/25m 정책의 foreground location service를 통한 명시적 Start/Stop background tracking, Android Keystore AES/GCM으로 token을 암호화하고 비밀이 아닌 base URL/Emulator mode만 private preferences에 저장하는 안전한 session recovery, 성공적인 reconnect 후 기존 raw Driver URL 자동 마이그레이션, uk/en/de/fr/es/ko/ja/zh-CN 현지화, protocol/unit test와 실제 synthetic Control trip에 연결되는 Emulator smoke.

평문 HTTP는 로컬 debug에서만 허용하며 공개 경로는 HTTPS-first입니다.

**운전자의 일상 업무**

- 완료하지 않은 지점마다 “내비게이션으로 열기” 버튼;
- “완료로 표시”는 확인을 요청합니다;
- 새 파일은 지점 또는 운행 전체에 연결할 수 있습니다;
- 화면이 열려 있는 동안 15초마다 자동으로 갱신하고 연결 상태를 표시합니다;
- 연결이 없으면 이동 경로(최근 위치 최대 500개)를 보관했다가 순서대로 보냅니다;
- 배차 프로그램이 거부한 파일은 목록에 남아 다시 보내거나 삭제할 수 있습니다;
- 오류 메시지는 휴대전화 언어로, 내부 용어 없이 표시합니다.

다음 작업: 실제 기기 LAN/background tracking 검수, HTTPS/public/NAT/CGNAT 현장 경로, push/notifications, production signing.
