#!/usr/bin/env python3
"""One-shot: run phase1.py:run_bindgen() for one header, print result.

Used to verify the bindgen pipeline independently of `claude -p`.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO_ROOT / "scripts"))

from phase1 import run_bindgen, bindgen_baseline_path

if len(sys.argv) != 2:
    print("usage: _bindgen_check.py <header.h>", file=sys.stderr)
    sys.exit(1)

header = sys.argv[1]
out, status = run_bindgen(header)
print(f"status: {status}")
print(f"output: {out.relative_to(REPO_ROOT)}")
print(f"size:   {out.stat().st_size} bytes")
print()
print("--- first 40 lines ---")
text = out.read_text()
for line in text.splitlines()[:40]:
    print(line)
