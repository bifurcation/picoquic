#!/usr/bin/env python3
"""Inspect inventory.json for malformed/error entries."""
from __future__ import annotations
import json
import sys
from pathlib import Path

inv = json.loads(Path(sys.argv[1]).read_text())
errors = []
ok = 0
for tu in inv["translation_units"]:
    if "error" in tu:
        errors.append(tu)
    elif "module" not in tu:
        errors.append({"file": tu.get("file"), "issue": "no module field"})
    else:
        ok += 1
print(f"ok: {ok}")
print(f"errors: {len(errors)}")
for e in errors[:20]:
    print(" ", e)
