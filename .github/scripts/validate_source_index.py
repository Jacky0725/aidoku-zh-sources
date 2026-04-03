#!/usr/bin/env python3
import json
import sys
from pathlib import Path


def main() -> int:
    repo_root = Path(__file__).resolve().parents[2]
    public_dir = repo_root / "public"
    index_path = public_dir / "index.min.json"

    if not index_path.exists():
        print(f"error: missing index file: {index_path}")
        return 1

    try:
        data = json.loads(index_path.read_text(encoding="utf-8"))
    except Exception as exc:
        print(f"error: invalid json in {index_path}: {exc}")
        return 1

    # New format: {"name": "...", "sources": [...]}
    # Legacy format: [{...}, {...}]
    if isinstance(data, dict):
        sources = data.get("sources")
        if not isinstance(sources, list):
            print("error: top-level object must include an array field named 'sources'")
            return 1
    elif isinstance(data, list):
        sources = data
    else:
        print("error: top-level json must be an object or array")
        return 1

    errors: list[str] = []
    for i, src in enumerate(sources):
        if not isinstance(src, dict):
            errors.append(f"[{i}] source entry is not an object")
            continue

        src_id = str(src.get("id", f"index-{i}"))

        # Prefer new fields, fallback to legacy fields.
        download_rel = src.get("downloadURL")
        if not download_rel and src.get("file"):
            download_rel = f"sources/{src['file']}"

        icon_rel = src.get("iconURL")
        if not icon_rel and src.get("icon"):
            icon_rel = f"icons/{src['icon']}"

        if not download_rel:
            errors.append(f"{src_id}: missing downloadURL/file")
        else:
            download_path = public_dir / str(download_rel)
            if not download_path.exists():
                errors.append(f"{src_id}: missing package file '{download_rel}'")

        if not icon_rel:
            errors.append(f"{src_id}: missing iconURL/icon")
        else:
            icon_path = public_dir / str(icon_rel)
            if not icon_path.exists():
                errors.append(f"{src_id}: missing icon file '{icon_rel}'")

    if errors:
        print("error: source index validation failed")
        for msg in errors:
            print(f"  - {msg}")
        return 1

    print(f"ok: validated {len(sources)} source entries in {index_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

