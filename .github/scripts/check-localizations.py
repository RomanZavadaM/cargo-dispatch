#!/usr/bin/env python3
from __future__ import annotations

import json
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
EXPECTED = ["uk", "en", "de", "fr", "es", "ko", "ja", "zh-Hans"]

ANDROID = {
    "uk": "apps/driver-android/app/src/main/res/values/strings.xml",
    "en": "apps/driver-android/app/src/main/res/values-en/strings.xml",
    "de": "apps/driver-android/app/src/main/res/values-de/strings.xml",
    "fr": "apps/driver-android/app/src/main/res/values-fr/strings.xml",
    "es": "apps/driver-android/app/src/main/res/values-es/strings.xml",
    "ko": "apps/driver-android/app/src/main/res/values-ko/strings.xml",
    "ja": "apps/driver-android/app/src/main/res/values-ja/strings.xml",
    "zh-Hans": "apps/driver-android/app/src/main/res/values-zh-rCN/strings.xml",
}

READMES = [
    "README.md",
    "docs/readme/README.en.md",
    "docs/readme/README.de.md",
    "docs/readme/README.fr.md",
    "docs/readme/README.es.md",
    "docs/readme/README.ko.md",
    "docs/readme/README.ja.md",
    "docs/readme/README.zh.md",
    "apps/driver-android/README.md",
    "apps/driver-android/README.en.md",
    "apps/driver-android/README.de.md",
    "apps/driver-android/README.fr.md",
    "apps/driver-android/README.es.md",
    "apps/driver-android/README.ko.md",
    "apps/driver-android/README.ja.md",
    "apps/driver-android/README.zh.md",
    "apps/driver-ios/README.md",
    "apps/driver-ios/README.en.md",
    "apps/driver-ios/README.de.md",
    "apps/driver-ios/README.fr.md",
    "apps/driver-ios/README.es.md",
    "apps/driver-ios/README.ko.md",
    "apps/driver-ios/README.ja.md",
    "apps/driver-ios/README.zh.md",
]


def fail(message: str) -> None:
    print(f"LOCALIZATION ERROR: {message}", file=sys.stderr)
    raise SystemExit(1)


def android_keys(path: Path) -> set[str]:
    root = ET.parse(path).getroot()
    return {
        item.attrib["name"]
        for item in root.findall("string")
        if "name" in item.attrib
    }


def check_android() -> None:
    baseline = android_keys(ROOT / ANDROID["uk"])
    if not baseline:
        fail("Android Ukrainian catalog is empty")
    for locale, relative in ANDROID.items():
        keys = android_keys(ROOT / relative)
        missing = sorted(baseline - keys)
        extra = sorted(keys - baseline)
        if missing or extra:
            fail(
                f"Android {locale}: missing={missing or '-'} extra={extra or '-'}"
            )


def check_xcstrings(relative: str) -> None:
    path = ROOT / relative
    data = json.loads(path.read_text(encoding="utf-8"))
    strings = data.get("strings", {})
    if not strings:
        fail(f"{relative} is empty")
    for key, value in strings.items():
        localizations = value.get("localizations", {})
        missing = [
            locale
            for locale in EXPECTED
            if not localizations.get(locale, {})
            .get("stringUnit", {})
            .get("value")
        ]
        if missing:
            fail(f"{relative}: {key!r} misses {missing}")


def check_control() -> None:
    catalog = (ROOT / "apps/control-ui/src/i18n.rs").read_text(encoding="utf-8")
    expected_statics = ["UK", "EN", "DE", "FR", "ES", "KO", "JA", "ZH"]
    for name in expected_statics:
        if f"static {name}: ControlStrings" not in catalog:
            fail(f"Control UI catalog missing {name}")

    index = (ROOT / "apps/control-ui/index.html").read_text(encoding="utf-8")
    if '<html lang="uk">' in index:
        fail("Control UI index is hard-coded to Ukrainian")


def check_driver_browser() -> None:
    source = (ROOT / "crates/control-web/src/lib.rs").read_text(encoding="utf-8")
    if '<html lang="uk">' in source:
        fail("Driver Browser is hard-coded to Ukrainian")
    if 'id="languageSelect"' not in source:
        fail("Driver Browser language selector is missing")

    markers = {
        "uk": r"  uk: \{(?P<body>.*?)\n  \},\n  en:",
        "en": r"  en: \{(?P<body>.*?)\n  \},\n  de:",
        "de": r"  de: \{(?P<body>.*?)\n  \},\n  fr:",
        "fr": r"  fr: \{(?P<body>.*?)\n  \},\n  es:",
        "es": r"  es: \{(?P<body>.*?)\n  \},\n  ko:",
        "ko": r"  ko: \{(?P<body>.*?)\n  \},\n  ja:",
        "ja": r"  ja: \{(?P<body>.*?)\n  \},\n  \"zh-Hans\":",
        "zh-Hans": r'  "zh-Hans": \{(?P<body>.*?)\n  \}\n\};',
    }
    locale_keys: dict[str, set[str]] = {}
    for locale, pattern in markers.items():
        match = re.search(pattern, source, flags=re.DOTALL)
        if not match:
            fail(f"Driver Browser catalog missing {locale}")
        locale_keys[locale] = set(
            re.findall(r"^    ([A-Za-z][A-Za-z0-9]*):", match.group("body"), flags=re.MULTILINE)
        )

    baseline = locale_keys["uk"]
    for locale, keys in locale_keys.items():
        missing = sorted(baseline - keys)
        extra = sorted(keys - baseline)
        if missing or extra:
            fail(
                f"Driver Browser {locale}: missing={missing or '-'} extra={extra or '-'}"
            )


def check_readmes() -> None:
    missing = [relative for relative in READMES if not (ROOT / relative).is_file()]
    if missing:
        fail(f"README translations missing: {missing}")


def main() -> None:
    check_android()
    check_xcstrings("apps/driver-ios/CargoDispatchDriver/Localizable.xcstrings")
    check_xcstrings("apps/driver-ios/CargoDispatchDriver/InfoPlist.xcstrings")
    check_control()
    check_driver_browser()
    check_readmes()
    print("Localization coverage PASS: uk/en/de/fr/es/ko/ja/zh-Hans")


if __name__ == "__main__":
    main()
