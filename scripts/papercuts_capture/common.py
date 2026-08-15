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
import shlex
import sqlite3
import subprocess
import sys
import tempfile
import time
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any


SCHEMA_VERSION = "pronto-papercuts-hook/v1"
OBSERVATION_CONTRACT_VERSION = "pronto-papercuts-observation/v1"
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
    "downstream_contract_invalid": (
        "PAPERCUTS-E5003",
        "contract_validation",
        "the installed Pronto CLI rejected the observation contract",
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


def configured_cli() -> Path:
    configured = os.environ.get("PAPERCUTS_PRONTO_CLI")
    return Path(configured).expanduser() if configured else DEFAULT_CLI


def recovery_command() -> str:
    return f"{shlex.quote(str(configured_cli()))} papercuts health --json"


def diagnostic_for(key: str) -> dict[str, Any]:
    code, stage, message = FAIL_OPEN_ERROR_CODES[key]
    return {
        "error_code": code,
        "failure_kind": key,
        "stage": stage,
        "message": message,
        "retryable": key not in {
            "contract_invalid",
            "spooled_contract_invalid",
            "downstream_contract_invalid",
        },
        "recovery_command": recovery_command(),
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


def observation_contract() -> dict[str, Any]:
    """Return the deploy-time contract shared with the native Pronto CLI."""
    return {
        "schema_version": OBSERVATION_CONTRACT_VERSION,
        "signal_kinds": sorted(VALID_SIGNAL_KINDS),
        "target_kinds": sorted(VALID_TARGET_KINDS),
        "minimal_input": {
            "event_key": "v1:example:opaque-event",
            "scope_id": "opaque:v1:example-scope",
            "signal_kind": "capability_gap",
            "target_kind": "tool",
            "summary": "Sanitized factual summary",
            "failure_mode": "stable-failure-mode",
        },
    }


