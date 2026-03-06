import datetime as dt
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


def load_module():
    module_path = Path(__file__).resolve().parents[1] / "onboard_metrics_report.py"
    spec = importlib.util.spec_from_file_location("onboard_metrics_report", module_path)
    module = importlib.util.module_from_spec(spec)
    assert spec is not None and spec.loader is not None
    spec.loader.exec_module(module)
    return module


mod = load_module()


class OnboardMetricsReportTests(unittest.TestCase):
    @staticmethod
    def _ms(y: int, m: int, d: int, h: int = 0, minute: int = 0) -> int:
        return int(
            dt.datetime(y, m, d, h, minute, tzinfo=dt.timezone.utc).timestamp() * 1000
        )

    def _write_fixture(self, base_dir: Path):
        base_dir.mkdir(parents=True, exist_ok=True)

        metrics = {
            "attempted_first_task": 10,
            "first_task_success": 7,
            "first_task_failed": 3,
            "failure_by_category": {
                "model_unavailable": 2,
                "provider_unreachable": 1,
            },
            "updated_at_ms": 1772755200000,
        }
        (base_dir / "onboard-metrics.json").write_text(
            json.dumps(metrics, indent=2) + "\n", encoding="utf-8"
        )

        events = [
            {
                "ts_ms": self._ms(2026, 3, 3, 0, 0),
                "workspace": "w1",
                "session_id": "s1",
                "outcome": "success",
            },
            {
                "ts_ms": self._ms(2026, 3, 3, 12, 0),
                "workspace": "w2",
                "session_id": "s2",
                "outcome": "failed",
                "failure_category": "model_unavailable",
                "error": "model not found",
            },
            {
                "ts_ms": self._ms(2026, 3, 5, 0, 0),
                "workspace": "w3",
                "session_id": "s3",
                "outcome": "success",
            },
            {
                "ts_ms": self._ms(2026, 3, 5, 10, 0),
                "workspace": "w4",
                "session_id": "s4",
                "outcome": "failed",
                "failure_category": "provider_unreachable",
                "error": "timed out",
            },
        ]
        with (base_dir / "onboard-events.jsonl").open("w", encoding="utf-8") as f:
            for row in events:
                f.write(json.dumps(row, ensure_ascii=False) + "\n")

    def test_build_report_aggregates_recent_window_and_daily(self):
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp) / ".loopforge"
            self._write_fixture(base)

            now_ms = int(
                dt.datetime(2026, 3, 6, 0, 0, tzinfo=dt.timezone.utc).timestamp() * 1000
            )
            report = mod.build_report(
                base_dir=base,
                days=4,
                window_hours=24,
                now_ms=now_ms,
            )

            recent = report["recent_window"]
            self.assertEqual(recent["attempted"], 2)
            self.assertEqual(recent["success"], 1)
            self.assertEqual(recent["failed"], 1)
            self.assertEqual(recent["success_rate"], "50.00%")
            self.assertEqual(recent["failure_by_category"]["provider_unreachable"], 1)

            recommendations = report["recommendations"]
            self.assertTrue(recommendations)
            self.assertEqual(recommendations[0]["category"], "provider_unreachable")
            self.assertIn("ollama serve", recommendations[0]["suggestion"])

            daily = report["daily"]
            self.assertEqual(len(daily), 4)
            row_0303 = [r for r in daily if r["date"] == "2026-03-03"][0]
            self.assertEqual(row_0303["attempted"], 2)
            self.assertEqual(row_0303["success"], 1)
            self.assertEqual(row_0303["failed"], 1)

    def test_render_markdown_contains_core_sections(self):
        report = {
            "generated_at": "2026-03-06T00:00:00+00:00",
            "base_dir": "/tmp/.loopforge",
            "metrics_snapshot": {
                "attempted_first_task": 10,
                "first_task_success": 7,
                "first_task_failed": 3,
                "success_rate": "70.00%",
            },
            "recent_window": {
                "window_hours": 24,
                "attempted": 3,
                "success": 2,
                "failed": 1,
                "success_rate": "66.67%",
                "failure_by_category": {"model_unavailable": 1},
            },
            "recommendations": [
                {
                    "category": "model_unavailable",
                    "count": 1,
                    "suggestion": "Run `ollama list` and set a chat model that exists locally.",
                }
            ],
            "daily": [
                {
                    "date": "2026-03-05",
                    "attempted": 3,
                    "success": 2,
                    "failed": 1,
                    "success_rate": "66.67%",
                }
            ],
        }
        md = mod.render_markdown(report)
        self.assertIn("# Onboard Metrics Report", md)
        self.assertIn("## Metrics Snapshot", md)
        self.assertIn("## Recent Window (Last 24h)", md)
        self.assertIn("## Recommended Fixes", md)
        self.assertIn("## Daily Trend", md)
        self.assertIn("model_unavailable", md)
        self.assertIn("ollama list", md)

    def test_main_writes_json_and_markdown(self):
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp) / ".loopforge"
            out = Path(tmp) / "out"
            self._write_fixture(base)

            exit_code = mod.main(
                [
                    "--base-dir",
                    str(base),
                    "--out-dir",
                    str(out),
                    "--days",
                    "3",
                    "--window-hours",
                    "24",
                ]
            )
            self.assertEqual(exit_code, 0)
            self.assertTrue((out / "onboard-report.json").exists())
            self.assertTrue((out / "onboard-report.md").exists())


if __name__ == "__main__":
    unittest.main()
