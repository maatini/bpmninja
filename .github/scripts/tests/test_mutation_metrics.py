#!/usr/bin/env python3
import json
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_DIR))

import build_mutation_metrics  # noqa: E402
import merge_quality_metrics  # noqa: E402


class ParseOutcomesTest(unittest.TestCase):
    def test_modern_summary_field(self) -> None:
        payload = {
            "outcomes": [
                {"summary": "Success", "scenario": "Baseline"},
                {"summary": "CaughtMutant"},
                {"summary": "CaughtMutant"},
                {"summary": "MissedMutant"},
                {"summary": "Timeout"},
                {"summary": "Unviable"},
            ]
        }
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "outcomes.json"
            path.write_text(json.dumps(payload), encoding="utf-8")
            caught, missed, timeout, unviable = build_mutation_metrics.parse_outcomes_file(path)
        self.assertEqual((caught, missed, timeout, unviable), (2, 1, 1, 1))

    def test_zero_toplevel_counters_do_not_hide_outcomes(self) -> None:
        payload = {
            "caught": 0,
            "missed": 0,
            "timeout": 0,
            "unviable": 0,
            "outcomes": [
                {"summary": "CaughtMutant"},
                {"summary": "CaughtMutant"},
                {"summary": "MissedMutant"},
            ],
        }
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "outcomes.json"
            path.write_text(json.dumps(payload), encoding="utf-8")
            caught, missed, timeout, unviable = build_mutation_metrics.parse_outcomes_file(path)
        self.assertEqual((caught, missed, timeout, unviable), (2, 1, 0, 0))

    def test_txt_fallback(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            out = root / "mutants.out"
            out.mkdir()
            (out / "caught.txt").write_text("a.rs:1\nb.rs:2\n", encoding="utf-8")
            (out / "missed.txt").write_text("c.rs:3\n", encoding="utf-8")
            (out / "timeout.txt").write_text("", encoding="utf-8")
            (out / "unviable.txt").write_text("# comment\n", encoding="utf-8")
            metrics = build_mutation_metrics.build_metrics(root)
        self.assertEqual(metrics["status"], "ok")
        self.assertEqual(metrics["caught"], 2)
        self.assertEqual(metrics["missed"], 1)
        self.assertEqual(metrics["total_tested"], 3)
        self.assertAlmostEqual(metrics["mutation_score"], 66.7)

    def test_nested_summary_object(self) -> None:
        payload = {
            "summary": {"caught": 10, "missed": 2, "timeout": 1, "unviable": 3},
            "outcomes": [],
        }
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "outcomes.json"
            path.write_text(json.dumps(payload), encoding="utf-8")
            counts = build_mutation_metrics.parse_outcomes_file(path)
        self.assertEqual(counts, (10, 2, 1, 3))


class MergeMetricsTest(unittest.TestCase):
    def test_keeps_previous_mutation_when_incoming_is_empty(self) -> None:
        existing = {
            "mutation": {
                "status": "ok",
                "mutation_score": 72.4,
                "caught": 100,
                "missed": 38,
                "timeout": 0,
                "unviable": 4,
                "total_tested": 138,
            },
            "fuzz": {"status": "ok", "targets_total": 9, "targets_passed": 9, "targets_failed": 0},
        }
        incoming = {
            "status": "no-data",
            "mutation_score": 0.0,
            "caught": 0,
            "missed": 0,
            "timeout": 0,
            "unviable": 0,
            "total_tested": 0,
        }
        merged = merge_quality_metrics.merge_metrics(existing, None, incoming)
        self.assertEqual(merged["mutation"]["mutation_score"], 72.4)
        self.assertTrue(merged["mutation"]["retained"])

    def test_replaces_when_incoming_has_signal(self) -> None:
        existing = {
            "mutation": {
                "status": "ok",
                "mutation_score": 50.0,
                "caught": 5,
                "missed": 5,
                "timeout": 0,
                "unviable": 0,
                "total_tested": 10,
            }
        }
        incoming = {
            "status": "ok",
            "mutation_score": 80.0,
            "caught": 8,
            "missed": 2,
            "timeout": 0,
            "unviable": 0,
            "total_tested": 10,
        }
        merged = merge_quality_metrics.merge_metrics(existing, None, incoming)
        self.assertEqual(merged["mutation"]["mutation_score"], 80.0)


if __name__ == "__main__":
    unittest.main()
