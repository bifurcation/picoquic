#!/usr/bin/env python3
"""Dump the parsed flag list for one TU. Sanity check for inventory.py."""

from __future__ import annotations

import json
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO_ROOT / "scripts"))

from inventory import args_for, parse_compile_commands

target = sys.argv[1]
for e in parse_compile_commands():
    if e["file"].endswith("/" + target) or e["file"].endswith(target):
        print("file:", e["file"])
        print("directory:", e["directory"])
        print("command:", e["command"])
        print()
        print("parsed args:")
        for a in args_for(e):
            print("  ", a)
        break
else:
    print("No entry for", target)
    sys.exit(1)
