#!/usr/bin/env python3
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location(
    "collect", Path(__file__).with_name("collect-third-party-licenses.py")
)
collect = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(collect)


def crate(root: Path, name: str, files: dict[str, str]) -> dict:
    folder = root / name
    folder.mkdir()
    (folder / "Cargo.toml").write_text("", encoding="utf-8")
    for relative, text in files.items():
        target = folder / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text, encoding="utf-8")
    return {
        "id": name,
        "name": name,
        "version": "1.0.0",
        "license": "MIT",
        "manifest_path": str(folder / "Cargo.toml"),
    }


class CollectTest(unittest.TestCase):
    def test_texts_are_included_and_own_crates_are_not(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            own = crate(root, "own", {"LICENSE": "OWN-TEXT"})
            theirs = crate(
                root,
                "theirs",
                {"LICENSE-MIT": "MIT-TEXT", "NOTICE": "NOTICE-TEXT", "LICENSES/extra.txt": "EXTRA-TEXT"},
            )
            metadata = root / "metadata.json"
            metadata.write_text(
                json.dumps({"workspace_members": ["own"], "packages": [own, theirs]}),
                encoding="utf-8",
            )
            output = root / "out.txt"
            self.assertEqual(collect.main(["x", str(output), str(metadata)]), 0)
            text = output.read_text(encoding="utf-8")
            for expected in ("theirs 1.0.0 (MIT)", "MIT-TEXT", "NOTICE-TEXT", "EXTRA-TEXT"):
                self.assertIn(expected, text)
            self.assertNotIn("OWN-TEXT", text)

    def test_fails_when_texts_are_mostly_missing(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            bare = crate(root, "bare", {})
            metadata = root / "metadata.json"
            metadata.write_text(
                json.dumps({"workspace_members": [], "packages": [bare]}), encoding="utf-8"
            )
            self.assertEqual(collect.main(["x", str(root / "out.txt"), str(metadata)]), 1)


if __name__ == "__main__":
    unittest.main()
