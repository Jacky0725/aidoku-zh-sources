#!/usr/bin/env python3
import json
import subprocess
import sys
from pathlib import Path


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8-sig"))


def is_deprecated(meta: dict) -> bool:
    return bool(meta.get("deprecated")) or bool((meta.get("info") or {}).get("deprecated"))


def main() -> int:
    repo_root = Path(__file__).resolve().parents[2]
    sources_root = repo_root / "src"

    errors: list[str] = []
    checked = 0

    for source_dir in sorted(sources_root.glob("*/*")):
        meta_path = source_dir / "res" / "source.json"
        if not meta_path.exists():
            continue

        try:
            meta = load_json(meta_path)
        except Exception as exc:
            errors.append(f"{source_dir.name}: invalid source.json ({exc})")
            continue

        if is_deprecated(meta):
            continue

        package_path = source_dir / "package.aix"
        if not package_path.exists():
            errors.append(f"{source_dir.name}: missing package.aix (build step likely failed)")
            continue

        result = subprocess.run(
            ["aidoku", "verify", str(package_path)],
            cwd=repo_root,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            check=False,
        )
        checked += 1
        if result.returncode != 0:
            output = result.stdout.strip().replace("\n", " | ")
            errors.append(f"{source_dir.name}: verify failed ({output})")

    if errors:
        print("error: package verification failed")
        for err in errors:
            print(f"  - {err}")
        return 1

    print(f"ok: verified {checked} non-deprecated packages")
    return 0


if __name__ == "__main__":
    sys.exit(main())
