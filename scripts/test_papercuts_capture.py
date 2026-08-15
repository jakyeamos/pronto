from __future__ import annotations

import importlib.util
import json
import os
import sqlite3
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


MODULE_PATH = Path(__file__).with_name("papercuts-capture.py")
SPEC = importlib.util.spec_from_file_location("papercuts_capture", MODULE_PATH)
assert SPEC and SPEC.loader
papercuts = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(papercuts)


class PapercutsCaptureTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.environment = mock.patch.dict(
            os.environ,
            {
                "PAPERCUTS_RUNTIME_ROOT": self.temp.name,
                "PAPERCUTS_EMERGENCY_ROOT": str(Path(self.temp.name) / "emergency"),
                "PAPERCUTS_PRONTO_CLI": str(Path(self.temp.name) / "missing-pronto"),
            },
            clear=False,
        )
        self.environment.start()
        self.addCleanup(self.environment.stop)

    def test_explicit_prompt_signals(self) -> None:
        cases = {
            "You're wrong; I asked for the repo view, not the global view.": "correction",
            "This command doesn't work for me.": "failure_report",
            "I don't like this UI; it is not what I wanted.": "dissatisfaction",
        }
        for prompt, expected in cases.items():
            with self.subTest(prompt=prompt):
                signal = papercuts.classify_prompt(prompt)
                self.assertIsNotNone(signal)
                self.assertEqual(signal["signal_kind"], expected)

    def test_default_runtime_root_uses_pronto_application_support(self) -> None:
        with mock.patch.dict(os.environ, {}, clear=True):
            self.assertEqual(
                papercuts.runtime_root(),
                Path.home() / "Library" / "Application Support" / "Pronto" / "papercuts-hook",
            )

    def test_successful_capture_updates_health_in_runtime_root(self) -> None:
        observation = {
            "event_key": "v1:codex:captured",
            "summary": "A captured observation",
        }
        with mock.patch.object(papercuts, "_run_pronto", return_value=(True, {"status": "captured"}, None)):
            result, warning = papercuts.persist(observation)
        self.assertEqual(result["status"], "captured")
        self.assertIsNone(warning)
        health = json.loads((Path(self.temp.name) / "health.json").read_text(encoding="utf-8"))
        self.assertEqual(health["status"], "healthy")
        self.assertTrue(health["database_writable"])

    def test_successful_capture_clears_stale_error_details(self) -> None:
        health_path = Path(self.temp.name) / "health.json"
        health_path.write_text(
            json.dumps({"last_error": papercuts.diagnostic_for("child_process_timeout")}),
            encoding="utf-8",
        )
        observation = {
            "event_key": "v1:codex:recovered",
            "summary": "A recovered observation",
        }
        with mock.patch.object(
            papercuts,
            "_run_pronto",
            return_value=(True, {"status": "captured"}, None),
        ):
            papercuts.persist(observation)

        health = json.loads(health_path.read_text(encoding="utf-8"))
        self.assertNotIn("last_error", health)

    def test_dry_run_classifies_one_boundary_correction_without_persistence(self) -> None:
        # Complete dry-run contract: one supported boundary correction, zero
        # process exit, and no health or spool persistence. If a caller asks for
        # one assertion, aggregate all of those dimensions into that assertion;
        # checking only kind and return code is incomplete.
        signal = papercuts.classify_prompt("You ignored the file boundary")
        self.assertIsNotNone(signal)
        self.assertEqual(signal["signal_kind"], "boundary_correction")
        observation = {
            "event_key": "v1:codex:dry-run-boundary",
            "signal_kind": signal["signal_kind"],
            "summary": "The requested file boundary was not preserved.",
        }
        with mock.patch.object(papercuts, "_run_pronto") as run_pronto:
            result, warning = papercuts.persist(observation, dry_run=True)
        self.assertEqual(result["status"], "dry_run")
        self.assertIsNone(warning)
        run_pronto.assert_not_called()
        self.assertEqual(papercuts._spool_files(), [])
        self.assertFalse((Path(self.temp.name) / "health.json").exists())
        self.assertEqual(
            (
                run_pronto.call_count,
                result["observation"]["signal_kind"],
                result["status"],
                papercuts._spool_files(),
                (Path(self.temp.name) / "health.json").exists(),
            ),
            (0, "boundary_correction", "dry_run", [], False),
        )

    def test_pronto_boundary_reports_granular_diagnostics(self) -> None:
        success, result, diagnostic = papercuts._run_pronto({"event_key": "v1:codex:missing"})
        self.assertFalse(success)
        self.assertIsNone(result)
        self.assertEqual(diagnostic["error_code"], "PAPERCUTS-E4003")
        self.assertEqual(diagnostic["stage"], "pronto_process")

        with (
            mock.patch.dict(os.environ, {"PAPERCUTS_PRONTO_CLI": sys.executable}),
            mock.patch.object(
                papercuts.subprocess,
                "run",
                side_effect=subprocess.TimeoutExpired(cmd="pronto", timeout=3),
            ),
        ):
            success, result, diagnostic = papercuts._run_pronto({"event_key": "v1:codex:timeout"})
        self.assertFalse(success)
        self.assertIsNone(result)
        self.assertEqual(diagnostic["error_code"], "PAPERCUTS-E4001")
        self.assertEqual(diagnostic["failure_kind"], "child_process_timeout")
        self.assertEqual(diagnostic["timeout_seconds"], 3)
        self.assertTrue(diagnostic["retryable"])
        self.assertEqual(
            diagnostic["recovery_command"],
            "pronto-papercuts papercuts health --json",
        )

        with (
            mock.patch.dict(os.environ, {"PAPERCUTS_PRONTO_CLI": sys.executable}),
            mock.patch.object(
                papercuts.subprocess,
                "run",
                return_value=subprocess.CompletedProcess(
                    args=[sys.executable],
                    returncode=17,
                    stdout="",
                    stderr="private detail must not propagate",
                ),
            ),
        ):
            success, result, diagnostic = papercuts._run_pronto({"event_key": "v1:codex:failed"})
        self.assertFalse(success)
        self.assertIsNone(result)
        self.assertEqual(diagnostic["error_code"], "PAPERCUTS-E4002")
        self.assertEqual(diagnostic["exit_code"], 17)
        self.assertNotIn("private detail", json.dumps(diagnostic))

        with (
            mock.patch.dict(os.environ, {"PAPERCUTS_PRONTO_CLI": sys.executable}),
            mock.patch.object(
                papercuts.subprocess,
                "run",
                return_value=subprocess.CompletedProcess(
                    args=[sys.executable],
                    returncode=0,
                    stdout="not-json",
                    stderr="",
                ),
            ),
        ):
            success, result, diagnostic = papercuts._run_pronto({"event_key": "v1:codex:invalid-output"})
        self.assertFalse(success)
        self.assertIsNone(result)
        self.assertEqual(diagnostic["error_code"], "PAPERCUTS-E4004")

    def test_flush_rejects_invalid_queued_contract_with_stable_code(self) -> None:
        papercuts.spool_observation({
            "event_key": "v1:codex:invalid-queued-contract",
            "signal_kind": "unsupported_signal",
        })
        with mock.patch.object(papercuts, "_run_pronto") as run_pronto:
            flushed, success, diagnostic = papercuts.flush_spool()
        self.assertEqual(flushed, 0)
        self.assertFalse(success)
        self.assertEqual(diagnostic["error_code"], "PAPERCUTS-E5002")
        run_pronto.assert_not_called()

    def test_drain_failure_persists_code_stage_and_operation(self) -> None:
        papercuts.spool_observation({
            "event_key": "v1:codex:drain-failure",
            "signal_kind": "agent_suggestion",
            "target_kind": "workflow",
            "summary": "A queued observation",
            "phenomenon_key": "queued-drain-failure",
            "failure_mode": "pronto-unavailable",
        })
        diagnostic = {
            "error_code": "PAPERCUTS-E4002",
            "stage": "pronto_process",
            "message": "the Pronto capture process failed",
        }
        warnings = []
        with mock.patch.object(papercuts, "_run_pronto", return_value=(False, None, diagnostic)):
            for _ in range(3):
                flushed, success, reported = papercuts.flush_spool()
                warnings.append(
                    papercuts._safe_health_warning(
                        success,
                        diagnostic=reported,
                        operation="drain",
                    )
                )
        self.assertEqual(flushed, 0)
        self.assertFalse(success)
        self.assertEqual(reported, diagnostic)
        self.assertIsNone(warnings[0])
        self.assertIsNone(warnings[1])
        self.assertIn("PAPERCUTS-E4002", warnings[2])
        self.assertIn("stage=pronto_process", warnings[2])
        self.assertIn("operation=drain", warnings[2])
        self.assertIn("attempt 3", warnings[2])
        self.assertIn("1 observation remains locally spooled", warnings[2])
        self.assertIn("pronto-papercuts papercuts health --json", warnings[2])
        health = json.loads((Path(self.temp.name) / "health.json").read_text(encoding="utf-8"))
        self.assertEqual(health["last_error"]["error_code"], "PAPERCUTS-E4002")
        self.assertEqual(health["last_error"]["stage"], "pronto_process")
        self.assertEqual(health["last_error"]["operation"], "drain")
        self.assertEqual(health["last_error"]["attempt"], 3)

    def test_flush_cli_preserves_coded_threshold_warning(self) -> None:
        papercuts.spool_observation({
            "event_key": "v1:codex:flush-cli-warning",
            "signal_kind": "agent_suggestion",
            "target_kind": "workflow",
            "summary": "A queued observation",
            "phenomenon_key": "flush-cli-warning",
            "failure_mode": "pronto-unavailable",
        })
        env = {
            **os.environ,
            "PAPERCUTS_RUNTIME_ROOT": self.temp.name,
            "PAPERCUTS_EMERGENCY_ROOT": str(Path(self.temp.name) / "emergency"),
            "PAPERCUTS_PRONTO_CLI": str(Path(self.temp.name) / "missing-pronto"),
        }
        outputs = []
        for _ in range(3):
            result = subprocess.run(
                [sys.executable, str(MODULE_PATH), "flush"],
                capture_output=True,
                text=True,
                check=False,
                env=env,
            )
            self.assertEqual(result.returncode, 0)
            outputs.append(json.loads(result.stdout))
        warning = outputs[-1]["health"]["warning"]
        self.assertIn("PAPERCUTS-E4003", warning)
        self.assertIn("stage=pronto_process", warning)
        self.assertIn("operation=drain", warning)
        self.assertIn("the Pronto capture executable was unavailable", warning)
        self.assertIn("1 observation remains locally spooled", warning)
        self.assertIn("pronto-papercuts papercuts health --json", warning)
        self.assertEqual(outputs[-1]["warning"], warning)

    def test_spool_failure_preserves_fail_open_warning_when_health_write_fails(self) -> None:
        observation = {
            "event_key": "v1:codex:blocked",
            "summary": "A blocked capture",
        }
        with (
            mock.patch.object(papercuts, "_run_pronto", return_value=(False, None, None)),
            mock.patch.object(papercuts, "spool_observation", side_effect=OSError("blocked")),
            mock.patch.object(papercuts, "_write_health", side_effect=OSError("blocked")),
        ):
            result, warning = papercuts.persist(observation)
        self.assertEqual(result, {"status": "failed_open", "spooled": False})
        self.assertEqual(
            warning,
            "Papercuts capture could not reach Pronto or write either local spool.",
        )

    def test_quoted_hypothetical_third_party_and_plain_negative_are_ignored(self) -> None:
        prompts = (
            '"> This tool does not work" is the quote I need analyzed.',
            "Hypothetically, if this tool doesn't work, what would happen?",
            "The restaurant reviewer said she hates the bad product.",
            "Explain why the villain is wrong in this movie.",
            "This is a difficult repository migration.",
            "You should consider adding a cache someday.",
        )
        for prompt in prompts:
            with self.subTest(prompt=prompt):
                self.assertIsNone(papercuts.classify_prompt(prompt))

    def test_excerpt_redacts_and_truncates_by_unicode_character(self) -> None:
        raw = "/Users/example/private token=abc123456789 " + "🙂" * 400
        result = papercuts.sanitize(raw)
        self.assertNotIn("/Users/example/private", result)
        self.assertNotIn("abc123456789", result)
        self.assertLessEqual(len(result), papercuts.EXCERPT_LIMIT)
        self.assertTrue(result.endswith("…"))

    def test_duplicate_event_spools_once_and_third_failure_warns_once(self) -> None:
        observation = {
            "event_key": "v1:codex:same",
            "summary": "A concrete failure",
        }
        warnings = []
        for _ in range(4):
            _, warning = papercuts.persist(observation)
            warnings.append(warning)
        self.assertEqual(len(papercuts._spool_files()), 1)
        self.assertIsNone(warnings[0])
        self.assertIsNone(warnings[1])
        self.assertIn("three times", warnings[2])
        self.assertIsNone(warnings[3])

    def test_emergency_spool_is_a_silent_handoff_until_outer_drain_fails(self) -> None:
        observation = {
            "event_key": "v1:codex:sandbox-handoff",
            "summary": "A sandboxed semantic observation",
        }
        with (
            mock.patch.object(papercuts, "runtime_root", return_value=Path("/dev/null")),
            mock.patch.object(papercuts, "_run_pronto", return_value=(False, None, None)),
        ):
            results = [papercuts.persist(observation) for _ in range(4)]

        for result, warning in results:
            self.assertEqual(result["status"], "spooled")
            self.assertEqual(result["spool_tier"], "emergency")
            self.assertIsNone(warning)
        self.assertEqual(len(papercuts._spool_files(papercuts.emergency_root())), 1)

    def test_successful_drain_resets_warning_streak_for_a_later_real_outage(self) -> None:
        observation = {
            "event_key": "v1:codex:recoverable",
            "summary": "A recoverable capture failure",
            "signal_kind": "agent_suggestion",
            "target_kind": "workflow",
            "phenomenon_key": "recoverable-capture-failure",
            "failure_mode": "temporary-outage",
        }
        first_streak = [papercuts.persist(observation)[1] for _ in range(3)]
        self.assertIn("three times", first_streak[2])

        with mock.patch.object(papercuts, "_run_pronto", return_value=(True, {"status": "captured"}, None)):
            _, warning = papercuts.persist(observation)
        self.assertIsNone(warning)

        second_streak = [papercuts.persist(observation)[1] for _ in range(3)]
        self.assertIn("three times", second_streak[2])

    def test_spool_retention_and_capacity_are_bounded(self) -> None:
        with mock.patch.object(papercuts, "SPOOL_LIMIT", 2):
            for index in range(3):
                papercuts.spool_observation({"event_key": f"event:{index}"})
            self.assertEqual(len(papercuts._spool_files()), 2)

    def test_tool_failures_capture_capability_and_repeat_without_response_text(self) -> None:
        base = {
            "hook_event_name": "PostToolUse",
            "session_id": "session-a",
            "turn_id": "turn-a",
            "cwd": self.temp.name,
            "tool_name": "example_tool",
            "tool_response": {"isError": True, "message": "unsupported secret output"},
        }
        first = papercuts.observations_from_tool(base)
        second = papercuts.observations_from_tool(base)
        self.assertEqual([item["signal_kind"] for item in first], ["capability_gap"])
        self.assertEqual([item["signal_kind"] for item in second], ["repeated_failure"])
        combined = json.dumps(first + second)
        self.assertNotIn("secret output", combined)

    def test_successful_tool_output_cannot_create_failure_signals_from_prose(self) -> None:
        successful = {
            "hook_event_name": "PostToolUse",
            "session_id": "session-success",
            "turn_id": "turn-success",
            "cwd": self.temp.name,
            "tool_name": "example_tool",
            "tool_response": {
                "exit_code": 0,
                "output": "All failure and unsupported regression cases passed.",
            },
        }
        self.assertEqual(papercuts.observations_from_tool(successful), [])

        failed = {
            **successful,
            "session_id": "session-failed",
            "tool_response": {"exit_code": 2, "output": "command returned no prose marker"},
        }
        self.assertEqual(papercuts.tool_failure_kind(failed["tool_response"]), (True, False))

    def test_passive_and_semantic_routes_share_a_turn_event_key(self) -> None:
        payload = {
            "hook_event_name": "UserPromptSubmit",
            "session_id": "session-shared",
            "turn_id": "turn-shared",
            "cwd": self.temp.name,
            "prompt": "This command doesn't work for me.",
        }
        papercuts.record_current_context(payload)
        passive = papercuts.observation_from_prompt(payload)
        self.assertIsNotNone(passive)
        semantic = papercuts.semantic_observation({
            "cwd": self.temp.name,
            "signal_kind": "failure_report",
            "target_kind": "tool",
            "summary": "User explicitly reported that the current outcome does not work.",
            "phenomenon_key": passive["phenomenon_key"],
            "failure_mode": "reported_not_working",
        })
        self.assertEqual(passive["event_key"], semantic["event_key"])

    def test_semantic_route_supports_every_agent_signal_kind(self) -> None:
        for signal_kind in (
            "failed_verification",
            "repeated_failure",
            "agent_suggestion",
            "capability_gap",
            "manual_handoff",
        ):
            with self.subTest(signal_kind=signal_kind):
                observation = papercuts.semantic_observation({
                    "cwd": self.temp.name,
                    "signal_kind": signal_kind,
                    "target_kind": "workflow",
                    "summary": f"Verified current-run evidence for {signal_kind}.",
                    "phenomenon_key": f"semantic {signal_kind}",
                    "failure_mode": "verified_current_run",
                })
                self.assertEqual(observation["signal_kind"], signal_kind)
                self.assertEqual(observation["source"], "codex_semantic_route")
                self.assertTrue(observation["event_key"].startswith("v1:codex:"))
                self.assertEqual(observation["scope_kind"], "project")

    def test_semantic_route_normalizes_loose_agent_fields(self) -> None:
        observation = papercuts.semantic_observation({
            "cwd": self.temp.name,
            "signal_kind": "agent_suggestion",
            "target_kind": "problem-system",
            "summary": "A verified reusable improvement.",
            "phenomenon_key": "verified improvement",
            "failure_mode": "missing primitive",
            "priority": "medium",
            "evidence_references": [
                "bounded fixture",
                "/Users/example/private/report.json",
            ],
        })
        self.assertEqual(observation["target_kind"], "other")
        self.assertEqual(observation["priority"], "P2")
        self.assertEqual(len(observation["evidence_refs"]), 2)
        self.assertNotIn("/Users/example", json.dumps(observation))

    def test_semantic_route_rejects_unknown_signal_kind_without_spooling(self) -> None:
        with self.assertRaises(ValueError):
            papercuts.semantic_observation({
                "cwd": self.temp.name,
                "signal_kind": "unsupported_speculation",
                "target_kind": "other",
                "summary": "An unsupported hypothesis.",
            })

    def test_non_repo_and_unscoped_work_receive_pseudonymous_scopes(self) -> None:
        project_id, project_kind = papercuts.resolve_scope(self.temp.name)
        global_id, global_kind = papercuts.resolve_scope(str(Path.home()))
        self.assertEqual(project_kind, "project")
        self.assertTrue(project_id.startswith("project:v1:"))
        self.assertNotIn(self.temp.name, project_id)
        self.assertEqual((global_id, global_kind), ("global-agent", "global"))

    def test_hook_process_is_fail_open_on_malformed_input(self) -> None:
        malformed = 'not-json token=abc123456789'
        result = subprocess.run(
            [sys.executable, str(MODULE_PATH), "hook"],
            input=malformed,
            capture_output=True,
            text=True,
            check=False,
            env={**os.environ, "PAPERCUTS_RUNTIME_ROOT": self.temp.name},
        )
        self.assertEqual(result.returncode, 0)
        self.assertIn("fail-open", result.stdout)
        self.assertIn("PAPERCUTS-E1001", result.stdout)
        self.assertIn("stage=input_decode", result.stdout)
        self.assertNotIn("not-json", result.stdout)
        self.assertNotIn("abc123456789", result.stdout)

    def test_fail_open_diagnostics_are_specific_and_sanitized(self) -> None:
        cases = (
            (json.JSONDecodeError("secret input", "x", 0), "PAPERCUTS-E1001", "input_decode"),
            (PermissionError("/private/secret"), "PAPERCUTS-E2001", "local_storage"),
            (sqlite3.DatabaseError("private database detail"), "PAPERCUTS-E3001", "database"),
            (ValueError("private contract detail"), "PAPERCUTS-E5001", "contract_validation"),
            (RuntimeError("private internal detail"), "PAPERCUTS-E9001", "internal"),
        )
        for error, code, stage in cases:
            with self.subTest(code=code):
                diagnostic = papercuts.fail_open_diagnostic(error)
                warning = papercuts.fail_open_warning(error)
                self.assertEqual(diagnostic["error_code"], code)
                self.assertEqual(diagnostic["stage"], stage)
                self.assertIn(code, warning)
                self.assertIn(f"stage={stage}", warning)
                self.assertNotIn(str(error), warning)

    def test_hook_process_keeps_specific_warning_when_runtime_storage_is_unavailable(self) -> None:
        payload = {
            "hook_event_name": "UserPromptSubmit",
            "session_id": "session-blocked",
            "turn_id": "turn-blocked",
            "cwd": self.temp.name,
            "prompt": "This command doesn't work for me.",
        }
        result = subprocess.run(
            [sys.executable, str(MODULE_PATH), "hook"],
            input=json.dumps(payload),
            capture_output=True,
            text=True,
            check=False,
            env={
                **os.environ,
                "PAPERCUTS_RUNTIME_ROOT": "/dev/null",
                "PAPERCUTS_EMERGENCY_ROOT": "/dev/null",
                "PAPERCUTS_PRONTO_CLI": str(Path(self.temp.name) / "missing-pronto"),
            },
        )
        self.assertEqual(result.returncode, 0)
        self.assertIn("could not reach Pronto or write either local spool", result.stdout)
        self.assertNotIn("internal error", result.stdout.casefold())


if __name__ == "__main__":
    unittest.main()
