#!/usr/bin/env python3
"""Mark phase3a sources as complete in phase3a_state.json."""
import json
import sys
from datetime import datetime

STATE_PATH = "xlate/phase3a_state.json"

with open(STATE_PATH) as f:
    state = json.load(f)

now = datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%S")
sources = sys.argv[1:]

for s in sources:
    state[s] = {"at": now, "changed": True, "status": "ok"}

with open(STATE_PATH, "w") as f:
    json.dump(state, f, indent=2)
    f.write("\n")

print(f"Marked {len(sources)} source(s) as done: {', '.join(sources)}")
