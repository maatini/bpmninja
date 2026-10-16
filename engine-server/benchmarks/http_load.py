"""Reproduzierbarer HTTP-/NATS-Lasttest; ausschließlich Python-Standardbibliothek."""

import argparse
import concurrent.futures
import http.client
import json
import math
import os
import platform
import threading
import time
import uuid
from pathlib import Path
from urllib.parse import urlsplit


class Client:
    def __init__(self, base_url, timeout):
        url = urlsplit(base_url)
        if (url.scheme not in ("http", "https") or not url.hostname
                or url.path not in ("", "/") or url.query or url.fragment or url.username):
            raise ValueError("base-url muss eine HTTP(S)-Origin ohne Pfad sein")
        cls = http.client.HTTPSConnection if url.scheme == "https" else http.client.HTTPConnection
        self.connection = cls(url.hostname, url.port, timeout=timeout)
        self.headers = {"Content-Type": "application/json"}
        if os.environ.get("BPMNINJA_API_KEY"):
            self.headers["X-API-Key"] = os.environ["BPMNINJA_API_KEY"]

    def request(self, method, path, body=None):
        self.connection.request(method, path, json.dumps(body) if body is not None else None, self.headers)
        response = self.connection.getresponse()
        data = response.read()
        if not 200 <= response.status < 300:
            raise RuntimeError(f"{method} {path}: HTTP {response.status}")
        if not data:
            return None
        return json.loads(data) if "application/json" in response.getheader("Content-Type", "") else data.decode()

    def close(self):
        self.connection.close()


def workflow(steps, process_id):
    nodes = ['<startEvent id="start"/>']
    ids = ["start"]
    for index in range(steps):
        node_id = f"script{index}"
        nodes.append(f'<scriptTask id="{node_id}" scriptFormat="rhai"><script>counter += 1;</script></scriptTask>')
        ids.append(node_id)
    nodes.append('<endEvent id="end"/>')
    ids.append("end")
    flows = [f'<sequenceFlow id="f{i}" sourceRef="{a}" targetRef="{b}"/>'
             for i, (a, b) in enumerate(zip(ids, ids[1:]))]
    return ('<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">'
            f'<process id="{process_id}" isExecutable="true">'
            + "".join(nodes + flows) + '</process></definitions>')


def percentile(values, fraction):
    ordered = sorted(values)
    return ordered[max(0, math.ceil(len(ordered) * fraction) - 1)] if ordered else None


def rss_reader(pid):
    """Aktueller RSS/Working Set des lokalen Serverprozesses, nicht des Clients."""
    if platform.system() == "Linux":
        def read():
            fields = Path(f"/proc/{pid}/statm").read_text().split()
            return int(fields[1]) * os.sysconf("SC_PAGE_SIZE")
        read()  # Ungültige PID früh erkennen.
        return read, lambda: None
    if platform.system() == "Windows":
        import ctypes
        from ctypes import wintypes

        class Counters(ctypes.Structure):
            _fields_ = [("cb", wintypes.DWORD), ("PageFaultCount", wintypes.DWORD)] + [
                (name, ctypes.c_size_t) for name in (
                    "PeakWorkingSetSize", "WorkingSetSize", "QuotaPeakPagedPoolUsage",
                    "QuotaPagedPoolUsage", "QuotaPeakNonPagedPoolUsage", "QuotaNonPagedPoolUsage",
                    "PagefileUsage", "PeakPagefileUsage")]

        kernel = ctypes.WinDLL("kernel32", use_last_error=True)
        psapi = ctypes.WinDLL("psapi", use_last_error=True)
        kernel.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
        kernel.OpenProcess.restype = wintypes.HANDLE
        kernel.CloseHandle.argtypes = [wintypes.HANDLE]
        psapi.GetProcessMemoryInfo.argtypes = [wintypes.HANDLE, ctypes.POINTER(Counters), wintypes.DWORD]
        handle = kernel.OpenProcess(0x0400 | 0x0010, False, pid)
        if not handle:
            raise ctypes.WinError(ctypes.get_last_error())

        def read():
            counters = Counters()
            counters.cb = ctypes.sizeof(counters)
            if not psapi.GetProcessMemoryInfo(handle, ctypes.byref(counters), counters.cb):
                raise ctypes.WinError(ctypes.get_last_error())
            return counters.WorkingSetSize
        try:
            read()
        except Exception:
            kernel.CloseHandle(handle)
            raise
        return read, lambda: kernel.CloseHandle(handle)
    raise ValueError("RSS-Sampling unterstützt Linux und Windows; sonst --server-pid weglassen")


def run_phase(args, definition_key, count):
    """Closed-loop: eine sequenzielle Start-/Lese-Transaktion pro Worker."""
    barrier = threading.Barrier(args.concurrency + 1)

    def worker(worker_index):
        client = Client(args.base_url, args.timeout)
        timings, errors = [], []
        barrier.wait()
        try:
            for _ in range(worker_index, count, args.concurrency):
                started = time.perf_counter()
                try:
                    instance = client.request("POST", "/api/start", {
                        "definition_key": definition_key,
                        "variables": {"counter": 0, "payload": "x" * args.payload_bytes},
                    })
                    state = client.request("GET", f'/api/instances/{instance["instance_id"]}')
                    if state["variables"]["counter"] != args.steps or state["state"] != "Completed":
                        raise RuntimeError("Instanz nicht erfolgreich abgeschlossen")
                    timings.append((time.perf_counter() - started) * 1000)
                except Exception as error:
                    errors.append(str(error))
                    client.close()  # Nach Transportfehler neue Verbindung beim nächsten Request.
        finally:
            client.close()
        return timings, errors

    with concurrent.futures.ThreadPoolExecutor(max_workers=args.concurrency) as pool:
        futures = [pool.submit(worker, i) for i in range(args.concurrency)]
        started = time.perf_counter()
        barrier.wait()
        results = [future.result() for future in futures]
        elapsed = time.perf_counter() - started
    timings = [value for result, _ in results for value in result]
    errors = [error for _, result in results for error in result]
    return {
        "attempted": count, "succeeded": len(timings), "failed": len(errors),
        "elapsed_seconds": elapsed, "transactions_per_second": len(timings) / elapsed,
        "latency_ms": {"p50": percentile(timings, .50), "p95": percentile(timings, .95),
                       "p99": percentile(timings, .99), "max": max(timings) if timings else None},
        "error_examples": sorted(set(errors))[:5],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-url", default="http://127.0.0.1:8081")
    parser.add_argument("--requests", type=int, default=1000)
    parser.add_argument("--warmup", type=int, default=100)
    parser.add_argument("--concurrency", type=int, default=8)
    parser.add_argument("--steps", type=int, default=10)
    parser.add_argument("--payload-bytes", type=int, default=1024)
    parser.add_argument("--timeout", type=float, default=30)
    parser.add_argument("--server-pid", type=int)
    parser.add_argument("--allow-in-memory", action="store_true")
    parser.add_argument("--label", required=True, help="Allocator, Profil und Revision des Servers")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if min(args.requests, args.concurrency, args.steps, args.timeout) <= 0 or min(args.warmup, args.payload_bytes) < 0:
        parser.error("requests/concurrency/steps/timeout müssen positiv, warmup/payload-bytes nicht negativ sein")
    if not math.isfinite(args.timeout) or (args.server_pid is not None and args.server_pid <= 0):
        parser.error("timeout muss endlich und server-pid positiv sein")
    client = Client(args.base_url, args.timeout)
    stop = threading.Event()
    rss, rss_errors = [], []
    sampler = None
    close_reader = lambda: None
    try:
        client.request("GET", "/api/ready")
        backend = client.request("GET", "/api/info")
        if not args.allow_in_memory and backend["backend_type"] != "persistent":
            raise RuntimeError("NATS-Persistenz erforderlich; für Smoke-Test --allow-in-memory verwenden")
        process_id = "http_bench_" + uuid.uuid4().hex
        definition = client.request("POST", "/api/deploy", {
            "xml": workflow(args.steps, process_id), "name": process_id})
        if args.warmup:
            warmup = run_phase(args, definition["definition_key"], args.warmup)
            if warmup["failed"]:
                raise RuntimeError(f'Warmup fehlgeschlagen: {warmup["error_examples"]}')
        persistence_before = client.request("GET", "/api/monitoring")["persistence_errors"]
        if args.server_pid:
            read_rss, close_reader = rss_reader(args.server_pid)
            rss.append(read_rss())

            def sample():
                while not stop.wait(.1):
                    try:
                        rss.append(read_rss())
                    except Exception as error:
                        rss_errors.append(str(error))
                        return
            sampler = threading.Thread(target=sample, daemon=True)
            sampler.start()
        result = run_phase(args, definition["definition_key"], args.requests)
        if args.server_pid:
            rss.append(read_rss())
        stop.set()
        if sampler:
            sampler.join()
        persistence_after = client.request("GET", "/api/monitoring")["persistence_errors"]
        client.request("GET", "/api/ready")
        result.update({
            "label": args.label, "platform": platform.platform(), "backend": backend["backend_type"],
            "workload": "script-chain-start-and-get", "concurrency": args.concurrency,
            "steps": args.steps, "payload_bytes": args.payload_bytes, "warmup": args.warmup,
            "rss_bytes": {"before": rss[0] if rss else None, "after": rss[-1] if rss else None,
                          "sampled_peak": max(rss) if rss else None, "samples": len(rss)},
            "rss_errors": rss_errors,
            "persistence_errors_delta": persistence_after - persistence_before,
        })
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
        print(json.dumps(result, indent=2))
        return 1 if result["failed"] or rss_errors or result["persistence_errors_delta"] else 0
    finally:
        stop.set()
        if sampler:
            sampler.join()
        close_reader()
        client.close()


if __name__ == "__main__":
    raise SystemExit(main())
