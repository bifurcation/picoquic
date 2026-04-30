#!/usr/bin/env python3
"""Run Phase 0 end-to-end.

Steps:
  1. Confirm build/compile_commands.json exists (or hint at how to make it).
     This file is produced by cmake and lives in the cmake build dir;
     translation artifacts go in xlate/ instead.
  2. inventory.py        -> xlate/inventory.json
  3. call_graph.py       -> xlate/call_graph.json
  4. ifdef_scan.py       -> xlate/ifdef_manifest.json
  5. dashboard.py        -> xlate/dashboard.html
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
SCRIPTS = REPO_ROOT / "scripts"
CC = REPO_ROOT / "build" / "compile_commands.json"


def step(name: str, *args: str) -> None:
    print(f"\n=== {name} ===")
    res = subprocess.run([sys.executable, *args], cwd=REPO_ROOT)
    if res.returncode != 0:
        sys.exit(res.returncode)


def main() -> int:
    if not CC.exists():
        print(
            f"build/compile_commands.json not found.\n"
            f"Run:\n"
            f"  mkdir -p build && cd build && "
            f"cmake -DPICOQUIC_FETCH_PTLS=Y "
            f"-DCMAKE_EXPORT_COMPILE_COMMANDS=ON ..\n",
            file=sys.stderr,
        )
        return 1
    step("inventory",   str(SCRIPTS / "inventory.py"))
    step("call graph",  str(SCRIPTS / "call_graph.py"))
    step("ifdef scan",  str(SCRIPTS / "ifdef_scan.py"))
    step("dashboard",   str(SCRIPTS / "dashboard.py"))
    print("\nPhase 0 done.")
    print(f"  open {(REPO_ROOT / 'xlate' / 'dashboard.html').relative_to(REPO_ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
