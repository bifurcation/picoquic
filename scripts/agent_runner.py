#!/usr/bin/env python3
"""Shared AI-agent invocation helpers for translation drivers.

The translation scripts historically invoked Claude Code directly.
This module keeps that path as the default while giving each driver a
single provider switch for Codex:

  python3 scripts/phase4.py --agent codex
  XLATE_AGENT=codex python3 scripts/phase4.py

Claude-specific tool allowlists are still passed to Claude.  Codex CLI
does not expose an equivalent per-invocation tool allowlist, so Codex
runs with an explicit workspace-write sandbox and a short runner note
that tells the agent to treat the allowlist as the intended action
scope.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shlex
import shutil
import subprocess
import time
from dataclasses import dataclass
from functools import lru_cache
from pathlib import Path
from typing import Any, TextIO


PROVIDERS = ("claude", "codex")


RATE_LIMIT_PATTERNS = (
    re.compile(r"\bapi[_ -]?error[_ -]?status\b[^0-9]{0,16}429"),
    re.compile(r"\bhttp(?: status)?\b[^0-9]{0,16}429"),
    re.compile(r"\bstatus(?: code)?\b[^0-9]{0,16}429"),
    re.compile(r"\b429\b.{0,80}\b(?:rate[- ]?limit(?:ed)?|quota|too many requests)\b"),
    re.compile(r"\b(?:rate[- ]?limit(?:ed)?|quota|too many requests)\b.{0,80}\b429\b"),
)


@dataclass(frozen=True)
class AgentConfig:
    provider: str
    model: str | None = None
    codex_sandbox: str = "workspace-write"
    codex_approval: str = "never"

    @property
    def label(self) -> str:
        return self.provider

    @property
    def command_name(self) -> str:
        if self.provider == "claude":
            return "claude -p"
        if self.provider == "codex":
            return "codex exec"
        return self.provider

    @property
    def model_label(self) -> str:
        return self.model or "default"


@dataclass(frozen=True)
class AgentRun:
    returncode: int
    stdout: str
    stderr: str
    summary: str
    elapsed: float
    command_display: str
    rate_limited: bool = False


def add_agent_args(
    parser: argparse.ArgumentParser,
    *,
    claude_default_model: str | None = None,
    model_help_context: str = "selected agent",
) -> None:
    """Add common `--agent` / `--model` arguments to a script parser."""
    default_agent = os.environ.get("XLATE_AGENT", "claude").lower()
    if default_agent not in PROVIDERS:
        default_agent = "claude"

    parser.add_argument(
        "--agent",
        choices=PROVIDERS,
        default=default_agent,
        help=(
            "AI agent provider to invoke (default: $XLATE_AGENT or "
            "claude)."
        ),
    )
    parser.add_argument(
        "--model",
        default=None,
        help=(
            f"Model for the {model_help_context}.  If omitted, Claude "
            f"uses {claude_default_model or 'its CLI default'} and "
            "Codex uses its CLI/config default.  Environment overrides: "
            "XLATE_AGENT_MODEL, XLATE_CLAUDE_MODEL, XLATE_CODEX_MODEL."
        ),
    )
    parser.add_argument(
        "--codex-sandbox",
        choices=("read-only", "workspace-write", "danger-full-access"),
        default=os.environ.get("XLATE_CODEX_SANDBOX", "workspace-write"),
        help=(
            "Sandbox mode passed to `codex exec` (default: "
            "$XLATE_CODEX_SANDBOX or workspace-write)."
        ),
    )
    parser.add_argument(
        "--codex-approval",
        choices=("untrusted", "on-request", "on-failure", "never"),
        default=os.environ.get("XLATE_CODEX_APPROVAL", "never"),
        help=(
            "Approval policy passed to `codex exec` (default: "
            "$XLATE_CODEX_APPROVAL or never)."
        ),
    )


def config_from_args(
    args: argparse.Namespace,
    *,
    claude_default_model: str | None = None,
) -> AgentConfig:
    provider = args.agent.lower()
    model = (
        args.model
        or os.environ.get("XLATE_AGENT_MODEL")
        or os.environ.get(f"XLATE_{provider.upper()}_MODEL")
    )
    if model is None and provider == "claude":
        model = claude_default_model
    return AgentConfig(
        provider=provider,
        model=model,
        codex_sandbox=args.codex_sandbox,
        codex_approval=args.codex_approval,
    )


def log_dir(repo_root: Path, agent: AgentConfig, *parts: str) -> Path:
    return repo_root / "xlate" / f"{agent.provider}_logs" / Path(*parts)


def provider_suffix_prompt(
    agent: AgentConfig,
    prompt: str,
    *,
    allowed_tools: str | None,
    max_turns: int,
) -> str:
    if agent.provider != "codex":
        return prompt
    note = [
        "",
        "## Codex Runner Note",
        "",
        "This invocation is running under `codex exec`, not Claude Code.",
        "Follow the task prompt and repository instructions exactly.",
        f"The script's turn budget is {max_turns}; keep the session bounded.",
    ]
    if allowed_tools:
        note.extend([
            "",
            "The legacy Claude allowed-tools scope for this pass was:",
            "",
            f"`{allowed_tools}`",
            "",
            "Treat that as the intended action scope.  Do not use shell",
            "commands or file edits outside what the prompt permits.",
        ])
    return prompt.rstrip() + "\n" + "\n".join(note) + "\n"


def _extra_args(provider: str) -> list[str]:
    raw = os.environ.get(f"XLATE_{provider.upper()}_ARGS", "")
    return shlex.split(raw) if raw else []


def _looks_rate_limited(text: str) -> bool:
    lower = text.lower()
    return any(pattern.search(lower) for pattern in RATE_LIMIT_PATTERNS)


@lru_cache(maxsize=1)
def _codex_approval_flag() -> str | None:
    try:
        res = subprocess.run(
            ["codex", "exec", "--help"],
            capture_output=True,
            text=True,
            stdin=subprocess.DEVNULL,
        )
    except OSError:
        return None
    help_text = res.stdout + res.stderr
    if "--ask-for-approval" in help_text:
        return "--ask-for-approval"
    if "--approval-policy" in help_text:
        return "--approval-policy"
    return None


def build_command(
    agent: AgentConfig,
    prompt: str,
    *,
    repo_root: Path,
    allowed_tools: str | None,
    max_turns: int,
    stream_json: bool,
) -> tuple[list[str], str]:
    """Return (argv, display_without_prompt)."""
    if agent.provider == "claude":
        cmd = ["claude", "-p", prompt]
        display = ["claude", "-p"]
        if agent.model:
            cmd.extend(["--model", agent.model])
            display.extend(["--model", agent.model])
        if allowed_tools:
            cmd.extend(["--allowedTools", allowed_tools])
            display.extend(["--allowedTools", allowed_tools])
        cmd.extend(["--max-turns", str(max_turns)])
        display.extend(["--max-turns", str(max_turns)])
        if stream_json:
            cmd.extend(["--output-format", "stream-json", "--verbose"])
            display.extend(["--output-format", "stream-json", "--verbose"])
        extra = _extra_args("claude")
        cmd.extend(extra)
        display.extend(extra)
    elif agent.provider == "codex":
        cmd = [
            "codex", "exec",
            "--cd", str(repo_root),
            "--sandbox", agent.codex_sandbox,
        ]
        approval_flag = _codex_approval_flag()
        if approval_flag is not None:
            cmd.extend([approval_flag, agent.codex_approval])
        display = list(cmd)
        if agent.model:
            cmd.extend(["--model", agent.model])
            display.extend(["--model", agent.model])
        if stream_json:
            cmd.append("--json")
            display.append("--json")
        extra = _extra_args("codex")
        cmd.extend(extra)
        display.extend(extra)
        cmd.append(prompt)
    else:
        raise ValueError(f"unknown agent provider: {agent.provider}")
    return cmd, " ".join(shlex.quote(c) for c in display)


def _missing_cli_run(agent: AgentConfig, log_path: Path, phase: str,
                     label: str) -> AgentRun:
    msg = f"{agent.provider} CLI not found on PATH"
    log_path.parent.mkdir(parents=True, exist_ok=True)
    log_path.write_text(
        f"# {agent.command_name} ({phase}) for {label}\n"
        f"# exit: 127\n\n{msg}\n"
    )
    return AgentRun(127, "", msg, msg, 0.0, agent.command_name)


def run_capture(
    agent: AgentConfig,
    prompt: str,
    *,
    repo_root: Path,
    log_path: Path,
    phase: str,
    label: str,
    prompt_file: Path | None,
    allowed_tools: str | None,
    max_turns: int,
) -> AgentRun:
    """Run an agent to completion and write stdout/stderr to `log_path`."""
    if shutil.which(agent.provider) is None:
        return _missing_cli_run(agent, log_path, phase, label)

    actual_prompt = provider_suffix_prompt(
        agent, prompt, allowed_tools=allowed_tools, max_turns=max_turns,
    )
    cmd, display = build_command(
        agent, actual_prompt, repo_root=repo_root,
        allowed_tools=allowed_tools, max_turns=max_turns,
        stream_json=False,
    )
    t0 = time.monotonic()
    res = subprocess.run(
        cmd, cwd=repo_root, capture_output=True, text=True,
        stdin=subprocess.DEVNULL,
    )
    elapsed = time.monotonic() - t0
    log_path.parent.mkdir(parents=True, exist_ok=True)
    prompt_note = (
        f"# (prompt omitted; see {prompt_file.relative_to(repo_root)})\n"
        if prompt_file is not None else "# (inline prompt omitted)\n"
    )
    log_path.write_text(
        f"# {agent.command_name} ({phase}) for {label}\n"
        f"# started: {time.strftime('%Y-%m-%dT%H:%M:%S')}\n"
        f"# elapsed: {elapsed:.1f}s, exit: {res.returncode}\n"
        f"# model: {agent.model_label}\n"
        f"# command: {display}\n"
        f"{prompt_note}\n"
        f"## stdout\n{res.stdout}\n\n## stderr\n{res.stderr}\n"
    )
    text = (res.stdout + "\n" + res.stderr).lower()
    rate_limited = _looks_rate_limited(text)
    return AgentRun(
        res.returncode, res.stdout, res.stderr, res.stdout,
        elapsed, display, rate_limited=rate_limited,
    )


def _find_string_key(obj: Any, keys: set[str]) -> str:
    if isinstance(obj, dict):
        for k, v in obj.items():
            if k in keys and isinstance(v, str):
                return v
        for v in obj.values():
            found = _find_string_key(v, keys)
            if found:
                return found
    elif isinstance(obj, list):
        for v in obj:
            found = _find_string_key(v, keys)
            if found:
                return found
    return ""


def _json_contains_rate_limit(obj: Any) -> bool:
    text = json.dumps(obj, sort_keys=True).lower()
    return _looks_rate_limited(text)


def _parse_claude_event(ev: dict[str, Any]) -> tuple[str, str, bool]:
    tool = ""
    text = ""
    rate_limited = False
    if ev.get("type") == "assistant":
        for block in ev.get("message", {}).get("content", []):
            if block.get("type") == "tool_use":
                name = block.get("name", "?")
                inp = block.get("input", {})
                label = ""
                if name in {"Read", "Edit", "Write"}:
                    label = inp.get("file_path", "")
                elif name == "Bash":
                    label = inp.get("command", "")[:80]
                elif name in {"Glob", "Grep"}:
                    label = inp.get("pattern", inp.get("query", ""))
                tool = f"{name}({label})"
            elif block.get("type") == "text":
                text = block.get("text", "")
    elif ev.get("type") == "result":
        text = ev.get("result", "") or ""
        if ev.get("api_error_status") == 429 and ev.get("is_error"):
            rate_limited = True
    return tool, text, rate_limited


def _parse_codex_event(ev: dict[str, Any]) -> tuple[str, str, bool]:
    typ = str(ev.get("type", ""))
    tool = ""
    text = ""
    if "exec" in typ or "command" in typ:
        cmd = _find_string_key(ev, {"command", "cmd"})
        if cmd:
            tool = f"Bash({cmd[:80]})"
    elif "patch" in typ:
        path = _find_string_key(ev, {"path", "file", "file_path"})
        tool = f"Patch({path})" if path else "Patch()"

    if "assistant" in typ or "message" in typ or "completed" in typ:
        text = _find_string_key(ev, {"text", "message", "content", "result"})
    return tool, text, _json_contains_rate_limit(ev)


def _parse_stream_event(
    agent: AgentConfig, line: str,
) -> tuple[str, str, bool]:
    try:
        ev = json.loads(line)
    except json.JSONDecodeError:
        lower = line.lower()
        limited = _looks_rate_limited(lower)
        if agent.provider == "claude":
            return "", "", limited
        return "", line, limited
    if agent.provider == "claude":
        return _parse_claude_event(ev)
    if agent.provider == "codex":
        return _parse_codex_event(ev)
    return "", "", _json_contains_rate_limit(ev)


def run_stream(
    agent: AgentConfig,
    prompt: str,
    *,
    repo_root: Path,
    log_path: Path,
    phase: str,
    label: str,
    prompt_file: Path,
    allowed_tools: str | None,
    max_turns: int,
) -> AgentRun:
    """Run an agent with JSONL streaming where supported.

    Returns exit 99 when the stream clearly reports an HTTP 429/rate
    limit so callers can abort without marking the item failed.
    """
    if shutil.which(agent.provider) is None:
        return _missing_cli_run(agent, log_path, phase, label)

    actual_prompt = provider_suffix_prompt(
        agent, prompt, allowed_tools=allowed_tools, max_turns=max_turns,
    )
    cmd, display = build_command(
        agent, actual_prompt, repo_root=repo_root,
        allowed_tools=allowed_tools, max_turns=max_turns,
        stream_json=True,
    )

    t0 = time.monotonic()
    log_path.parent.mkdir(parents=True, exist_ok=True)
    summary = ""
    last_text = ""
    rate_limited = False
    with log_path.open("w", buffering=1) as log_f:
        _write_stream_header(
            log_f, agent=agent, phase=phase, label=label,
            prompt_file=prompt_file, repo_root=repo_root,
            command_display=display,
        )
        proc = subprocess.Popen(
            cmd, cwd=repo_root, stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT, text=True,
            stdin=subprocess.DEVNULL, bufsize=1,
        )
        assert proc.stdout is not None
        for line in proc.stdout:
            log_f.write(line)
            stripped = line.rstrip()
            if not stripped:
                continue
            tool, text, limited = _parse_stream_event(agent, stripped)
            if tool:
                print(f"  · {tool}", flush=True)
            if text:
                last_text = text
            if limited:
                rate_limited = True
        rc = proc.wait()
        elapsed = time.monotonic() - t0
        log_f.write(f"\n# elapsed: {elapsed:.1f}s, exit: {rc}\n")
    if rate_limited:
        rc = 99
    summary = last_text
    return AgentRun(
        rc, "", "", summary, elapsed, display,
        rate_limited=rate_limited,
    )


def _write_stream_header(
    log_f: TextIO,
    *,
    agent: AgentConfig,
    phase: str,
    label: str,
    prompt_file: Path,
    repo_root: Path,
    command_display: str,
) -> None:
    log_f.write(
        f"# {agent.command_name} ({phase}) for {label}\n"
        f"# started: {time.strftime('%Y-%m-%dT%H:%M:%S')}\n"
        f"# model: {agent.model_label}\n"
        f"# command: {command_display}\n"
        f"# (prompt: {prompt_file.relative_to(repo_root)})\n\n"
    )
