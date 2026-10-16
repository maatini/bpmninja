"""Regressionstests für Lasttest-Zählung, Validierung und Transportfehler."""

import argparse
import json
import os
import platform
import threading
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

from http_load import Client, percentile, rss_reader, run_phase


class LoadTest(unittest.TestCase):
    def run_server(self, failure=None):
        class Handler(BaseHTTPRequestHandler):
            protocol_version = "HTTP/1.1"

            def log_message(self, *_):
                pass

            def respond(self, body, status=200):
                payload = json.dumps(body).encode()
                self.send_response(status)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(payload)))
                self.end_headers()
                self.wfile.write(payload)

            def do_POST(self):
                self.rfile.read(int(self.headers["Content-Length"]))
                if failure == "http":
                    self.respond({"error": "failure"}, 500)
                else:
                    self.respond({"instance_id": "test"})

            def do_GET(self):
                self.respond({"state": "Completed", "variables": {
                    "counter": 0 if failure == "result" else 3}})

        server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        self.addCleanup(server.server_close)
        self.addCleanup(server.shutdown)
        args = argparse.Namespace(base_url=f"http://127.0.0.1:{server.server_port}",
                                  timeout=2, concurrency=4, steps=3, payload_bytes=32)
        return run_phase(args, "definition", 11)

    def test_non_divisible_request_count(self):
        result = self.run_server()
        self.assertEqual(result["succeeded"], 11)
        self.assertEqual(result["failed"], 0)
        self.assertGreater(result["transactions_per_second"], 0)
        self.assertLessEqual(result["latency_ms"]["p50"], result["latency_ms"]["p99"])

    def test_http_errors_fail_every_attempt(self):
        result = self.run_server("http")
        self.assertEqual(result["failed"], 11)
        self.assertEqual(result["succeeded"], 0)
        self.assertIsNone(result["latency_ms"]["p99"])

    def test_incorrect_script_result_is_failure(self):
        self.assertEqual(self.run_server("result")["failed"], 11)

    def test_nearest_rank_percentile(self):
        self.assertEqual(percentile([4, 1, 3, 2], .50), 2)
        self.assertEqual(percentile([4, 1, 3, 2], .99), 4)
        self.assertIsNone(percentile([], .99))

    @unittest.skipUnless(platform.system() in ("Windows", "Linux"), "RSS-Plattform")
    def test_rss_samples_target_process(self):
        read, close = rss_reader(os.getpid())
        try:
            self.assertGreater(read(), 0)
        finally:
            close()

    def test_invalid_origin_is_rejected(self):
        with self.assertRaises(ValueError):
            Client("http://localhost/api", 2)


if __name__ == "__main__":
    unittest.main()
