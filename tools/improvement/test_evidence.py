#!/usr/bin/env python3
"""Unit tests for the evidence extraction shapes."""

from __future__ import annotations

import unittest

from common import model_metrics, portfolio_metrics


class PortfolioMetricsTests(unittest.TestCase):
    def test_analytics_metrics_shape(self) -> None:
        report = {
            "metrics": {
                "total_return": 0.1,
                "max_drawdown": 0.2,
                "trade_count": 5,
                "profit_factor": 1.5,
                "win_rate": 0.6,
                "halted": False,
                "final_equity": 11000.0,
                "exposure": 0.4,
            }
        }
        values, notes = portfolio_metrics(report)
        self.assertEqual(values["return"], 0.1)
        self.assertEqual(values["trades"], 5)
        self.assertEqual(values["final_equity"], 11000.0)
        self.assertEqual(values["exposure"], 0.4)
        self.assertTrue(all("missing" in note for note in notes))

    def test_portfolio_metrics_shape(self) -> None:
        report = {
            "total_return": -0.1,
            "max_drawdown": 0.3,
            "trade_count": 2,
            "profit_factor": 0.0,
            "win_rate": 0.0,
            "halted": True,
            "final_equity": 9000.0,
        }
        values, notes = portfolio_metrics(report)
        self.assertEqual(values["return"], -0.1)
        self.assertEqual(values["trades"], 2)
        self.assertTrue(values["halted"])
        self.assertIn("report.exposure is missing", notes)

    def test_historical_summary_shape(self) -> None:
        report = {
            "summary": {
                "total_return": 0.2,
                "max_drawdown": 0.1,
                "trades": 3,
                "profit_factor": 2.0,
                "win_rate": 0.5,
                "halted": False,
                "final_equity": 12000.0,
            }
        }
        values, notes = portfolio_metrics(report)
        self.assertEqual(values["return"], 0.2)
        self.assertEqual(values["trades"], 3)
        self.assertEqual(values["final_equity"], 12000.0)

    def test_missing_values_are_null(self) -> None:
        values, notes = portfolio_metrics({})
        self.assertIsNone(values["return"])
        self.assertTrue(notes)


class ModelMetricsTests(unittest.TestCase):
    def test_model_report_shape(self) -> None:
        report = {
            "model_version": "M1.0",
            "view_contract": "2026-11-05",
            "data_version": "test:v1",
            "requested_period": {"from": "2025-01-01", "to": "2025-01-02"},
            "market_report": {
                "ladder": [60, 300],
                "timeframes": [
                    {
                        "timeframe_secs": 60,
                        "bars": 10,
                        "missing_bars": 0,
                        "coverage_share": 1.0,
                    }
                ],
            },
        }
        values, notes = model_metrics(report)
        self.assertEqual(values["model_version"], "M1.0")
        self.assertEqual(values["ladder_seconds"], [60, 300])
        self.assertEqual(len(values["timeframes"]), 1)
        self.assertEqual(notes, [])


if __name__ == "__main__":
    unittest.main()
