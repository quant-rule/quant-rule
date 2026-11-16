#!/usr/bin/env python3
"""Unit tests for the composed-tier preflight.

The composed tier itself needs Docker and the real stack, so it is not part of
the fast tier. These tests exercise the gateway checks the tier performs first,
against a loopback server, and seed the two ways the preflight must refuse: an
unreachable gateway and an unhealthy one. The run-parity suite is replaced with
a stub so no container is required.
"""

from __future__ import annotations

import http.server
import json
import os
import subprocess
import sys
import tempfile
import threading
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SCRIPT = ROOT / "tools" / "composed.sh"


class Handler(http.server.BaseHTTPRequestHandler):
    """Serve a fixed health and version document for one test."""

    health: dict[str, object] = {}
    version: dict[str, object] = {}

    def do_GET(self) -> None:  # noqa: N802 - the stdlib hook name
        if self.path.endswith("/health"):
            document = self.health
        elif self.path.endswith("/version"):
            document = self.version
        else:
            self.send_error(404)
            return
        body = json.dumps(document).encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *args: object) -> None:
        pass


def serve(health: dict[str, object], version: dict[str, object]) -> tuple[http.server.ThreadingHTTPServer, int]:
    handler = type("SeededHandler", (Handler,), {"health": health, "version": version})
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    return server, server.server_address[1]


def run_composed(api_url: str) -> subprocess.CompletedProcess[str]:
    environment = os.environ.copy()
    environment.update(
        {
            "QUANTRULE_API_URL": api_url,
            "QUANTRULE_TIMEOUT": "1",
            "QUANTRULE_VERIFY": "/bin/true",
            "QR_PYTHON": sys.executable,
        }
    )
    return subprocess.run(
        ["bash", str(SCRIPT)],
        cwd=ROOT,
        env=environment,
        capture_output=True,
        text=True,
    )


class ComposedPreflightTests(unittest.TestCase):
    def test_a_healthy_gateway_passes_the_preflight(self) -> None:
        server, port = serve(
            {"v": "quantrule.api/v1", "state": "healthy"},
            {"api": "quantrule.api/v1", "product": "1.1.0"},
        )
        try:
            result = run_composed(f"http://127.0.0.1:{port}/api/v1")
        finally:
            server.shutdown()
            server.server_close()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("run parity verified", result.stdout)

    def test_an_unhealthy_gateway_is_reported(self) -> None:
        server, port = serve(
            {"v": "quantrule.api/v1", "state": "degraded"},
            {"api": "quantrule.api/v1", "product": "1.1.0"},
        )
        try:
            result = run_composed(f"http://127.0.0.1:{port}/api/v1")
        finally:
            server.shutdown()
            server.server_close()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("not 'healthy'", result.stderr)

    def test_a_missing_product_version_is_reported(self) -> None:
        server, port = serve(
            {"v": "quantrule.api/v1", "state": "healthy"},
            {"api": "quantrule.api/v1"},
        )
        try:
            result = run_composed(f"http://127.0.0.1:{port}/api/v1")
        finally:
            server.shutdown()
            server.server_close()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("no product version", result.stderr)

    def test_an_unreachable_gateway_is_reported(self) -> None:
        result = run_composed("http://127.0.0.1:1/api/v1")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("not reachable", result.stderr)


if __name__ == "__main__":
    unittest.main()
