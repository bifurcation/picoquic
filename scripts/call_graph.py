#!/usr/bin/env python3
"""Derive the call graph from inventory.json.

Produces build/call_graph.json with:
  * adjacency lists (forward and reverse)
  * strongly-connected components (Tarjan)
  * for each function, height = shortest path in the *condensed* DAG to a leaf
    (a leaf is a node with no outgoing edges in the condensation).
    Functions inside the same SCC share the SCC's height.
  * mutually-recursive groups (SCCs with size > 1)
  * unresolved callees: names that appear as callees but for which we have
    no definition in scope (typically libc / picotls / OS APIs). Also
    written so the translator knows what crates/sys to bind.

Per TRANSLATE_PLAN.md, Phase 3 translates tests in order of the maximum
height of any function reachable from each test, ascending.
"""

from __future__ import annotations

import json
import sys
from collections import defaultdict, deque
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
INV_PATH = REPO_ROOT / "xlate" / "inventory.json"
OUT_PATH = REPO_ROOT / "xlate" / "call_graph.json"


def tarjan_scc(nodes: list[str], adj: dict[str, list[str]]) -> list[list[str]]:
    """Iterative Tarjan SCC. Returns SCCs in reverse-topological order."""
    index_of: dict[str, int] = {}
    lowlink: dict[str, int] = {}
    on_stack: set[str] = set()
    stack: list[str] = []
    sccs: list[list[str]] = []
    counter = [0]

    def strongconnect(v: str) -> None:
        # Iterative variant to handle deep call trees without blowing
        # Python's recursion limit.
        work: list[tuple[str, int]] = [(v, 0)]
        call_stack: list[tuple[str, int]] = []
        while work:
            node, i = work.pop()
            if i == 0:
                index_of[node] = counter[0]
                lowlink[node] = counter[0]
                counter[0] += 1
                stack.append(node)
                on_stack.add(node)
            recurse = False
            children = adj.get(node, [])
            for j in range(i, len(children)):
                w = children[j]
                if w not in index_of:
                    work.append((node, j + 1))
                    work.append((w, 0))
                    call_stack.append((node, w))
                    recurse = True
                    break
                if w in on_stack:
                    lowlink[node] = min(lowlink[node], index_of[w])
            if recurse:
                continue
            # Post-children: propagate lowlinks back up.
            while call_stack and call_stack[-1][0] == node:
                _, w = call_stack.pop()
                if w in lowlink:
                    lowlink[node] = min(lowlink[node], lowlink[w])
            if lowlink[node] == index_of[node]:
                scc: list[str] = []
                while True:
                    w = stack.pop()
                    on_stack.discard(w)
                    scc.append(w)
                    if w == node:
                        break
                sccs.append(scc)

    for n in nodes:
        if n not in index_of:
            strongconnect(n)
    return sccs


def main() -> int:
    inv = json.loads(INV_PATH.read_text())
    edges: list[tuple[str, str]] = [(c, e) for c, e in inv["call_edges"]]
    # Functions defined in scope.
    defined: set[str] = set()
    decl_only: set[str] = set()
    for f in inv["files"]:
        for d in f["decls"]:
            if d["kind"] == "function":
                if d.get("is_definition"):
                    defined.add(d["name"])
                else:
                    decl_only.add(d["name"])

    # Forward adjacency, restricted to in-scope callees so heights mean
    # something. Out-of-scope callees (libc, picotls, system) are recorded
    # separately as "external".
    adj: dict[str, list[str]] = defaultdict(list)
    radj: dict[str, list[str]] = defaultdict(list)
    external_calls: dict[str, set[str]] = defaultdict(set)
    callees_per_caller: dict[str, set[str]] = defaultdict(set)
    for caller, callee in edges:
        callees_per_caller[caller].add(callee)
        if callee in defined:
            adj[caller].append(callee)
            radj[callee].append(caller)
        else:
            external_calls[caller].add(callee)

    # Dedupe adjacency lists.
    for n in list(adj):
        adj[n] = sorted(set(adj[n]))
    for n in list(radj):
        radj[n] = sorted(set(radj[n]))

    nodes = sorted(defined)
    sccs = tarjan_scc(nodes, adj)
    # Map node -> SCC index.
    scc_of: dict[str, int] = {}
    for i, scc in enumerate(sccs):
        for n in scc:
            scc_of[n] = i

    # Condensed DAG: scc i -> set of scc j (i != j) for any (u in i, v in j)
    # with u->v.
    cdag: dict[int, set[int]] = defaultdict(set)
    cdag_rev: dict[int, set[int]] = defaultdict(set)
    for u, vs in adj.items():
        i = scc_of[u]
        for v in vs:
            j = scc_of[v]
            if i != j:
                cdag[i].add(j)
                cdag_rev[j].add(i)

    # Heights in condensation: leaves (no outgoing edges in cdag) are 0;
    # everyone else is 1 + min(height(child)).
    # We BFS from leaves through reverse condensation.
    height_scc: dict[int, int] = {}
    queue: deque[int] = deque()
    for i in range(len(sccs)):
        if not cdag.get(i):
            height_scc[i] = 0
            queue.append(i)
    while queue:
        i = queue.popleft()
        for parent in cdag_rev.get(i, ()):
            cand = height_scc[i] + 1
            if parent not in height_scc or cand < height_scc[parent]:
                height_scc[parent] = cand
                queue.append(parent)
    # Any SCC unreachable from a leaf has no path to a leaf — shouldn't
    # happen in a real C codebase (ultimate leaves include externs),
    # but be defensive.
    for i in range(len(sccs)):
        height_scc.setdefault(i, -1)

    height_of: dict[str, int] = {
        n: height_scc[scc_of[n]] for n in nodes
    }

    mutually_recursive = [
        sorted(scc) for scc in sccs if len(scc) > 1
    ]

    # External (out-of-scope) callees, aggregated.
    external_targets: dict[str, int] = defaultdict(int)
    for callers in external_calls.values():
        for t in callers:
            external_targets[t] += 1

    out = {
        "metadata": {
            "function_count": len(defined),
            "decl_only_count": len(decl_only),
            "scc_count": len(sccs),
            "mutually_recursive_groups": len(mutually_recursive),
            "edge_count": sum(len(v) for v in adj.values()),
            "external_call_targets": len(external_targets),
        },
        "adjacency": {n: adj.get(n, []) for n in nodes},
        "reverse_adjacency": {n: radj.get(n, []) for n in nodes},
        "scc_of": scc_of,
        "sccs": sccs,
        "height_of": height_of,
        "mutually_recursive": mutually_recursive,
        "external_callees_by_caller": {
            k: sorted(v) for k, v in external_calls.items()
        },
        "external_call_target_counts": dict(
            sorted(external_targets.items(), key=lambda x: -x[1])
        ),
    }
    OUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    OUT_PATH.write_text(json.dumps(out, indent=2) + "\n")
    print(f"Wrote {OUT_PATH}")
    print(f"  functions in scope:    {len(defined)}")
    print(f"  forward decls only:    {len(decl_only)}")
    print(f"  edges (in-scope):      {sum(len(v) for v in adj.values())}")
    print(f"  SCCs:                  {len(sccs)}")
    print(f"  mutually-rec groups:   {len(mutually_recursive)}")
    if mutually_recursive:
        for g in mutually_recursive[:5]:
            print(f"    - {g}")
        if len(mutually_recursive) > 5:
            print(f"    ... and {len(mutually_recursive) - 5} more")
    print(f"  external call targets: {len(external_targets)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
