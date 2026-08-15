"""Privacy-bounded classification and observation construction."""

from __future__ import annotations

import json
import os
import re
import sqlite3
import subprocess
from pathlib import Path
from typing import Any

from .common import *

def _quoted_or_hypothetical(text: str) -> bool:
    stripped = text.strip()
    if re.match(r"^(?:>|```|\"|“|'|‘)", stripped) and re.search(
        r"\b(?:wrong|broken|does(?:n'?t| not) work|failed|hate|dislike)\b",
        stripped,
        re.IGNORECASE,
    ):
        return True
    if re.search(
        r"\b(?:hypothetically|suppose|imagine|for example|e\.g\.|if)\b.{0,100}"
        r"\b(?:wrong|broken|does(?:n'?t| not) work|failed|hate|dislike)\b",
        stripped,
        re.IGNORECASE,
    ):
        return True
    if re.search(
        r"\b(?:translate|summarize|rewrite|quote|classify|analyze)\b.{0,60}"
        r"\b(?:this|the following|the phrase|the quote|these reviews?)\b",
        stripped,
        re.IGNORECASE,
    ):
        return True
    return False


def _third_party_sentiment_only(text: str) -> bool:
    has_interaction_target = bool(
        re.search(
            r"\b(?:you|your answer|your response|this answer|this workflow|this tool|"
            r"this command|this ui|this feature|what you|what we)\b",
            text,
            re.IGNORECASE,
        )
    )
    third_party = bool(
        re.search(
            r"\b(?:they|he|she|reviewer|customer|critic|article|movie|book|restaurant|"
            r"product review|third party)\b.{0,100}\b(?:hate|dislike|wrong|broken|bad)\b",
            text,
            re.IGNORECASE,
        )
    )
    return third_party and not has_interaction_target


def classify_prompt(prompt: str) -> dict[str, str] | None:
    """Return one primary explicit signal, or None for unsupported sentiment."""
    if not prompt.strip() or _quoted_or_hypothetical(prompt) or _third_party_sentiment_only(prompt):
        return None
    text = " ".join(prompt.casefold().split())
    correction = re.search(
        r"\b(?:you(?:'re| are| were) wrong|that(?:'s| is| was) (?:wrong|incorrect)|"
        r"your (?:answer|response) is (?:wrong|incorrect)|you (?:missed|ignored)|i (?:said|asked for) .{0,80} not|"
        r"no,? (?:i meant|that is not|you need to)|let me correct you)\b",
        text,
    )
    failure = re.search(
        r"\b(?:does(?:n'?t| not) work|isn'?t working|not working|is broken|broke|"
        r"failed again|keeps? failing|can(?:'t|not) (?:use|run|open|build|load|save|submit))\b",
        text,
    )
    dissatisfaction = re.search(
        r"\b(?:i (?:don'?t|do not) like|i hate|i(?:'m| am) (?:unhappy|dissatisfied|frustrated)|"
        r"this is (?:bad|awful|annoying|frustrating|not what i wanted)|not what i (?:asked for|wanted))\b",
        text,
    )
    if correction:
        kind = "boundary_correction" if re.search(r"\b(?:boundary|scope|path)\b", text) else "correction"
        mode = "boundary_not_preserved" if kind == "boundary_correction" else "incorrect_output"
    elif failure:
        kind, mode = "failure_report", "reported_not_working"
    elif dissatisfaction:
        kind, mode = "dissatisfaction", "explicit_dissatisfaction"
    else:
        return None
    target = classify_target(text)
    return {
        "signal_kind": kind,
        "target_kind": target,
        "failure_mode": mode,
        "summary": {
            "correction": "User explicitly corrected the current interaction outcome.",
            "boundary_correction": "User explicitly corrected a scope or file-boundary violation.",
            "failure_report": "User explicitly reported that the current outcome does not work.",
            "dissatisfaction": "User explicitly expressed dissatisfaction with the current outcome.",
        }[kind],
        "phenomenon_key": normalize_phenomenon(prompt),
    }


def classify_target(text: str) -> str:
    if re.search(r"\b(?:preference|prefer|taste|remember me|model me)\b", text):
        return "user_preference_model"
    if re.search(r"\b(?:tool|command|cli|hook|terminal|api|browser)\b", text):
        return "tool"
    if re.search(r"\b(?:repo|repository|codebase|branch|commit)\b", text):
        return "repository"
    if re.search(r"\b(?:ui|screen|page|button|artifact|document|image|design|feature)\b", text):
        return "artifact"
    if re.search(r"\b(?:workflow|process|step|handoff|automation)\b", text):
        return "workflow"
    return "agent_answer"


def _git_root(cwd: str) -> Path | None:
    try:
        result = subprocess.run(
            ["/usr/bin/git", "-C", cwd, "rev-parse", "--show-toplevel"],
            capture_output=True,
            text=True,
            timeout=0.7,
            check=False,
        )
        if result.returncode == 0 and result.stdout.strip():
            return Path(result.stdout.strip()).resolve()
    except (OSError, subprocess.SubprocessError):
        return None
    return None


def _git_remote(root: Path) -> str | None:
    try:
        result = subprocess.run(
            ["/usr/bin/git", "-C", str(root), "remote", "get-url", "origin"],
            capture_output=True,
            text=True,
            timeout=0.7,
            check=False,
        )
        return result.stdout.strip() if result.returncode == 0 else None
    except (OSError, subprocess.SubprocessError):
        return None


def resolve_scope(cwd: str) -> tuple[str, str]:
    root = _git_root(cwd)
    if root is None:
        if cwd and Path(cwd).expanduser() != Path.home():
            return f"project:v1:{stable_key(str(Path(cwd).resolve()))}", "project"
        return "global-agent", "global"
    database = Path(os.environ.get("PAPERCUTS_PRONTO_DB", DEFAULT_DB)).expanduser()
    remote = _git_remote(root)
    try:
        connection = sqlite3.connect(f"file:{database}?mode=ro", uri=True, timeout=0.2)
        try:
            for repository_id, raw in connection.execute("SELECT id, payload_json FROM repositories"):
                payload = json.loads(raw)
                registered = payload.get("path")
                registered_remote = payload.get("remote_url")
                path_match = registered and Path(registered).expanduser().resolve() == root
                remote_match = remote and registered_remote and remote == registered_remote
                if path_match or remote_match:
                    # Pronto's current IDs can embed absolute paths. Persist a
                    # stable opaque derivative so path redaction is preserved.
                    return f"repository:v1:{stable_key(str(repository_id))}", "repository"
        finally:
            connection.close()
    except (OSError, sqlite3.Error, json.JSONDecodeError):
        pass
    return f"project:v1:{stable_key(str(root))}", "project"


def _domain(cwd: str) -> str:
    return "software" if _git_root(cwd) is not None else "general"


def _event_key(payload: dict[str, Any], target: str, phenomenon: str, position: str) -> str:
    session_value = str(payload.get("session_id", "unknown-session"))
    turn_value = str(payload.get("turn_id", "unknown-turn"))
    session = session_value.removeprefix("key:") if session_value.startswith("key:") else stable_key(session_value, 20)
    turn = turn_value.removeprefix("key:") if turn_value.startswith("key:") else stable_key(turn_value, 20)
    identity = f"{EVENT_KEY_VERSION}|codex|{session}|{turn}|{position}|{target}|{phenomenon}"
    return f"{EVENT_KEY_VERSION}:codex:{stable_key(identity, 40)}"


def _current_context_path(cwd: str) -> Path:
    return runtime_root() / "current" / f"{stable_key(str(Path(cwd or os.getcwd()).resolve()), 32)}.json"


def record_current_context(payload: dict[str, Any]) -> None:
    cwd = str(payload.get("cwd", ""))
    try:
        atomic_json(
            _current_context_path(cwd),
            {
                "schema_version": SCHEMA_VERSION,
                "session_key": stable_key(str(payload.get("session_id", "unknown-session")), 20),
                "turn_key": stable_key(str(payload.get("turn_id", "unknown-turn")), 20),
                "observed_at": now_iso(),
            },
        )
    except (OSError, TypeError, ValueError):
        return


def current_context(cwd: str) -> dict[str, str]:
    value = load_json(_current_context_path(cwd), {})
    return {
        "session_id": f"key:{value.get('session_key', stable_key('semantic-task', 20))}",
        "turn_id": f"key:{value.get('turn_key', stable_key('semantic-turn', 20))}",
    }


def observation_from_prompt(payload: dict[str, Any]) -> dict[str, Any] | None:
    prompt = str(payload.get("prompt", ""))
    signal = classify_prompt(prompt)
    if signal is None:
        return None
    cwd = str(payload.get("cwd", ""))
    scope_id, scope_kind = resolve_scope(cwd)
    return {
        "event_key": _event_key(payload, signal["target_kind"], signal["phenomenon_key"], "prompt:0"),
        "scope_id": scope_id,
        "scope_kind": scope_kind,
        "domain": _domain(cwd),
        "signal_kind": signal["signal_kind"],
        "target_kind": signal["target_kind"],
        "summary": signal["summary"],
        "excerpt": sanitize(prompt),
        "source": "codex_passive_hook",
        "evidence_refs": [],
        "phenomenon_key": signal["phenomenon_key"],
        "failure_mode": signal["failure_mode"],
        "priority": "P2",
        "urgent": False,
        "verified": True,
        "observed_at": now_iso(),
    }


def _bounded_diagnostic(value: Any, remaining: list[int]) -> str:
    if remaining[0] <= 0:
        return ""
    if isinstance(value, str):
        piece = value[: remaining[0]]
        remaining[0] -= len(piece)
        return piece
    if isinstance(value, dict):
        parts = []
        for key, child in value.items():
            if remaining[0] <= 0:
                break
            if str(key).casefold() in {"error", "message", "output", "stderr", "status", "content"}:
                parts.append(_bounded_diagnostic(child, remaining))
        return " ".join(parts)
    if isinstance(value, list):
        return " ".join(_bounded_diagnostic(child, remaining) for child in value[:20])
    return ""


def _explicit_tool_failure(value: Any, remaining: list[int]) -> bool:
    if remaining[0] <= 0:
        return False
    remaining[0] -= 1
    if isinstance(value, dict):
        for key, child in value.items():
            normalized = str(key).casefold().replace("-", "_")
            if normalized in {"iserror", "is_error", "failed"} and child is True:
                return True
            if normalized in {"exit_code", "exitcode", "return_code", "returncode"}:
                if isinstance(child, int) and not isinstance(child, bool) and child != 0:
                    return True
            if normalized == "status" and str(child).casefold() in {
                "error",
                "failed",
                "failure",
                "timed_out",
                "timeout",
            }:
                return True
            if isinstance(child, (dict, list)) and _explicit_tool_failure(child, remaining):
                return True
    elif isinstance(value, list):
        return any(_explicit_tool_failure(child, remaining) for child in value[:50])
    return False


def tool_failure_kind(response: Any) -> tuple[bool, bool]:
    explicit = _explicit_tool_failure(response, [500])
    diagnostic = _bounded_diagnostic(response, [20_000])
    return explicit, explicit and bool(CAPABILITY_WORDS.search(diagnostic))


def _turn_state_path(payload: dict[str, Any]) -> Path:
    session = stable_key(str(payload.get("session_id", "unknown-session")), 20)
    turn = stable_key(str(payload.get("turn_id", "unknown-turn")), 20)
    return runtime_root() / "state" / f"{session}-{turn}.json"


def _prune_turn_state() -> None:
    directory = runtime_root() / "state"
    if not directory.is_dir():
        return
    cutoff = time.time() - STATE_RETENTION_HOURS * 3600
    for path in directory.glob("*.json"):
        try:
            if not path.is_symlink() and path.stat().st_mtime < cutoff:
                path.unlink()
        except OSError:
            continue


def observations_from_tool(payload: dict[str, Any]) -> list[dict[str, Any]]:
    failed, capability = tool_failure_kind(payload.get("tool_response"))
    if not failed and not capability:
        return []
    _prune_turn_state()
    path = _turn_state_path(payload)
    state = load_json(path, {"schema_version": SCHEMA_VERSION, "failures": {}, "captured": []})
    tool_name = sanitize(str(payload.get("tool_name", "unknown-tool")), 80)
    failures = state.setdefault("failures", {})
    failures[tool_name] = int(failures.get(tool_name, 0)) + (1 if failed else 0)
    captured = set(state.setdefault("captured", []))
    cwd = str(payload.get("cwd", ""))
    scope_id, scope_kind = resolve_scope(cwd)
    observations: list[dict[str, Any]] = []

    def add(kind: str, phenomenon: str, failure_mode: str, summary: str, position: str) -> None:
        marker = f"{kind}:{tool_name}"
        if marker in captured:
            return
        captured.add(marker)
        observations.append({
            "event_key": _event_key(payload, "tool", phenomenon, position),
            "scope_id": scope_id,
            "scope_kind": scope_kind,
            "domain": _domain(cwd),
            "signal_kind": kind,
            "target_kind": "tool",
            "summary": summary,
            "excerpt": None,
            "source": "codex_passive_hook",
            "evidence_refs": [f"tool:{stable_key(tool_name, 16)}"],
            "phenomenon_key": phenomenon,
            "failure_mode": failure_mode,
            "priority": "P1" if capability else "P2",
            "urgent": False,
            "verified": True,
            "observed_at": now_iso(),
        })

    if capability:
        add(
            "capability_gap",
            normalize_phenomenon(f"{tool_name} capability unavailable"),
            "capability_unavailable",
            f"The {tool_name} tool exposed a concrete capability gap.",
            "tool:capability",
        )
    if failures[tool_name] >= 2:
        add(
            "repeated_failure",
            normalize_phenomenon(f"{tool_name} repeated tool failure"),
            "repeated_tool_failure",
            f"The {tool_name} tool failed at least twice in one task.",
            "tool:repeated",
        )
    state["captured"] = sorted(captured)
    try:
        atomic_json(path, state)
    except (OSError, TypeError, ValueError):
        pass
    return observations


