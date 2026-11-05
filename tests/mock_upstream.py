#!/usr/bin/env python3
"""Mock upstream AI provider used by the integration tests.

The server exposes both OpenAI- and Anthropic-shaped endpoints plus a small control
API so tests can switch behaviours (status codes, delays, streaming) and inspect what
the proxy actually sent upstream.

Control API (only on the same process):
    GET  /__state          -> {"config": {...}, "requests": [...]}
    POST /__config         -> merge the given JSON object into the config
    POST /__reset          -> clear recorded requests
"""

from __future__ import annotations

import json
import sys
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

DEFAULT_CONFIG = {
    # Auth
    "require_key": True,
    "valid_keys": ["sk-upstream-1"],
    # Behaviour
    "models_status": 200,
    "models_body": "",
    "chat_status": 200,
    "chat_delay": 0.0,
    "stream_chunks": 3,
    "stream_delay": 0.01,
    "stream_status": 200,
    "stream_stall_after": 0,  # >0 => sleep this long after the first chunk
    "usage": {"prompt_tokens": 11, "completion_tokens": 7, "total_tokens": 18},
    "anthropic_usage": {
        "input_tokens": 10,
        "output_tokens": 5,
        "cache_read_input_tokens": 2,
    },
    "include_usage_in_stream": True,
    "error_body": "upstream failure",
}

STATE = {
    "config": json.loads(json.dumps(DEFAULT_CONFIG)),
    "requests": [],
}
LOCK = threading.Lock()


def get_config() -> dict:
    with LOCK:
        return json.loads(json.dumps(STATE["config"]))


def merge_config(patch: dict) -> dict:
    with LOCK:
        STATE["config"].update(patch)
        return json.loads(json.dumps(STATE["config"]))


def record(entry: dict) -> None:
    with LOCK:
        STATE["requests"].append(entry)


def snapshot_requests() -> list:
    with LOCK:
        return json.loads(json.dumps(STATE["requests"]))


def reset_requests() -> None:
    with LOCK:
        STATE["requests"] = []


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *args):  # silence request logging
        return

    # -- helpers ---------------------------------------------------------
    def _send_json(self, status: int, payload: dict) -> None:
        body = json.dumps(payload).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _read_body(self) -> bytes:
        length = int(self.headers.get("Content-Length") or 0)
        return self.rfile.read(length) if length else b""

    def _auth_ok(self, cfg: dict) -> bool:
        if not cfg["require_key"]:
            return True
        auth = self.headers.get("Authorization") or ""
        bearer = auth[7:].strip() if auth.lower().startswith("bearer ") else ""
        api_key = (self.headers.get("x-api-key") or "").strip()
        return bearer in cfg["valid_keys"] or api_key in cfg["valid_keys"]

    def _record(self, body: bytes, path: str) -> dict:
        entry = {
            "method": self.command,
            "path": path,
            "headers": {k.lower(): v for k, v in self.headers.items()},
            "body": body.decode("utf-8", errors="replace"),
            "time": time.time(),
        }
        record(entry)
        return entry

    # -- HTTP verbs ------------------------------------------------------
    def do_GET(self):
        cfg = get_config()

        if self.path == "/__state":
            self._send_json(200, {"config": cfg, "requests": snapshot_requests()})
            return

        self._record(b"", self.path)

        if not self._auth_ok(cfg):
            self._send_json(401, {"error": {"message": "invalid api key"}})
            return

        if self.path.rstrip("/").endswith("/models"):
            status = cfg["models_status"]
            if status == 200:
                self._send_json(200, {"object": "list", "data": [{"id": "mock-model"}]})
            elif cfg.get("models_body"):
                body = str(cfg["models_body"]).encode("utf-8")
                self.send_response(status)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)
            else:
                self._send_json(status, {"error": {"message": "models unavailable"}})
            return

        self._send_json(404, {"error": {"message": "not found"}})

    def do_POST(self):
        cfg = get_config()
        body = self._read_body()

        if self.path == "/__config":
            try:
                patch = json.loads(body or b"{}")
            except json.JSONDecodeError:
                self._send_json(400, {"error": "invalid json"})
                return
            self._send_json(200, merge_config(patch))
            return

        if self.path == "/__reset":
            reset_requests()
            self._send_json(200, {"ok": True})
            return

        entry = self._record(body, self.path)

        if not self._auth_ok(cfg):
            self._send_json(401, {"error": {"message": "invalid api key"}})
            return

        try:
            payload = json.loads(body or b"{}")
        except json.JSONDecodeError:
            self._send_json(400, {"error": {"message": "invalid json body"}})
            return

        if self.path.rstrip("/").endswith("/messages"):
            self._handle_anthropic(payload, cfg, entry)
            return

        if self.path.rstrip("/").endswith("/completions"):
            self._handle_openai(payload, cfg, entry)
            return

        self._send_json(404, {"error": {"message": "not found"}})

    # -- endpoints -------------------------------------------------------
    def _handle_openai(self, payload: dict, cfg: dict, entry: dict) -> None:
        if cfg["chat_delay"]:
            time.sleep(cfg["chat_delay"])

        status = cfg["chat_status"]
        if status != 200:
            payload_out = json.dumps({"error": {"message": cfg["error_body"]}}).encode()
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(payload_out)))
            self.end_headers()
            self.wfile.write(payload_out)
            return

        if payload.get("stream"):
            self._stream_openai(payload, cfg, entry)
            return

        response = {
            "id": "chatcmpl-mock",
            "object": "chat.completion",
            "model": payload.get("model"),
            "choices": [
                {
                    "index": 0,
                    "message": {"role": "assistant", "content": "pong"},
                    "finish_reason": "stop",
                }
            ],
            "usage": cfg["usage"],
        }
        self._send_json(200, response)

    def _stream_openai(self, payload: dict, cfg: dict, entry: dict) -> None:
        if cfg["stream_status"] != 200:
            body = json.dumps({"error": {"message": cfg["error_body"]}}).encode()
            self.send_response(cfg["stream_status"])
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            return

        self._begin_stream()

        chunks = max(1, int(cfg["stream_chunks"]))
        for index in range(chunks):
            piece = {
                "id": "chatcmpl-mock",
                "object": "chat.completion.chunk",
                "model": payload.get("model"),
                "choices": [
                    {
                        "index": 0,
                        "delta": {"content": f"chunk{index}"},
                        "finish_reason": None,
                    }
                ],
            }
            self._write_sse(json.dumps(piece))
            if index == 0 and cfg["stream_stall_after"]:
                self.wfile.flush()
                time.sleep(cfg["stream_stall_after"])
            else:
                time.sleep(cfg["stream_delay"])

        # Final usage chunk, as requested through stream_options.include_usage.
        wants_usage = (
            payload.get("stream_options", {}).get("include_usage") is True
            and cfg["include_usage_in_stream"]
        )
        if wants_usage:
            self._write_sse(
                json.dumps(
                    {
                        "id": "chatcmpl-mock",
                        "object": "chat.completion.chunk",
                        "model": payload.get("model"),
                        "choices": [],
                        "usage": cfg["usage"],
                    }
                )
            )

        self._write_sse("[DONE]")
        self._close_stream()

    def _handle_anthropic(self, payload: dict, cfg: dict, entry: dict) -> None:
        if cfg["chat_delay"]:
            time.sleep(cfg["chat_delay"])

        status = cfg["chat_status"]
        if status != 200:
            body = json.dumps({"type": "error", "error": {"message": cfg["error_body"]}}).encode()
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            return

        if payload.get("stream"):
            self._stream_anthropic(payload, cfg, entry)
            return

        response = {
            "id": "msg_mock",
            "type": "message",
            "role": "assistant",
            "model": payload.get("model"),
            "content": [{"type": "text", "text": "pong"}],
            "usage": cfg["anthropic_usage"],
        }
        self._send_json(200, response)

    def _stream_anthropic(self, payload: dict, cfg: dict, entry: dict) -> None:
        self._begin_stream()

        start = {
            "type": "message_start",
            "message": {
                "id": "msg_mock",
                "type": "message",
                "role": "assistant",
                "model": payload.get("model"),
                "usage": {
                    "input_tokens": cfg["anthropic_usage"]["input_tokens"],
                    "cache_read_input_tokens": cfg["anthropic_usage"].get(
                        "cache_read_input_tokens", 0
                    ),
                    "cache_creation_input_tokens": cfg["anthropic_usage"].get(
                        "cache_creation_input_tokens", 0
                    ),
                },
            },
        }
        self._write_sse(json.dumps(start), event="message_start")

        for index in range(max(1, int(cfg["stream_chunks"]))):
            delta = {
                "type": "content_block_delta",
                "index": 0,
                "delta": {"type": "text_delta", "text": f"chunk{index}"},
            }
            self._write_sse(json.dumps(delta), event="content_block_delta")
            time.sleep(cfg["stream_delay"])

        stop = {
            "type": "message_delta",
            "delta": {"stop_reason": "end_turn"},
            "usage": {"output_tokens": cfg["anthropic_usage"]["output_tokens"]},
        }
        self._write_sse(json.dumps(stop), event="message_delta")
        self._write_sse(json.dumps({"type": "message_stop"}), event="message_stop")
        self._close_stream()

    def _begin_stream(self) -> None:
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Cache-Control", "no-cache")
        # Real providers stream with chunked transfer encoding; without framing a
        # HTTP/1.1 client would treat the body as empty.
        self.send_header("Transfer-Encoding", "chunked")
        self.end_headers()

    def _write_sse(self, data: str, event: str | None = None) -> None:
        payload = ""
        if event:
            payload += f"event: {event}\n"
        payload += f"data: {data}\n\n"
        raw = payload.encode("utf-8")
        self.wfile.write(f"{len(raw):X}\r\n".encode("ascii") + raw + b"\r\n")
        self.wfile.flush()

    def _close_stream(self) -> None:
        try:
            self.wfile.write(b"0\r\n\r\n")
            self.wfile.flush()
            self.close_connection = True
        except Exception:
            pass


def main() -> int:
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 0
    server = ThreadingHTTPServer(("127.0.0.1", port), Handler)
    server.daemon_threads = True
    print(f"MOCK_PORT={server.server_address[1]}", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
