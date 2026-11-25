#!/usr/bin/env python3
import http.client
import sys
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import urlsplit

if len(sys.argv) != 3:
    raise SystemExit("usage: native-ios-smoke-proxy.py <driver-url> <request-log>")

driver_url, request_log = sys.argv[1:]
target = urlsplit(driver_url)
if target.scheme != "http" or not target.hostname or not target.port:
    raise SystemExit("smoke proxy expects an http Driver URL with explicit port")

log_lock = threading.Lock()

class ProxyHandler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def do_GET(self):
        self._relay()

    def do_POST(self):
        self._relay()

    def do_PUT(self):
        self._relay()

    def do_DELETE(self):
        self._relay()

    def log_message(self, format, *args):
        return

    def _relay(self):
        length = int(self.headers.get("content-length", "0") or "0")
        body = self.rfile.read(length) if length else None

        forwarded = {}
        for name, value in self.headers.items():
            lower = name.lower()
            if lower in {"host", "connection", "content-length", "accept-encoding"}:
                continue
            forwarded[name] = value

        connection = http.client.HTTPConnection(target.hostname, target.port, timeout=30)
        try:
            connection.request(
                self.command,
                self.path,
                body=body,
                headers=forwarded,
            )
            response = connection.getresponse()
            response_body = response.read()

            with log_lock, open(request_log, "a", encoding="utf-8") as handle:
                handle.write(f"{self.command} {self.path} {response.status}\n")
                handle.flush()

            self.send_response(response.status)
            for name, value in response.getheaders():
                lower = name.lower()
                if lower in {"connection", "transfer-encoding", "content-length"}:
                    continue
                self.send_header(name, value)
            self.send_header("content-length", str(len(response_body)))
            self.end_headers()
            if response_body:
                self.wfile.write(response_body)
        finally:
            connection.close()

server = ThreadingHTTPServer(("127.0.0.1", 0), ProxyHandler)
port = server.server_address[1]
print(
    f"CARGO_DISPATCH_SMOKE_PROXY_URL=http://127.0.0.1:{port}{target.path}",
    flush=True,
)
server.serve_forever()
