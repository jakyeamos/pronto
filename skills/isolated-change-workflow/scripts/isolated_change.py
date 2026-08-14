#!/usr/bin/env python3
"""Receipt-backed Git worktree isolation for agent-owned repository changes."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import uuid
from pathlib import Path
from typing import Any, Sequence


SCHEMA = "isolated-change-task/v1"
MANAGED_HOOK_MARKER = "# isolated-change-workflow managed hook v1"
SLUG_RE = re.compile(r"[^a-z0-9]+")


class WorkflowError(RuntimeError):
    def __init__(self, code: str, message: str, details: dict[str, Any] | None = None):
        super().__init__(message)
        self.code = code
        self.details = details or {}


def clean_env() -> dict[str, str]:
    return {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}


def run_git(
    repo: Path,
    args: Sequence[str],
    *,
    check: bool = True,
    text: bool = True,
) -> subprocess.CompletedProcess[Any]:
    result = subprocess.run(
        ["git", "-C", str(repo), *args],
        capture_output=True,
        check=False,
        env=clean_env(),
        text=text,
    )
    if check and result.returncode != 0:
        stderr = result.stderr.strip() if text else result.stderr.decode(errors="replace").strip()
        raise WorkflowError("git-command-failed", f"git {' '.join(args)} failed: {stderr}")
    return result


def resolve_repo(path: str) -> Path:
    candidate = Path(path).expanduser().resolve()
    result = run_git(candidate, ["rev-parse", "--show-toplevel"])
    return Path(result.stdout.strip()).resolve()


def common_git_dir(repo: Path) -> Path:
    value = run_git(repo, ["rev-parse", "--path-format=absolute", "--git-common-dir"]).stdout.strip()
    return Path(value).resolve()


def receipt_dir(repo: Path) -> Path:
    return common_git_dir(repo) / "isolated-change-workflow" / "tasks"


def validate_worktree_root(repo: Path, worktree_root: Path) -> None:
    if worktree_root == repo or worktree_root.is_relative_to(repo):
        raise WorkflowError(
            "worktree-root-inside-repository",
            "task worktree root must be outside the source checkout",
            {"repository": str(repo), "worktree_root": str(worktree_root)},
        )


def atomic_json_write(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    handle, temporary = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(handle, "w", encoding="utf-8") as stream:
            json.dump(payload, stream, indent=2, sort_keys=True)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def slug(value: str) -> str:
    normalized = SLUG_RE.sub("-", value.lower()).strip("-")
    if not normalized:
        raise WorkflowError("invalid-task", "task name must contain a letter or number")
    return normalized[:48].rstrip("-")


def ref_sha(repo: Path, ref: str) -> str:
    result = run_git(repo, ["rev-parse", "--verify", f"{ref}^{{commit}}"])
    return result.stdout.strip()


def current_branch(repo: Path) -> str | None:
    result = run_git(repo, ["symbolic-ref", "--quiet", "--short", "HEAD"], check=False)
    return result.stdout.strip() if result.returncode == 0 else None


def current_head(repo: Path) -> str:
    return ref_sha(repo, "HEAD")


def worktrees(repo: Path) -> list[dict[str, Any]]:
    result = run_git(repo, ["worktree", "list", "--porcelain", "-z"], text=False)
    fields = result.stdout.split(b"\0")
    records: list[dict[str, Any]] = []
    record: dict[str, Any] = {}
    for raw in fields:
        if not raw:
            if record:
                records.append(record)
                record = {}
            continue
        key, _, value = raw.partition(b" ")
        record[key.decode()] = value.decode(errors="surrogateescape") if value else True
    if record:
        records.append(record)
    return records


def has_operation(repo: Path) -> list[str]:
    git_dir = Path(run_git(repo, ["rev-parse", "--path-format=absolute", "--git-dir"]).stdout.strip())
    candidates = {
        "MERGE_HEAD": git_dir / "MERGE_HEAD",
        "CHERRY_PICK_HEAD": git_dir / "CHERRY_PICK_HEAD",
        "REVERT_HEAD": git_dir / "REVERT_HEAD",
        "rebase-merge": git_dir / "rebase-merge",
        "rebase-apply": git_dir / "rebase-apply",
    }
    return [name for name, path in candidates.items() if path.exists()]


def is_clean(repo: Path) -> bool:
    result = run_git(repo, ["status", "--porcelain=v2", "-z"], text=False)
    return not result.stdout


def load_receipts(repo: Path) -> list[dict[str, Any]]:
    directory = receipt_dir(repo)
    if not directory.exists():
        return []
    receipts: list[dict[str, Any]] = []
    for path in sorted(directory.glob("*.json")):
        try:
            payload = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            receipts.append({"schema_version": "invalid", "receipt_path": str(path), "error": str(error)})
            continue
        payload["receipt_path"] = str(path)
        receipts.append(payload)
    return receipts


def matching_receipt(repo: Path) -> dict[str, Any]:
    root = str(repo.resolve())
    matches = [item for item in load_receipts(repo) if item.get("worktree") == root and item.get("status") == "active"]
    if len(matches) != 1:
        raise WorkflowError(
            "missing-task-receipt",
            "current worktree does not have exactly one active isolation receipt",
            {"worktree": root, "active_receipts": len(matches)},
        )
    receipt = matches[0]
    if receipt.get("schema_version") != SCHEMA:
        raise WorkflowError("unsupported-receipt", "task receipt schema is not supported")
    return receipt


def guard_payload(repo: Path) -> dict[str, Any]:
    receipt = matching_receipt(repo)
    records = worktrees(repo)
    if not records or Path(str(records[0]["worktree"])).resolve() == repo:
        raise WorkflowError("primary-worktree", "commits and task mutations must occur in a linked task worktree")
    branch = current_branch(repo)
    if branch != receipt.get("branch"):
        raise WorkflowError(
            "branch-mismatch",
            "current branch does not match the task receipt",
            {"expected": receipt.get("branch"), "actual": branch},
        )
    base_sha = str(receipt["base_sha"])
    ancestry = run_git(repo, ["merge-base", "--is-ancestor", base_sha, "HEAD"], check=False)
    if ancestry.returncode != 0:
        raise WorkflowError("base-not-ancestor", "task branch no longer descends from its recorded base")
    operations = has_operation(repo)
    if operations:
        raise WorkflowError("git-operation-active", "task worktree has an active Git operation", {"operations": operations})
    return {
        "schema_version": SCHEMA,
        "allowed": True,
        "task_id": receipt["task_id"],
        "worktree": str(repo),
        "branch": branch,
        "base_sha": base_sha,
        "head_sha": current_head(repo),
    }


def command_start(args: argparse.Namespace) -> dict[str, Any]:
    repo = resolve_repo(args.repo)
    operations = has_operation(repo)
    if operations:
        raise WorkflowError("git-operation-active", "source checkout has an active Git operation", {"operations": operations})
    task_slug = slug(args.task)
    branch = args.branch or f"codex/{task_slug}"
    if not branch.startswith("codex/"):
        raise WorkflowError("invalid-branch", "task branch must use the codex/ prefix")
    if run_git(repo, ["show-ref", "--verify", "--quiet", f"refs/heads/{branch}"], check=False).returncode == 0:
        raise WorkflowError("branch-exists", f"branch already exists: {branch}")
    base_sha = ref_sha(repo, args.base)
    task_id = f"{dt.datetime.now(dt.timezone.utc).strftime('%Y%m%dT%H%M%SZ')}-{uuid.uuid4().hex[:8]}"
    worktree_root = (
        Path(args.worktree_root).expanduser().resolve()
        if args.worktree_root
        else (Path.home() / ".codex" / "worktrees").resolve()
    )
    validate_worktree_root(repo, worktree_root)
    destination = worktree_root / task_id / repo.name
    if destination.exists():
        raise WorkflowError("worktree-exists", f"worktree destination already exists: {destination}")
    destination.parent.mkdir(parents=True, exist_ok=False)
    created = False
    try:
        run_git(
            repo,
            [
                "worktree",
                "add",
                "--lock",
                "--reason",
                f"isolated-change:{task_id}",
                "-b",
                branch,
                str(destination),
                base_sha,
            ],
        )
        created = True
        payload = {
            "schema_version": SCHEMA,
            "task_id": task_id,
            "task": args.task,
            "status": "active",
            "repository": str(repo),
            "common_git_dir": str(common_git_dir(repo)),
            "worktree": str(destination),
            "branch": branch,
            "base_ref": args.base,
            "base_sha": base_sha,
            "created_at": dt.datetime.now(dt.timezone.utc).isoformat(),
        }
        path = receipt_dir(repo) / f"{task_id}.json"
        atomic_json_write(path, payload)
        payload["receipt_path"] = str(path)
        payload["primary_worktree_dirty_preserved"] = not is_clean(repo)
        return payload
    except Exception:
        if created:
            run_git(repo, ["worktree", "unlock", str(destination)], check=False)
            run_git(repo, ["worktree", "remove", str(destination)], check=False)
            run_git(repo, ["branch", "-d", branch], check=False)
        if destination.parent.exists() and not any(destination.parent.iterdir()):
            destination.parent.rmdir()
        raise


def command_guard(args: argparse.Namespace) -> dict[str, Any]:
    return guard_payload(resolve_repo(args.repo))


def changed_paths(repo: Path, left: str, right: str) -> list[str]:
    result = run_git(repo, ["diff", "--name-only", "-z", f"{left}..{right}"], text=False)
    return sorted(path.decode(errors="surrogateescape") for path in result.stdout.split(b"\0") if path)


def command_verify(args: argparse.Namespace) -> dict[str, Any]:
    repo = resolve_repo(args.repo)
    guard = guard_payload(repo)
    receipt = matching_receipt(repo)
    base = str(receipt["base_sha"])
    head = guard["head_sha"]
    target_ref = args.target or receipt.get("base_ref") or base
    target = ref_sha(repo, str(target_ref))
    clean = is_clean(repo)
    task_paths = changed_paths(repo, base, head)
    target_paths = changed_paths(repo, base, target) if target != base else []
    overlap = sorted(set(task_paths).intersection(target_paths))
    commits = int(run_git(repo, ["rev-list", "--count", f"{base}..{head}"]).stdout.strip())
    conflicts = False
    conflict_probe = "not-needed"
    if target != base and head != target:
        probe = run_git(repo, ["merge-tree", "--write-tree", target, head], check=False)
        conflicts = probe.returncode != 0
        conflict_probe = "conflicted" if conflicts else "clean"
    blockers: list[str] = []
    if not clean:
        blockers.append("task-worktree-dirty")
    if commits == 0:
        blockers.append("no-task-commits")
    if not task_paths:
        blockers.append("no-task-diff")
    if conflicts:
        blockers.append("target-merge-conflict")
    return {
        "schema_version": SCHEMA,
        "task_id": receipt["task_id"],
        "worktree": str(repo),
        "branch": receipt["branch"],
        "base_sha": base,
        "head_sha": head,
        "target_ref": target_ref,
        "target_sha": target,
        "target_moved": target != base,
        "task_commit_count": commits,
        "task_changed_paths": task_paths,
        "target_changed_paths": target_paths,
        "overlapping_paths": overlap,
        "merge_tree_probe": conflict_probe,
        "worktree_clean": clean,
        "blockers": blockers,
        "ready_for_integration": not blockers,
    }


def command_status(args: argparse.Namespace) -> dict[str, Any]:
    repo = resolve_repo(args.repo)
    items: list[dict[str, Any]] = []
    for receipt in load_receipts(repo):
        item = dict(receipt)
        path_value = item.get("worktree")
        path = Path(path_value) if isinstance(path_value, str) else None
        item["worktree_present"] = bool(path and path.exists())
        if item["worktree_present"]:
            try:
                item["branch_live"] = current_branch(path)
                item["head_sha"] = current_head(path)
                item["worktree_clean"] = is_clean(path)
            except WorkflowError as error:
                item["inspection_error"] = {"code": error.code, "message": str(error)}
        items.append(item)
    return {"schema_version": "isolated-change-status/v1", "repository": str(repo), "tasks": items}


def hooks_dir(repo: Path) -> Path:
    result = run_git(repo, ["rev-parse", "--path-format=absolute", "--git-path", "hooks"])
    directory = Path(result.stdout.strip()).resolve()
    common = common_git_dir(repo)
    if not directory.is_relative_to(common):
        configured = run_git(repo, ["config", "--get", "core.hooksPath"], check=False).stdout.strip()
        raise WorkflowError(
            "external-hooks-path",
            "refusing to modify a hooks directory outside this repository's common Git directory",
            {"hooks_path": str(directory), "configured_hooks_path": configured},
        )
    return directory


def command_install_hook(args: argparse.Namespace) -> dict[str, Any]:
    if not args.apply:
        raise WorkflowError("apply-required", "install-hook requires --apply because it changes repository Git metadata")
    repo = resolve_repo(args.repo)
    directory = hooks_dir(repo)
    hook = directory / "pre-commit"
    if hook.exists() and MANAGED_HOOK_MARKER not in hook.read_text(encoding="utf-8", errors="replace"):
        raise WorkflowError("existing-hook", "refusing to overwrite an unmanaged pre-commit hook", {"path": str(hook)})
    installed_tool = common_git_dir(repo) / "isolated-change-workflow" / "bin" / "isolated_change.py"
    installed_tool.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(Path(__file__).resolve(), installed_tool)
    installed_tool.chmod(0o755)
    directory.mkdir(parents=True, exist_ok=True)
    content = (
        "#!/bin/sh\n"
        f"{MANAGED_HOOK_MARKER}\n"
        f'exec python3 "{installed_tool}" guard --repo "$PWD"\n'
    )
    hook.write_text(content, encoding="utf-8")
    hook.chmod(0o755)
    return {
        "schema_version": "isolated-change-hook/v1",
        "installed": True,
        "repository": str(repo),
        "hook": str(hook),
        "tool": str(installed_tool),
    }


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(description=__doc__)
    subcommands = root.add_subparsers(dest="command", required=True)

    start = subcommands.add_parser("start", help="create a receipt-backed task worktree")
    start.add_argument("--repo", required=True)
    start.add_argument("--task", required=True)
    start.add_argument("--base", default="HEAD")
    start.add_argument("--branch")
    start.add_argument("--worktree-root")
    start.add_argument("--json", action="store_true")
    start.set_defaults(handler=command_start)

    guard = subcommands.add_parser("guard", help="prove the current worktree is task-owned")
    guard.add_argument("--repo", required=True)
    guard.add_argument("--json", action="store_true")
    guard.set_defaults(handler=command_guard)

    verify = subcommands.add_parser("verify", help="check readiness for reviewed integration")
    verify.add_argument("--repo", required=True)
    verify.add_argument("--target")
    verify.add_argument("--json", action="store_true")
    verify.set_defaults(handler=command_verify)

    status = subcommands.add_parser("status", help="list task receipts and live worktree state")
    status.add_argument("--repo", required=True)
    status.add_argument("--json", action="store_true")
    status.set_defaults(handler=command_status)

    install = subcommands.add_parser("install-hook", help="install the managed pre-commit guard")
    install.add_argument("--repo", required=True)
    install.add_argument("--apply", action="store_true")
    install.add_argument("--json", action="store_true")
    install.set_defaults(handler=command_install_hook)
    return root


def emit(payload: dict[str, Any], as_json: bool) -> None:
    if as_json:
        print(json.dumps(payload, indent=2, sort_keys=True))
    else:
        for key, value in payload.items():
            print(f"{key}: {value}")


def main(argv: Sequence[str] | None = None) -> int:
    args = parser().parse_args(argv)
    try:
        payload = args.handler(args)
        emit(payload, bool(getattr(args, "json", False)))
        return 0
    except WorkflowError as error:
        payload = {"ok": False, "error": {"code": error.code, "message": str(error), "details": error.details}}
        print(json.dumps(payload, indent=2, sort_keys=True) if getattr(args, "json", False) else f"{error.code}: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
