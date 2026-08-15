"""Durable storage, delivery, health, and command orchestration."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any

from .common import *
from .intake import *
from .intake import _domain, _event_key

def _health_path(root: Path | None = None) -> Path:
    return (root or runtime_root()) / "health.json"


def _spool_dir(root: Path | None = None) -> Path:
    return (root or runtime_root()) / "spool"


def _quarantine_dir(root: Path | None = None) -> Path:
    return (root or runtime_root()) / "quarantine"


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


def _quarantine_files(root: Path | None = None) -> list[Path]:
    directory = _quarantine_dir(root)
    try:
        if not directory.is_dir() or directory.is_symlink():
            return []
        return sorted(
            (path for path in directory.glob("*.json") if not path.is_symlink()),
            key=lambda path: path.stat().st_mtime,
        )
    except OSError:
        return []


def _all_quarantine_files() -> list[Path]:
    paths = [path for root in spool_roots() for path in _quarantine_files(root)]
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


def prune_quarantine(root: Path | None = None) -> None:
    cutoff = time.time() - SPOOL_RETENTION_DAYS * 86400
    paths = _quarantine_files(root)
    for path in paths:
        try:
            if path.stat().st_mtime < cutoff:
                path.unlink()
        except OSError:
            continue
    paths = _quarantine_files(root)
    for path in paths[: max(0, len(paths) - SPOOL_LIMIT)]:
        try:
            path.unlink()
        except OSError:
            continue


def quarantine_observation(path: Path) -> None:
    root = path.parent.parent
    directory = _quarantine_dir(root)
    private_dir(directory)
    prune_quarantine(root)
    target = directory / path.name
    os.replace(path, target)
    target.chmod(0o600)


def quarantine_value(observation: dict[str, Any], root: Path | None = None) -> None:
    root = root or runtime_root()
    directory = _quarantine_dir(root)
    private_dir(directory)
    prune_quarantine(root)
    paths = _quarantine_files(root)
    if len(paths) >= SPOOL_LIMIT:
        paths[0].unlink()
    event_key = str(observation.get("event_key", "missing-event"))
    atomic_json(directory / f"{stable_key(event_key, 48)}.json", observation)


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
    executable = configured_cli()
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
            try:
                value = json.loads(result.stdout)
            except (UnicodeError, json.JSONDecodeError):
                value = None
            if (
                isinstance(value, dict)
                and value.get("schema_version") == "pronto-cli-error/v1"
                and value.get("status") == "Blocked"
                and str(value.get("error", "")).startswith("Papercut ")
            ):
                diagnostic = diagnostic_for("downstream_contract_invalid")
                diagnostic["exit_code"] = result.returncode
                return False, None, diagnostic
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
    quarantined_paths = _all_quarantine_files()
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
    elif success and not quarantined_paths:
        health.pop("last_error", None)
    health.update({
        "schema_version": SCHEMA_VERSION,
        "status": (
            "healthy"
            if success and not paths and not quarantined_paths
            else ("degraded" if failures < 3 else "failing")
        ),
        "database_writable": success,
        "consecutive_failures": failures,
        # A warning is emitted once per uninterrupted failure streak. Reset the
        # marker only after a successful drain so a later real outage can warn.
        "last_warned_failure_count": 0 if success else int(health.get("last_warned_failure_count", 0)),
        "spooled_events": len(paths),
        "quarantined_events": len(quarantined_paths),
        "oldest_spool_at": oldest,
        "last_success_at": now_iso() if success else health.get("last_success_at"),
        "warning": warning if warning else (None if success else health.get("warning")),
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
        f"Run `{(diagnostic or {}).get('recovery_command', recovery_command())}` "
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
    quarantine_diagnostic = None
    for root in spool_roots():
        prune_spool(root)
        prune_quarantine(root)
    for path in _all_spool_files()[:limit]:
        observation, diagnostic = _load_spooled_observation(path)
        if diagnostic:
            try:
                quarantine_observation(path)
            except OSError:
                return flushed, False, diagnostic_for("spool_delete_failure")
            quarantine_diagnostic = diagnostic
            continue
        try:
            observation = normalize_semantic_contract(observation)
        except ValueError:
            try:
                quarantine_observation(path)
            except OSError:
                return flushed, False, diagnostic_for("spool_delete_failure")
            quarantine_diagnostic = diagnostic_for("spooled_contract_invalid")
            continue
        success, _, diagnostic = _run_pronto(observation)
        if not success:
            if diagnostic and diagnostic.get("failure_kind") == "downstream_contract_invalid":
                try:
                    quarantine_observation(path)
                except OSError:
                    return flushed, False, diagnostic_for("spool_delete_failure")
                quarantine_diagnostic = diagnostic
                continue
            return flushed, False, diagnostic
        try:
            path.unlink()
        except OSError:
            return flushed, False, diagnostic_for("spool_delete_failure")
        flushed += 1
    return flushed, True, quarantine_diagnostic


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
    if diagnostic and diagnostic.get("failure_kind") == "downstream_contract_invalid":
        for tier, root in (("primary", runtime_root()), ("emergency", emergency_root())):
            try:
                quarantine_value(observation, root)
                return _with_diagnostic({
                    "status": "quarantined",
                    "quarantined": True,
                    "quarantine_tier": tier,
                }, diagnostic), _safe_health_warning(
                    True,
                    diagnostic=diagnostic,
                    operation="capture",
                )
            except OSError:
                continue
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
    parser.add_argument(
        "mode",
        nargs="?",
        choices=("hook", "observe", "flush", "contract"),
        default="hook",
    )
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()
    if args.mode == "contract":
        print(json.dumps(observation_contract(), sort_keys=True))
        return 0
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


