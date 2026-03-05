#!/usr/bin/env python3
from __future__ import annotations

import argparse
import datetime as dt
import json
from pathlib import Path


def _iso_now(now_ms: int | None = None) -> str:
    if now_ms is None:
        now = dt.datetime.now(dt.timezone.utc)
    else:
        now = dt.datetime.fromtimestamp(now_ms / 1000, tz=dt.timezone.utc)
    return now.replace(microsecond=0).isoformat()


def _safe_rate(numerator: int, denominator: int) -> str:
    if denominator <= 0:
        return "0.00%"
    return f"{(numerator / denominator) * 100:.2f}%"


def load_metrics(base_dir: Path) -> dict[str, object]:
    path = base_dir / "onboard-metrics.json"
    if not path.exists():
        return {
            "attempted_first_task": 0,
            "first_task_success": 0,
            "first_task_failed": 0,
            "failure_by_category": {},
            "updated_at_ms": 0,
        }
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        return {
            "attempted_first_task": 0,
            "first_task_success": 0,
            "first_task_failed": 0,
            "failure_by_category": {},
            "updated_at_ms": 0,
        }


def load_events(base_dir: Path) -> list[dict[str, object]]:
    path = base_dir / "onboard-events.jsonl"
    if not path.exists():
        return []

    rows: list[dict[str, object]] = []
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line:
            continue
        try:
            obj = json.loads(line)
        except Exception:
            continue
        if isinstance(obj, dict):
            rows.append(obj)
    return rows


def _date_from_ms(ts_ms: int) -> str:
    return dt.datetime.fromtimestamp(ts_ms / 1000, tz=dt.timezone.utc).date().isoformat()


def summarize_daily(events: list[dict[str, object]], days: int, now_ms: int) -> list[dict[str, object]]:
    now_date = dt.datetime.fromtimestamp(now_ms / 1000, tz=dt.timezone.utc).date()
    start_date = now_date - dt.timedelta(days=days - 1)

    slots: dict[str, dict[str, object]] = {}
    for i in range(days):
        d = (start_date + dt.timedelta(days=i)).isoformat()
        slots[d] = {
            "date": d,
            "attempted": 0,
            "success": 0,
            "failed": 0,
            "success_rate": "0.00%",
        }

    for row in events:
        ts_ms = row.get("ts_ms")
        if not isinstance(ts_ms, int):
            continue
        day = _date_from_ms(ts_ms)
        slot = slots.get(day)
        if slot is None:
            continue
        slot["attempted"] = int(slot["attempted"]) + 1
        outcome = str(row.get("outcome", "")).strip().lower()
        if outcome == "success":
            slot["success"] = int(slot["success"]) + 1
        else:
            slot["failed"] = int(slot["failed"]) + 1

    out: list[dict[str, object]] = []
    for day in sorted(slots.keys()):
        row = slots[day]
        attempted = int(row["attempted"])
        success = int(row["success"])
        row["success_rate"] = _safe_rate(success, attempted)
        out.append(row)
    return out


def summarize_recent_window(
    events: list[dict[str, object]], window_hours: int, now_ms: int
) -> dict[str, object]:
    cutoff_ms = now_ms - window_hours * 60 * 60 * 1000
    recent = [
        row
        for row in events
        if isinstance(row.get("ts_ms"), int) and int(row["ts_ms"]) >= cutoff_ms
    ]

    attempted = len(recent)
    success = sum(1 for row in recent if str(row.get("outcome", "")).lower() == "success")
    failed = attempted - success

    by_category: dict[str, int] = {}
    for row in recent:
        if str(row.get("outcome", "")).lower() == "success":
            continue
        category = str(row.get("failure_category", "unknown")).strip() or "unknown"
        by_category[category] = by_category.get(category, 0) + 1

    return {
        "window_hours": window_hours,
        "attempted": attempted,
        "success": success,
        "failed": failed,
        "success_rate": _safe_rate(success, attempted),
        "failure_by_category": dict(sorted(by_category.items(), key=lambda kv: (-kv[1], kv[0]))),
    }


def build_report(base_dir: Path, days: int, window_hours: int, now_ms: int | None = None) -> dict[str, object]:
    now_ms = now_ms or int(dt.datetime.now(dt.timezone.utc).timestamp() * 1000)
    metrics = load_metrics(base_dir)
    events = load_events(base_dir)

    attempted_total = int(metrics.get("attempted_first_task", 0) or 0)
    success_total = int(metrics.get("first_task_success", 0) or 0)
    failed_total = int(metrics.get("first_task_failed", 0) or 0)

    report = {
        "generated_at": _iso_now(now_ms),
        "base_dir": str(base_dir),
        "metrics_snapshot": {
            "attempted_first_task": attempted_total,
            "first_task_success": success_total,
            "first_task_failed": failed_total,
            "success_rate": _safe_rate(success_total, attempted_total),
            "failure_by_category": metrics.get("failure_by_category", {}),
            "updated_at_ms": int(metrics.get("updated_at_ms", 0) or 0),
        },
        "recent_window": summarize_recent_window(events, window_hours=window_hours, now_ms=now_ms),
        "daily": summarize_daily(events, days=days, now_ms=now_ms),
        "event_count": len(events),
    }
    return report


def render_markdown(report: dict[str, object]) -> str:
    snapshot = report.get("metrics_snapshot", {})
    recent = report.get("recent_window", {})

    lines: list[str] = []
    lines.append("# Onboard Metrics Report")
    lines.append("")
    lines.append(f"- Generated: {report.get('generated_at', '')}")
    lines.append(f"- Base dir: `{report.get('base_dir', '')}`")
    lines.append(f"- Events loaded: {report.get('event_count', 0)}")
    lines.append("")

    lines.append("## Metrics Snapshot")
    lines.append("")
    lines.append(f"- Attempted: {snapshot.get('attempted_first_task', 0)}")
    lines.append(f"- Success: {snapshot.get('first_task_success', 0)}")
    lines.append(f"- Failed: {snapshot.get('first_task_failed', 0)}")
    lines.append(f"- Success rate: {snapshot.get('success_rate', '0.00%')}")
    lines.append("")

    lines.append(f"## Recent Window (Last {recent.get('window_hours', 24)}h)")
    lines.append("")
    lines.append(f"- Attempted: {recent.get('attempted', 0)}")
    lines.append(f"- Success: {recent.get('success', 0)}")
    lines.append(f"- Failed: {recent.get('failed', 0)}")
    lines.append(f"- Success rate: {recent.get('success_rate', '0.00%')}")
    lines.append("")

    lines.append("### Recent Failure Categories")
    lines.append("")
    lines.append("| Category | Count |")
    lines.append("|---|---:|")
    failure_by_category = recent.get("failure_by_category", {})
    if isinstance(failure_by_category, dict) and failure_by_category:
        for k, v in failure_by_category.items():
            lines.append(f"| {k} | {int(v)} |")
    else:
        lines.append("| (none) | 0 |")
    lines.append("")

    lines.append("## Daily Trend")
    lines.append("")
    lines.append("| Date (UTC) | Attempted | Success | Failed | Success rate |")
    lines.append("|---|---:|---:|---:|---:|")
    daily = report.get("daily", [])
    if isinstance(daily, list) and daily:
        for row in daily:
            if not isinstance(row, dict):
                continue
            lines.append(
                "| {date} | {attempted} | {success} | {failed} | {rate} |".format(
                    date=row.get("date", ""),
                    attempted=int(row.get("attempted", 0)),
                    success=int(row.get("success", 0)),
                    failed=int(row.get("failed", 0)),
                    rate=row.get("success_rate", "0.00%"),
                )
            )
    else:
        lines.append("| (no data) | 0 | 0 | 0 | 0.00% |")
    lines.append("")

    return "\n".join(lines)


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(
        description="Generate onboarding success/failure daily report from ~/.rexos metrics/events"
    )
    parser.add_argument(
        "--base-dir",
        default=str(Path.home() / ".rexos"),
        help="LoopForge data dir (default: ~/.rexos)",
    )
    parser.add_argument(
        "--out-dir",
        default=".tmp/onboard-report",
        help="Output directory for report files (default: .tmp/onboard-report)",
    )
    parser.add_argument(
        "--days",
        type=int,
        default=7,
        help="Number of UTC days in daily trend table (default: 7)",
    )
    parser.add_argument(
        "--window-hours",
        type=int,
        default=24,
        help="Rolling window size in hours (default: 24)",
    )
    args = parser.parse_args(argv)

    days = max(1, int(args.days))
    window_hours = max(1, int(args.window_hours))
    base_dir = Path(args.base_dir).expanduser().resolve()
    out_dir = Path(args.out_dir).resolve()
    out_dir.mkdir(parents=True, exist_ok=True)

    report = build_report(base_dir=base_dir, days=days, window_hours=window_hours)

    json_path = out_dir / "onboard-report.json"
    md_path = out_dir / "onboard-report.md"
    json_path.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    md_path.write_text(render_markdown(report) + "\n", encoding="utf-8")

    print(f"wrote: {json_path}")
    print(f"wrote: {md_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(__import__("sys").argv[1:]))
