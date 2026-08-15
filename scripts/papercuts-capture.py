#!/usr/bin/python3
"""Fail-open Codex intake for the local Pronto Papercuts corpus.

The hook persists only explicit interaction-quality signals and small sanitized
excerpts. It never reads transcripts, never blocks a task, and spools locally
when the Pronto CLI is unavailable.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import sqlite3
import subprocess
import sys
import tempfile
import time
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any


SCHEMA_VERSION = "pronto-papercuts-hook/v1"
EVENT_KEY_VERSION = "v1"
EXCERPT_LIMIT = 240
SPOOL_RETENTION_DAYS = 7
SPOOL_LIMIT = 10_000
STATE_RETENTION_HOURS = 48
# Semantic captures run inside a command sandbox that can write Pronto-owned
# application support but intentionally cannot mutate the protected Codex root.
DEFAULT_ROOT = Path.home() / "Library" / "Application Support" / "Pronto" / "papercuts-hook"
DEFAULT_DB = Path.home() / "Library" / "Application Support" / "Pronto" / "registry.db"
DEFAULT_CLI = Path.home() / ".codex" / "bin" / "pronto-papercuts"

SECRET_PATTERNS = (
    re.compile(r"\b(?:sk|rk)-[A-Za-z0-9_-]{12,}\b"),
    re.compile(r"\bgh[pousr]_[A-Za-z0-9]{12,}\b"),
    re.compile(r"\bBearer\s+[A-Za-z0-9._~+/=-]{8,}\b", re.IGNORECASE),
    re.compile(r"\b(?:api[_-]?key|token|secret|password)\s*[:=]\s*[^\s,;]+", re.IGNORECASE),
)
ABSOLUTE_PATH = re.compile(r"(?<![\w.])/(?:Users|private|tmp|var|opt|Applications)/[^\s,;:)]*")
WINDOWS_PATH = re.compile(r"\b[A-Za-z]:\\[^\s,;:)]*")
FAILURE_WORDS = re.compile(
    r"\b(?:error|failed|failure|timed? out|timeout|permission denied|access denied|"
    r"not found|unavailable|blocked|is_error)\b",
    re.IGNORECASE,
)
CAPABILITY_WORDS = re.compile(
    r"\b(?:unsupported|not implemented|no such (?:tool|method|command)|missing "
    r"(?:capability|primitive|interface)|cannot be called|not callable)\b",
    re.IGNORECASE,
)
VALID_SIGNAL_KINDS = {
    "dissatisfaction",
    "correction",
    "boundary_correction",
    "failure_report",
    "failed_verification",
    "repeated_failure",
    "agent_suggestion",
    "capability_gap",
    "manual_handoff",
}
VALID_TARGET_KINDS = {
    "agent_answer",
    "workflow",
    "tool",
    "repository",
    "artifact",
    "user_preference_model",
    "other",
}
PRIORITY_ALIASES = {
    "critical": "P0",
    "urgent": "P0",
    "high": "P1",
    "medium": "P2",
    "normal": "P2",
    "low": "P3",
}

# Stable public diagnostics for the fail-open process boundary. These codes are
# intentionally more specific than the process exit status (which remains zero
# for hook safety) and never include exception messages or user-controlled data.
FAIL_OPEN_ERROR_CODES = {
    "input_json_invalid": ("PAPERCUTS-E1001", "input_decode", "input was not valid JSON"),
    "input_encoding_invalid": ("PAPERCUTS-E1002", "input_decode", "input was not valid UTF-8 text"),
    "storage_permission_denied": (
        "PAPERCUTS-E2001",
        "local_storage",
        "collector storage permission was denied",
    ),
    "storage_path_missing": (
        "PAPERCUTS-E2002",
        "local_storage",
        "a required collector storage path was missing",
    ),
    "database_failure": ("PAPERCUTS-E3001", "database", "collector database access failed"),
    "child_process_timeout": (
        "PAPERCUTS-E4001",
        "pronto_process",
        "the Pronto capture process timed out",
    ),
    "child_process_failure": (
        "PAPERCUTS-E4002",
        "pronto_process",
        "the Pronto capture process failed",
    ),
    "pronto_cli_unavailable": (
        "PAPERCUTS-E4003",
        "pronto_process",
        "the Pronto capture executable was unavailable",
    ),
    "pronto_output_invalid": (
        "PAPERCUTS-E4004",
        "pronto_process",
        "the Pronto capture process returned invalid JSON",
    ),
    "contract_invalid": (
        "PAPERCUTS-E5001",
        "contract_validation",
        "the capture contract was invalid",
    ),
    "spooled_contract_invalid": (
        "PAPERCUTS-E5002",
        "contract_validation",
        "a spooled observation failed contract validation",
    ),
    "io_failure": ("PAPERCUTS-E6001", "io", "collector I/O failed"),
    "spool_read_failure": (
        "PAPERCUTS-E6002",
        "io",
        "the collector could not read a spooled observation",
    ),
    "spool_delete_failure": (
        "PAPERCUTS-E6003",
        "io",
        "the collector could not remove a flushed spool file",
    ),
    "unexpected": (
        "PAPERCUTS-E9001",
        "internal",
        "an unexpected collector error occurred",
    ),
}


PROCESS_TIMEOUT_SECONDS = 3
RECOVERY_COMMAND = "pronto-papercuts papercuts health --json"


def diagnostic_for(key: str) -> dict[str, Any]:
    code, stage, message = FAIL_OPEN_ERROR_CODES[key]
    return {
        "error_code": code,
        "failure_kind": key,
        "stage": stage,
        "message": message,
        "retryable": key not in {"contract_invalid", "spooled_contract_invalid"},
        "recovery_command": RECOVERY_COMMAND,
    }


def fail_open_diagnostic(error: BaseException) -> dict[str, Any]:
    """Return a sanitized, stable diagnostic for an unexpected top-level error."""
    if isinstance(error, json.JSONDecodeError):
        key = "input_json_invalid"
    elif isinstance(error, UnicodeError):
        key = "input_encoding_invalid"
    elif isinstance(error, PermissionError):
        key = "storage_permission_denied"
    elif isinstance(error, FileNotFoundError):
        key = "storage_path_missing"
    elif isinstance(error, sqlite3.Error):
        key = "database_failure"
    elif isinstance(error, subprocess.TimeoutExpired):
        key = "child_process_timeout"
    elif isinstance(error, subprocess.SubprocessError):
        key = "child_process_failure"
    elif isinstance(error, (KeyError, TypeError, ValueError)):
        key = "contract_invalid"
    elif isinstance(error, OSError):
        key = "io_failure"
    else:
        key = "unexpected"
    return diagnostic_for(key)


def fail_open_warning(error: BaseException) -> str:
    diagnostic = fail_open_diagnostic(error)
    return (
        "Papercuts capture remained fail-open "
        f"[{diagnostic['error_code']}; stage={diagnostic['stage']}]: "
        f"{diagnostic['message']}."
    )


def runtime_root() -> Path:
    return Path(os.environ.get("PAPERCUTS_RUNTIME_ROOT", DEFAULT_ROOT)).expanduser()


def emergency_root() -> Path:
    configured = os.environ.get("PAPERCUTS_EMERGENCY_ROOT")
    if configured:
        return Path(configured).expanduser()
    temporary_root = Path(os.environ.get("TMPDIR") or tempfile.gettempdir())
    return temporary_root / f"pronto-papercuts-hook-{os.getuid()}"


def spool_roots() -> list[Path]:
    roots: list[Path] = []
    for root in (runtime_root(), emergency_root()):
        if root not in roots:
            roots.append(root)
    return roots


def now_iso() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def stable_key(value: str, length: int = 24) -> str:
    return hashlib.sha256(value.encode("utf-8", errors="replace")).hexdigest()[:length]


def private_dir(path: Path) -> None:
    path.mkdir(parents=True, exist_ok=True, mode=0o700)
    if path.is_symlink() or not path.is_dir():
        raise OSError(f"Papercuts private path is not a real directory: {path}")
    try:
        path.chmod(0o700)
    except OSError:
        pass


def atomic_json(path: Path, value: dict[str, Any]) -> None:
    private_dir(path.parent)
    temporary = path.with_name(f".{path.name}.{os.getpid()}.tmp")
    temporary.write_text(
        json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n",
        encoding="utf-8",
    )
    temporary.chmod(0o600)
    os.replace(temporary, path)


def load_json(path: Path, default: dict[str, Any]) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
        return value if isinstance(value, dict) else dict(default)
    except (OSError, json.JSONDecodeError):
        return dict(default)


def sanitize(value: str, limit: int = EXCERPT_LIMIT) -> str:
    text = " ".join(str(value).split())
    for pattern in SECRET_PATTERNS:
        text = pattern.sub("[REDACTED_SECRET]", text)
    text = ABSOLUTE_PATH.sub("[REDACTED_PATH]", text)
    text = WINDOWS_PATH.sub("[REDACTED_PATH]", text)
    characters = list(text)
    if len(characters) > limit:
        text = "".join(characters[: max(0, limit - 1)]).rstrip() + "…"
    return text


def normalize_phenomenon(value: str) -> str:
    text = sanitize(value, 480).casefold()
    text = re.sub(r"\b(?:the|a|an|and|or|but|to|of|for|in|on|with|this|that|it|"
                  r"i|you|we|my|your|our|is|are|was|were|be|been)\b", " ", text)
    text = re.sub(r"[^a-z0-9]+", "-", text).strip("-")
    return "-".join(text.split("-")[:18]) or "explicit-interaction-signal"


def normalize_semantic_contract(value: dict[str, Any]) -> dict[str, Any]:
    signal_kind = normalize_phenomenon(str(value.get("signal_kind", ""))).replace("-", "_")
    if signal_kind not in VALID_SIGNAL_KINDS:
        raise ValueError("Papercuts semantic signal_kind is not supported")
    value["signal_kind"] = signal_kind

    target_kind = normalize_phenomenon(str(value.get("target_kind", "other"))).replace("-", "_")
    value["target_kind"] = target_kind if target_kind in VALID_TARGET_KINDS else "other"

    priority = str(value.get("priority", "P2")).strip()
    value["priority"] = PRIORITY_ALIASES.get(priority.casefold(), priority.upper())
    if value["priority"] not in {"P0", "P1", "P2", "P3"}:
        value["priority"] = "P2"

    references: list[str] = []
    for key in ("evidence_refs", "evidence_references", "evidence"):
        candidates = value.pop(key, [])
        if isinstance(candidates, str):
            candidates = [candidates]
        if isinstance(candidates, list):
            references.extend(sanitize(str(item)) for item in candidates if str(item).strip())
    value["evidence_refs"] = list(dict.fromkeys(references))[:20]
    value["verified"] = bool(value.get("verified", True))
    value["urgent"] = bool(value.get("urgent", False))
    return value


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


def _health_path(root: Path | None = None) -> Path:
    return (root or runtime_root()) / "health.json"


def _spool_dir(root: Path | None = None) -> Path:
    return (root or runtime_root()) / "spool"


def _spool_files(root: Path | None = None) -> list[Path]:
    directory = _spool_dir(root)
    try:
        if not directory.is_dir() or directory.is_symlink():
            return []
        return sorted(
            (path for path in directory.glob("*.json") if not path.is_symlink()),
            key=lambda path: path.stat().st_mtime,
        )
    except OSError:
        return []


def _all_spool_files() -> list[Path]:
    paths = [path for root in spool_roots() for path in _spool_files(root)]
    return sorted(paths, key=lambda path: path.stat().st_mtime)


def prune_spool(root: Path | None = None) -> None:
    cutoff = time.time() - SPOOL_RETENTION_DAYS * 86400
    paths = _spool_files(root)
    for path in paths:
        try:
            if path.stat().st_mtime < cutoff:
                path.unlink()
        except OSError:
            continue
    paths = _spool_files(root)
    for path in paths[: max(0, len(paths) - SPOOL_LIMIT)]:
        try:
            path.unlink()
        except OSError:
            continue


def spool_observation(observation: dict[str, Any], root: Path | None = None) -> None:
    root = root or runtime_root()
    prune_spool(root)
    directory = _spool_dir(root)
    private_dir(directory)
    paths = _spool_files(root)
    if len(paths) >= SPOOL_LIMIT:
        paths[0].unlink()
    event_key = str(observation.get("event_key", "missing-event"))
    atomic_json(directory / f"{stable_key(event_key, 48)}.json", observation)


def migrate_emergency_spool() -> int:
    primary = runtime_root()
    emergency = emergency_root()
    if primary == emergency:
        return 0
    migrated = 0
    for path in _spool_files(emergency):
        observation = load_json(path, {})
        if not observation:
            break
        try:
            spool_observation(observation, primary)
            path.unlink()
        except OSError:
            break
        migrated += 1
    return migrated


def _load_spooled_observation(path: Path) -> tuple[dict[str, Any] | None, dict[str, str] | None]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError):
        return None, diagnostic_for("spool_read_failure")
    except json.JSONDecodeError:
        return None, diagnostic_for("spooled_contract_invalid")
    if not isinstance(value, dict):
        return None, diagnostic_for("spooled_contract_invalid")
    return value, None


def _run_pronto(
    observation: dict[str, Any],
    dry_run: bool = False,
) -> tuple[bool, dict[str, Any] | None, dict[str, Any] | None]:
    configured = os.environ.get("PAPERCUTS_PRONTO_CLI")
    executable = Path(configured).expanduser() if configured else DEFAULT_CLI
    if not executable.is_file() or not os.access(executable, os.X_OK):
        return False, None, diagnostic_for("pronto_cli_unavailable")
    command = [str(executable), "papercuts", "observe", "--stdin", "--json"]
    if dry_run:
        command.append("--dry-run")
    try:
        result = subprocess.run(
            command,
            input=json.dumps(observation),
            capture_output=True,
            text=True,
            timeout=PROCESS_TIMEOUT_SECONDS,
            check=False,
        )
        if result.returncode != 0:
            diagnostic = diagnostic_for("child_process_failure")
            diagnostic["exit_code"] = result.returncode
            return False, None, diagnostic
        value = json.loads(result.stdout)
        if not isinstance(value, dict):
            return False, None, diagnostic_for("pronto_output_invalid")
        return True, value, None
    except subprocess.TimeoutExpired:
        diagnostic = diagnostic_for("child_process_timeout")
        diagnostic["timeout_seconds"] = PROCESS_TIMEOUT_SECONDS
        return False, None, diagnostic
    except (OSError, subprocess.SubprocessError):
        return False, None, diagnostic_for("child_process_failure")
    except (UnicodeError, json.JSONDecodeError):
        return False, None, diagnostic_for("pronto_output_invalid")


def _write_health(
    success: bool,
    warning: str | None = None,
    root: Path | None = None,
    diagnostic: dict[str, Any] | None = None,
    operation: str | None = None,
) -> dict[str, Any]:
    path = _health_path(root)
    health = load_json(path, {})
    failures = 0 if success else int(health.get("consecutive_failures", 0)) + 1
    paths = _all_spool_files()
    oldest = None
    if paths:
        oldest = datetime.fromtimestamp(paths[0].stat().st_mtime, timezone.utc).isoformat().replace("+00:00", "Z")
    if diagnostic:
        health["last_error"] = {
            **diagnostic,
            "operation": operation or "unknown",
            "attempt": failures,
            "observed_at": now_iso(),
        }
    health.update({
        "schema_version": SCHEMA_VERSION,
        "status": "healthy" if success and not paths else ("degraded" if failures < 3 else "failing"),
        "database_writable": success,
        "consecutive_failures": failures,
        # A warning is emitted once per uninterrupted failure streak. Reset the
        # marker only after a successful drain so a later real outage can warn.
        "last_warned_failure_count": 0 if success else int(health.get("last_warned_failure_count", 0)),
        "spooled_events": len(paths),
        "oldest_spool_at": oldest,
        "last_success_at": now_iso() if success else health.get("last_success_at"),
        "warning": warning if warning else (None if success and not paths else health.get("warning")),
        "excerpt_retention_days": 90,
    })
    atomic_json(path, health)
    return health


def _diagnostic_suffix(
    diagnostic: dict[str, Any] | None,
    operation: str | None = None,
) -> str:
    if not diagnostic:
        return ""
    return (
        f" [{diagnostic['error_code']}; stage={diagnostic['stage']}; "
        f"operation={operation or 'unknown'}]"
    )


def _threshold_warning(
    health: dict[str, Any],
    root: Path | None = None,
    diagnostic: dict[str, Any] | None = None,
    operation: str | None = None,
) -> str | None:
    failures = int(health.get("consecutive_failures", 0))
    if failures < 3:
        return None
    last_warned = int(health.get("last_warned_failure_count", 0))
    if last_warned >= 3:
        return None
    spool_count = int(health.get("spooled_events", 0))
    cause = diagnostic["message"] if diagnostic else "the capture drain failed"
    detail = ""
    if diagnostic and diagnostic.get("timeout_seconds") is not None:
        detail = f" after {diagnostic['timeout_seconds']} seconds"
    warning = (
        f"Papercuts drain failed three times (attempt {failures}): {cause}{detail}"
        f"{_diagnostic_suffix(diagnostic, operation)}. "
        f"{spool_count} observation{'s remain' if spool_count != 1 else ' remains'} locally spooled. "
        f"Run `{(diagnostic or {}).get('recovery_command', RECOVERY_COMMAND)}` "
        "for current health and recovery details, then retry the drain."
    )
    health["last_warned_failure_count"] = failures
    health["warning"] = warning
    atomic_json(_health_path(root), health)
    return warning


def _safe_health_warning(
    success: bool,
    warning: str | None = None,
    fallback: str | None = None,
    diagnostic: dict[str, Any] | None = None,
    operation: str | None = None,
) -> str | None:
    """Keep health bookkeeping fail-open when its storage is unavailable."""
    for root in spool_roots():
        try:
            return _threshold_warning(
                _write_health(success, warning, root, diagnostic, operation),
                root,
                diagnostic,
                operation,
            )
        except (OSError, TypeError, ValueError):
            continue
    return fallback


def flush_spool(limit: int = 100) -> tuple[int, bool, dict[str, Any] | None]:
    migrate_emergency_spool()
    flushed = 0
    all_success = True
    for root in spool_roots():
        prune_spool(root)
    for path in _all_spool_files()[:limit]:
        observation, diagnostic = _load_spooled_observation(path)
        if diagnostic:
            return flushed, False, diagnostic
        try:
            observation = normalize_semantic_contract(observation)
        except ValueError:
            return flushed, False, diagnostic_for("spooled_contract_invalid")
        success, _, diagnostic = _run_pronto(observation)
        if not success:
            return flushed, False, diagnostic
        try:
            path.unlink()
        except OSError:
            return flushed, False, diagnostic_for("spool_delete_failure")
        flushed += 1
    return flushed, all_success, None


def _with_diagnostic(
    result: dict[str, Any],
    diagnostic: dict[str, Any] | None,
) -> dict[str, Any]:
    if not diagnostic:
        return result
    return {**result, "diagnostic": diagnostic}


def persist(observation: dict[str, Any], dry_run: bool = False) -> tuple[dict[str, Any], str | None]:
    if dry_run:
        # Dry-run is a pure contract check: normalize the observation locally
        # and return the plan without migrating spools, writing health state, or
        # invoking the Pronto process.
        normalized = normalize_semantic_contract(dict(observation))
        return {"status": "dry_run", "observation": normalized}, None
    migrate_emergency_spool()
    success, result, diagnostic = _run_pronto(observation, dry_run=dry_run)
    if success:
        _, flush_success, flush_diagnostic = flush_spool()
        return _with_diagnostic(
            result or {"status": "captured"},
            flush_diagnostic,
        ), _safe_health_warning(
            flush_success,
            diagnostic=flush_diagnostic,
            operation="drain",
        )
    for tier, root in (("primary", runtime_root()), ("emergency", emergency_root())):
        try:
            spool_observation(observation, root)
            return _with_diagnostic({
                "status": "spooled",
                "spooled": True,
                "spool_tier": tier,
            }, diagnostic), (
                _safe_health_warning(
                    False,
                    diagnostic=diagnostic,
                    operation="capture",
                )
                if tier == "primary"
                # Repository-sandboxed semantic capture cannot always write
                # Pronto Application Support. Its emergency spool is a normal
                # handoff to the outer PostToolUse hook, which owns the drain
                # attempt and shared failure accounting.
                else None
            )
        except OSError:
            continue
    warning = "Papercuts capture could not reach Pronto or write either local spool."
    warning = f"{warning[:-1]}{_diagnostic_suffix(diagnostic, 'capture')}." if diagnostic else warning
    _safe_health_warning(
        False,
        warning,
        fallback=warning,
        diagnostic=diagnostic,
        operation="capture",
    )
    return _with_diagnostic({"status": "failed_open", "spooled": False}, diagnostic), warning


def hook_warning(event: str, warning: str) -> None:
    print(json.dumps({"hookSpecificOutput": {
        "hookEventName": event,
        "additionalContext": warning,
    }}, separators=(",", ":")))


def handle_hook(payload: dict[str, Any]) -> None:
    event = str(payload.get("hook_event_name", ""))
    observations: list[dict[str, Any]] = []
    if event == "UserPromptSubmit":
        record_current_context(payload)
        observation = observation_from_prompt(payload)
        if observation is not None:
            observations.append(observation)
    elif event == "PostToolUse":
        observations.extend(observations_from_tool(payload))
    warning = None
    for observation in observations:
        _, current_warning = persist(observation)
        warning = warning or current_warning
    try:
        migrate_emergency_spool()
        has_spool = bool(_all_spool_files())
    except (OSError, TypeError, ValueError):
        has_spool = False
    if not observations and has_spool:
        try:
            _, success, diagnostic = flush_spool(20)
        except (OSError, TypeError, ValueError):
            success = False
            diagnostic = diagnostic_for("io_failure")
        warning = _safe_health_warning(
            success,
            diagnostic=diagnostic,
            operation="drain",
        )
    if warning:
        hook_warning(event, warning)


def semantic_observation(value: dict[str, Any]) -> dict[str, Any]:
    cwd = str(value.pop("cwd", os.getcwd()))
    scope_id, scope_kind = resolve_scope(cwd)
    value.setdefault("scope_id", scope_id)
    value.setdefault("scope_kind", scope_kind)
    value.setdefault("domain", _domain(cwd))
    value.setdefault("source", "codex_semantic_route")
    value.setdefault("evidence_refs", [])
    value.setdefault("priority", "P2")
    value.setdefault("urgent", False)
    value.setdefault("verified", True)
    value.setdefault("observed_at", now_iso())
    value["summary"] = sanitize(str(value.get("summary", "")))
    excerpt = value.get("excerpt")
    value["excerpt"] = sanitize(str(excerpt)) if excerpt else None
    value["phenomenon_key"] = normalize_phenomenon(str(value.get("phenomenon_key") or value["summary"]))
    value["failure_mode"] = normalize_phenomenon(str(value.get("failure_mode") or "unspecified"))
    value = normalize_semantic_contract(value)
    if not value.get("event_key"):
        context = current_context(cwd)
        task = str(value.pop("task_id", context["session_id"]))
        turn = str(value.pop("turn_id", context["turn_id"]))
        default_position = (
            "prompt:0"
            if value.get("signal_kind") in {"dissatisfaction", "correction", "failure_report"}
            else "semantic:0"
        )
        value["event_key"] = _event_key(
            {"session_id": task, "turn_id": turn},
            str(value.get("target_kind", "other")),
            str(value["phenomenon_key"]),
            str(value.pop("signal_position", default_position)),
        )
    return value


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("mode", nargs="?", choices=("hook", "observe", "flush"), default="hook")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()
    if args.mode == "flush":
        flushed, success, diagnostic = flush_spool(SPOOL_LIMIT)
        health = None
        warning = None
        for root in spool_roots():
            try:
                health = _write_health(
                    success,
                    root=root,
                    diagnostic=diagnostic,
                    operation="drain",
                )
                warning = _threshold_warning(health, root, diagnostic, "drain")
                break
            except (OSError, TypeError, ValueError):
                continue
        health = health or {"status": "unavailable", "database_writable": False}
        payload = {"status": "ok" if success else "degraded", "flushed": flushed, "health": health}
        if diagnostic:
            payload["diagnostic"] = diagnostic
        if warning:
            payload["warning"] = warning
        print(json.dumps(payload))
        return 0
    payload = json.load(sys.stdin)
    if not isinstance(payload, dict):
        raise ValueError("Papercuts input must be a JSON object")
    if args.mode == "observe":
        observation = semantic_observation(payload)
        result, warning = persist(observation, dry_run=args.dry_run)
        print(json.dumps({"schema_version": SCHEMA_VERSION, **result, "warning": warning}))
    else:
        handle_hook(payload)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except SystemExit:
        raise
    except Exception as error:
        # Hooks are deliberately fail-open. If even the health/spool path is
        # unusable, emit one sanitized coded warning and still exit successfully.
        try:
            event = "UserPromptSubmit"
            hook_warning(event, fail_open_warning(error))
        except Exception:
            pass
        raise SystemExit(0)
