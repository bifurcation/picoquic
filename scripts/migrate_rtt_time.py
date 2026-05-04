#!/usr/bin/env python3
"""Migrate raw u64 RTT/time fields and parameters to typed
fugit Duration / Instant.

Operates on the files listed in TARGET_FILES.  For each name in
DURATION_NAMES, rewrites:
  - `pub <name>: u64,`    -> `pub <name>: Duration,`
  - `_<name>: u64,`       -> `_<name>: Duration,`
  - `_<name>: u64$`       -> `_<name>: Duration`
  - same for `<name>: u64` (no `pub`, no underscore — rare)

Same for INSTANT_NAMES with Instant in place of Duration.

Skipped: anything that's a count (timer_losses, nb_*) and field
names that overlap with non-time fields (none currently).

Run from the rs/fq/ dir.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path


DURATION_NAMES = [
    "rtt",
    "rtt_sample",
    "rtt_variant",
    "rtt_min",
    "rtt_max",
    "rtt_min_remote",
    "rtt_filtered_min",
    "rtt_measurement",
    "max_spurious_rtt",
    "rtt_update_delta",
    "rtt_threshold_low",
    "rtt_threshold_high",
    "smoothed_rtt",
    "seed_rtt_min",
    "rtt_packet_previous_period",
    "rtt_time_previous_period",
    "max_rtt_estimate_in_period",
    "min_rtt_estimate_in_period",
    "sum_rtt_estimate_in_period",
    "max_ack_delay",
    "max_ack_delay_remote",
    "max_ack_delay_local",
    "min_ack_delay",
    "min_ack_delay_remote",
    "min_ack_delay_local",
    "last_ack_delay",
    "ack_delay_remote",
    "ack_frequency_delay_local",
    "one_way_delay",
    "one_way_delay_sample",
    "send_delay",
    "retransmit_timer",
    "keep_alive_interval",
    "idle_timeout",
    "max_idle_timeout",
    "default_handshake_timeout",
    "stateless_reset_min_interval",
    "packet_time_microsec",
    "max_reorder_delay",
    "crypto_rotation_time_guard",
    "rtt_delta",
]

INSTANT_NAMES = [
    "delivered_time_prior",
    "delivered_time_last",
    "evaluation_time",
    "latest_sent_time",
    "max_sample_acked_time",
    "max_sample_sent_time",
    "last_sender_limited_time",
    "last_cwin_blocked_time",
    "last_time_acked_data_frame_sent",
    "app_wake_time",
    "largest_sent_time",
    "last_time_stamp_received",
    "time_stamp_largest_received",
    "lost_packet_sent_time",
    "packet_time",  # method param in MinMaxRtt::hystart_test
]

TARGET_FILES = [
    Path("src/internal.rs"),
    Path("src/lib.rs"),
    Path("src/cc_common.rs"),
]


def replace_field(name: str, new_type: str, text: str) -> tuple[str, int]:
    """Replace `<name>: u64` with `<name>: <new_type>` for both
    `pub <name>` (struct field) and `_<name>` (fn param) shapes,
    word-bounded so longer names don't capture short ones.
    """
    count = 0

    pat_pub = re.compile(rf"(\bpub\s+){name}: u64\b")
    text, n = pat_pub.subn(lambda m: f"{m.group(1)}{name}: {new_type}", text)
    count += n

    pat_param = re.compile(rf"(\b_){name}: u64\b")
    text, n = pat_param.subn(lambda m: f"{m.group(1)}{name}: {new_type}", text)
    count += n

    return text, count


def replace_array_field(name: str, new_type: str, text: str) -> tuple[str, int]:
    """`pub <name>: [u64; N]` -> `pub <name>: [<new_type>; N]`."""
    pat = re.compile(rf"(\bpub\s+){name}: \[u64;")
    text, n = pat.subn(lambda m: f"{m.group(1)}{name}: [{new_type};", text)
    return text, n


def main() -> int:
    total = 0
    for path in TARGET_FILES:
        if not path.exists():
            print(f"skip: {path} (missing)", file=sys.stderr)
            continue
        text = path.read_text()
        original = text
        for name in DURATION_NAMES:
            text, n = replace_field(name, "Duration", text)
            total += n
            text, n = replace_array_field(name, "Duration", text)
            total += n
        for name in INSTANT_NAMES:
            text, n = replace_field(name, "Instant", text)
            total += n
        if text != original:
            path.write_text(text)
            print(f"wrote: {path}")
    print(f"total replacements: {total}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
