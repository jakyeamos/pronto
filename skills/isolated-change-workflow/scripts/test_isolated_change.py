#!/usr/bin/env python3

from __future__ import annotations

import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("isolated_change.py")


def environment() -> dict[str, str]:
    return {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}


class IsolatedChangeTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.repo = self.root / "repo"
        self.repo.mkdir()
        self.git("init", "-b", "main")
        self.git("config", "user.name", "Fixture User")
        self.git("config", "user.email", "fixture@example.test")
        self.git("config", "core.hooksPath", str(self.repo / ".git" / "hooks"))
        (self.repo / ".pre-cr.json").write_text(
            json.dumps(
                {
                    "version": 1,
                    "testCommand": "python3 -m unittest",
                    "coveragePaths": [],
                    "threshold": 0,
                    "surfaces": {
                        "covered": ["*.py"],
                        "ignored": ["*.md", "*.json"],
                        "unsupported": [],
                    },
                    "checks": {"coverage": False, "security": False, "checklist": False},
                },
                indent=2,
            )
            + "\n",
            encoding="utf-8",
        )
        (self.repo / "shared.txt").write_text("base\n", encoding="utf-8")
        self.git("add", "-f", ".pre-cr.json", "shared.txt")
        self.git("commit", "-m", "base")

    def tearDown(self) -> None:
        self.temp.cleanup()

    def git(self, *args: str, cwd: Path | None = None, check: bool = True) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["git", "-C", str(cwd or self.repo), *args],
            capture_output=True,
            check=check,
            env=environment(),
            text=True,
        )

    def tool(self, *args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["python3", str(SCRIPT), *args],
            capture_output=True,
            check=check,
            env=environment(),
            text=True,
        )

    def start(self, task: str = "fixture-task") -> dict[str, object]:
        result = self.tool(
            "start",
            "--repo",
            str(self.repo),
            "--task",
            task,
            "--worktree-root",
            str(self.root / "worktrees"),
            "--json",
        )
        return json.loads(result.stdout)

    def test_dirty_primary_is_preserved_and_task_verifies(self) -> None:
        (self.repo / "owner.txt").write_text("uncommitted owner work\n", encoding="utf-8")
        payload = self.start()
        task = Path(str(payload["worktree"]))
        self.assertTrue(payload["primary_worktree_dirty_preserved"])
        self.assertFalse((task / "owner.txt").exists())
        self.assertTrue((task / ".pre-cr.json").is_file())

        guard = self.tool("guard", "--repo", str(task), "--json")
        self.assertTrue(json.loads(guard.stdout)["allowed"])
        (task / "task.txt").write_text("task change\n", encoding="utf-8")
        self.git("add", "task.txt", cwd=task)
        self.git("commit", "-m", "task change", cwd=task)

        verified = json.loads(self.tool("verify", "--repo", str(task), "--target", "main", "--json").stdout)
        self.assertTrue(verified["ready_for_integration"])
        self.assertEqual(verified["task_changed_paths"], ["task.txt"])
        self.assertTrue((self.repo / "owner.txt").exists())

    def test_start_accepts_external_sibling_worktree_root(self) -> None:
        external_root = self.root / "sibling-worktrees"
        result = self.tool(
            "start",
            "--repo",
            str(self.repo),
            "--task",
            "external-root",
            "--worktree-root",
            str(external_root),
            "--json",
        )
        payload = json.loads(result.stdout)
        task = Path(str(payload["worktree"]))

        self.assertTrue(task.resolve().is_relative_to(external_root.resolve()))
        self.assertFalse(task.resolve().is_relative_to(self.repo.resolve()))
        self.assertTrue(json.loads(self.tool("guard", "--repo", str(task), "--json").stdout)["allowed"])

    def test_start_rejects_worktree_root_inside_source_without_mutation(self) -> None:
        nested_root = self.repo / ".codex-task-worktrees"
        before_status = self.git("status", "--porcelain=v2", "-z").stdout
        before_worktrees = self.git("worktree", "list", "--porcelain").stdout

        result = self.tool(
            "start",
            "--repo",
            str(self.repo),
            "--task",
            "nested-root",
            "--worktree-root",
            str(nested_root),
            "--json",
            check=False,
        )

        self.assertEqual(result.returncode, 2)
        error = json.loads(result.stderr)["error"]
        self.assertEqual(error["code"], "worktree-root-inside-repository")
        self.assertEqual(error["details"]["repository"], str(self.repo.resolve()))
        self.assertEqual(error["details"]["worktree_root"], str(nested_root.resolve()))
        self.assertFalse(nested_root.exists())
        self.assertEqual(self.git("status", "--porcelain=v2", "-z").stdout, before_status)
        self.assertEqual(self.git("worktree", "list", "--porcelain").stdout, before_worktrees)
        branch = self.git(
            "show-ref", "--verify", "--quiet", "refs/heads/codex/nested-root", check=False
        )
        self.assertNotEqual(branch.returncode, 0)

    def test_guard_rejects_primary_checkout(self) -> None:
        result = self.tool("guard", "--repo", str(self.repo), "--json", check=False)
        self.assertEqual(result.returncode, 2)
        self.assertEqual(json.loads(result.stderr)["error"]["code"], "missing-task-receipt")

    def test_target_same_line_conflict_blocks_readiness(self) -> None:
        payload = self.start("conflict-task")
        task = Path(str(payload["worktree"]))
        (task / "shared.txt").write_text("task version\n", encoding="utf-8")
        self.git("add", "shared.txt", cwd=task)
        self.git("commit", "-m", "task version", cwd=task)

        (self.repo / "shared.txt").write_text("target version\n", encoding="utf-8")
        self.git("add", "shared.txt")
        self.git("commit", "-m", "target version")

        verified = json.loads(self.tool("verify", "--repo", str(task), "--target", "main", "--json").stdout)
        self.assertFalse(verified["ready_for_integration"])
        self.assertEqual(verified["merge_tree_probe"], "conflicted")
        self.assertIn("shared.txt", verified["overlapping_paths"])
        self.assertIn("target-merge-conflict", verified["blockers"])

    def test_managed_hook_rejects_primary_and_allows_task(self) -> None:
        payload = self.start("hook-task")
        task = Path(str(payload["worktree"]))
        installed = json.loads(
            self.tool("install-hook", "--repo", str(self.repo), "--apply", "--json").stdout
        )
        self.assertTrue(installed["installed"])

        (self.repo / "primary.txt").write_text("blocked\n", encoding="utf-8")
        self.git("add", "primary.txt")
        blocked = self.git("commit", "-m", "must fail", check=False)
        self.assertNotEqual(blocked.returncode, 0)
        self.git("restore", "--staged", "primary.txt")

        (task / "task.txt").write_text("allowed\n", encoding="utf-8")
        self.git("add", "task.txt", cwd=task)
        allowed = self.git("commit", "-m", "task commit", cwd=task, check=False)
        self.assertEqual(allowed.returncode, 0, allowed.stderr)

    def test_install_hook_refuses_unmanaged_hook(self) -> None:
        hooks = Path(self.git("rev-parse", "--path-format=absolute", "--git-path", "hooks").stdout.strip())
        hooks.mkdir(parents=True, exist_ok=True)
        (hooks / "pre-commit").write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
        result = self.tool("install-hook", "--repo", str(self.repo), "--apply", "--json", check=False)
        self.assertEqual(result.returncode, 2)
        self.assertEqual(json.loads(result.stderr)["error"]["code"], "existing-hook")

    def test_install_hook_refuses_shared_external_hooks_path(self) -> None:
        external = self.root / "shared-hooks"
        external.mkdir()
        self.git("config", "core.hooksPath", str(external))
        result = self.tool("install-hook", "--repo", str(self.repo), "--apply", "--json", check=False)
        self.assertEqual(result.returncode, 2)
        self.assertEqual(json.loads(result.stderr)["error"]["code"], "external-hooks-path")


if __name__ == "__main__":
    unittest.main()
