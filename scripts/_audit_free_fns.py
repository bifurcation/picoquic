#!/usr/bin/env python3
"""Diagnostic: every `pub fn` whose first arg is `&[mut] <UpperT>`,
across the crate.  Lists candidates that should arguably be
methods on T."""
import re
from pathlib import Path

# Types we DON'T want to fold a free fn into a method on (slices,
# stdlib types, by-value tokens, by-value enums, parameter-bag
# types, etc.).
NON_RECEIVERS = {
    "str", "Error", "Vec", "Option", "Result", "Box",
    "SocketAddr", "Ipv4Addr", "Ipv6Addr", "IpAddr",
    "ConnectionId",          # already a method receiver via its own impl
    "Instant", "Duration",
    "PacketHeader", "PacketContext",
    "Tuple", "Packet", "Pacing", "Epoch",
    "PathToken", "ConnectionToken", "StreamToken", "PacketToken",
    "LocalConnectionIdToken", "SackItemToken", "HashToken", "SplayToken",
    "TransportParameters", "TransportParameter",
    "FrameType", "PreferredAddress", "VersionNegotiation",
    "Token", "Arena",
    "PathQuality", "PerAckState", "DatagramActive",
    "RegisteredToken",
    # tls-side traits / opaque types
    "TlsCallbacks", "Session", "ClientConfig", "ServerConfig",
    "PacketKey", "HeaderKey", "Keys", "KeyPair",
    # function-pointer trait / config types passed by value/ref but
    # not "owned" by a receiver concept
    "CongestionAlgorithm", "Config",
}

PAT = re.compile(
    r"^pub fn (?P<name>[A-Za-z_][A-Za-z0-9_]*)"
    r"\(\s*(?P<recv>_[a-z][a-z_0-9]*)\s*:\s*"
    r"&(?P<mutkw>mut\s+)?(?P<ty>[A-Za-z_][A-Za-z0-9_]+)\b",
    re.MULTILINE | re.DOTALL,
)

ROOT = Path(__file__).resolve().parent.parent / "rs" / "fq" / "src"
files = sorted(ROOT.rglob("*.rs"))
hits = []
for f in files:
    text = f.read_text()
    for m in PAT.finditer(text):
        ty = m.group("ty")
        if ty in NON_RECEIVERS:
            continue
        if ty[0].islower():
            continue
        # Compute line number.
        line = text.count("\n", 0, m.start()) + 1
        hits.append((f.relative_to(ROOT.parent.parent), line, m.group("name"),
                     m.group("recv"), bool(m.group("mutkw")), ty))

if not hits:
    print("(none — all receiver-shaped free fns folded into methods)")
else:
    for path, line, name, recv, is_mut, ty in hits:
        kind = "&mut" if is_mut else "&"
        print(f"{path}:{line}  pub fn {name}({recv}: {kind} {ty}, ...)")
