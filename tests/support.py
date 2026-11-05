"""Test harness for the CoRide-API integration tests.

Starts a real backend binary against a throw-away SQLite database and a mock upstream
provider, so tests exercise the same code path as production (auth, routing, quota,
streaming, logging).
"""

from __future__ import annotations

import json
import os
import pathlib
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import http.client
from typing import Any, Iterator

ROOT = pathlib.Path(__file__).resolve().parents[1]
BIN = ROOT / "backend" / "target" / "debug" / ("coride-api.exe" if os.name == "nt" else "coride-api")
MOCK = ROOT / "tests" / "mock_upstream.py"
# Work inside the repository: the sandboxed test environment allows writes here
# (and `backend/target` is already git-ignored).
TEST_RUN_ROOT = ROOT / "backend" / "target" / "itest"

ADMIN_USER = "admin"
ADMIN_PASSWORD = "admin123"


def free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def wait_for_port(port: int, timeout: float = 30.0) -> None:
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            with socket.create_connection(("127.0.0.1", port), timeout=0.5):
                return
        except OSError:
            time.sleep(0.05)
    raise RuntimeError(f"port {port} did not open within {timeout}s")


class MockUpstream:
    def __init__(self) -> None:
        self.port = free_port()
        self.proc: subprocess.Popen | None = None

    def start(self) -> "MockUpstream":
        self.proc = subprocess.Popen(
            [sys.executable, str(MOCK), str(self.port)],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            env={**os.environ, "PYTHONUNBUFFERED": "1"},
        )
        wait_for_port(self.port)
        return self

    def stop(self) -> None:
        if self.proc is None:
            return
        self.proc.terminate()
        try:
            self.proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.proc.kill()
        self.proc = None

    @property
    def base_url(self) -> str:
        return f"http://127.0.0.1:{self.port}"

    def configure(self, **patch: Any) -> dict:
        status, body = _http(
            "POST",
            f"http://127.0.0.1:{self.port}/__config",
            json_body=patch,
        )
        assert status == 200, f"mock config failed: {status} {body}"
        return body

    def state(self) -> dict:
        status, body = _http("GET", f"http://127.0.0.1:{self.port}/__state")
        assert status == 200, f"mock state failed: {status} {body}"
        return body

    def requests(self) -> list:
        return self.state()["requests"]

    def reset(self) -> None:
        _http("POST", f"http://127.0.0.1:{self.port}/__reset")
        self.configure(**{
            "valid_keys": ["sk-upstream-1"],
            "require_key": True,
            "models_status": 200,
            "chat_status": 200,
            "chat_delay": 0.0,
            "stream_chunks": 3,
            "stream_delay": 0.01,
            "stream_status": 200,
            "stream_stall_after": 0,
            "usage": {"prompt_tokens": 11, "completion_tokens": 7, "total_tokens": 18},
            "anthropic_usage": {
                "input_tokens": 10,
                "output_tokens": 5,
                "cache_read_input_tokens": 2,
            },
            "include_usage_in_stream": True,
            "error_body": "upstream failure",
        })


def _http(
    method: str,
    url: str,
    json_body: Any = None,
    headers: dict | None = None,
    timeout: float = 60.0,
):
    from urllib.parse import urlparse

    parsed = urlparse(url)
    path = parsed.path or "/"
    if parsed.query:
        path += "?" + parsed.query

    connection_cls = (
        http.client.HTTPSConnection if parsed.scheme == "https" else http.client.HTTPConnection
    )
    conn = connection_cls(parsed.hostname, parsed.port, timeout=timeout)
    body = None
    request_headers = dict(headers or {})
    if json_body is not None:
        body = json.dumps(json_body).encode()
        request_headers.setdefault("Content-Type", "application/json")

    try:
        conn.request(method, path, body=body, headers=request_headers)
        response = conn.getresponse()
        raw = response.read()
        status = response.status
        response_headers = {k.lower(): v for k, v in response.getheaders()}
    finally:
        conn.close()

    text = raw.decode("utf-8", errors="replace")
    try:
        return status, json.loads(text)
    except json.JSONDecodeError:
        return status, text


class Backend:
    def __init__(
        self,
        mock: MockUpstream,
        health_interval_secs: int | None = None,
        extra_env: dict | None = None,
    ) -> None:
        self.mock = mock
        self.port = free_port()
        TEST_RUN_ROOT.mkdir(parents=True, exist_ok=True)
        self.workdir = pathlib.Path(tempfile.mkdtemp(prefix="coride-", dir=TEST_RUN_ROOT))
        self.proc: subprocess.Popen | None = None
        self.log_path = self.workdir / "backend.log"
        self._log_handle = None
        self.admin_token: str | None = None
        self.health_interval_secs = health_interval_secs
        self.extra_env = dict(extra_env or {})

    # -- lifecycle -------------------------------------------------------
    def start(self) -> "Backend":
        if not BIN.exists():
            raise RuntimeError(f"backend binary not found: {BIN} (run `cargo build` first)")

        (self.workdir / "config").mkdir(parents=True, exist_ok=True)
        (self.workdir / "data").mkdir(parents=True, exist_ok=True)
        self._write_config()
        self._spawn()
        return self

    def _spawn(self) -> None:
        if self._log_handle is not None:
            self._log_handle.close()
        self._log_handle = open(self.log_path, "a", encoding="utf-8")
        env = {
            **os.environ,
            "CORIDE_PORT": str(self.port),
            "CORIDE_DB_PATH": str(self.workdir / "data" / "test.db"),
            "CORIDE_LOG_LEVEL": "info",
            "CORIDE_JWT_SECRET": "integration-test-secret",
            # Must not override the config file: the health checks are enabled/disabled
            # through config.yaml in `_write_config`.
            "CORIDE_HEALTH_CHECK_ENABLED": (
                "false" if self.health_interval_secs is None else "true"
            ),
            "RUST_BACKTRACE": "1",
            **self.extra_env,
        }
        self.proc = subprocess.Popen(
            [str(BIN)],
            cwd=str(self.workdir),
            stdout=self._log_handle,
            stderr=subprocess.STDOUT,
            env=env,
        )
        wait_for_port(self.port)
        self._wait_healthy()

    def restart(self, extra_env: dict | None = None) -> "Backend":
        """Restart the backend on the same database, optionally with extra env vars."""
        self.stop()
        if extra_env:
            self.extra_env.update(extra_env)
        self.admin_token = None
        self._spawn()
        return self

    def _write_config(self) -> None:
        if self.health_interval_secs is None:
            health_block = """
health_check:
  enabled: false
  interval_secs: 60
  failure_threshold: 3
  timeout_secs: 5
"""
        else:
            health_block = f"""
health_check:
  enabled: true
  interval_secs: {self.health_interval_secs}
  failure_threshold: 2
  timeout_secs: 5
"""

        config = f"""
server:
  port: {self.port}
  host: "127.0.0.1"
  cors_allowed_origins: null
database:
  path: "./data/test.db"
  pool_size: 5
admin:
  username: "{ADMIN_USER}"
  password: "{ADMIN_PASSWORD}"
jwt:
  secret: "integration-test-secret"
  expires_in: 3600
log:
  level: "info"
  retention_days: 30
  max_records: 100000
proxy:
  timeout: 30000
  max_retries: 1
  log_request_body: false
  log_response_body: false
  trusted_proxies:
    - "127.0.0.1"
global_rate_limit:
  qps: 1000
  concurrency: 100
  action: "reject"
channel_queue:
  max_concurrent: 4
  max_retries: 2
  delay_min_ms: 10
  delay_max_ms: 50
{health_block}"""
        (self.workdir / "config" / "config.yaml").write_text(config, encoding="utf-8")

    def _wait_healthy(self, timeout: float = 45.0) -> None:
        deadline = time.time() + timeout
        while time.time() < deadline:
            if self.proc and self.proc.poll() is not None:
                raise RuntimeError(f"backend exited early:\n{self.read_log()}")
            try:
                status, body = self.api("GET", "/health")
                if status == 200 and isinstance(body, dict) and body.get("status") == "ok":
                    return
            except OSError:
                pass
            time.sleep(0.1)
        raise RuntimeError(f"backend did not become healthy:\n{self.read_log()}")

    def stop(self) -> None:
        if self.proc is not None:
            self.proc.terminate()
            try:
                self.proc.wait(timeout=10)
            except subprocess.TimeoutExpired:
                self.proc.kill()
                self.proc.wait(timeout=5)
            self.proc = None
        if self._log_handle is not None:
            self._log_handle.close()
            self._log_handle = None

    def cleanup(self) -> None:
        self.stop()
        shutil.rmtree(self.workdir, ignore_errors=True)

    def read_log(self) -> str:
        try:
            return self.log_path.read_text(encoding="utf-8", errors="replace")
        except FileNotFoundError:
            return ""

    def sql(self, query: str, params: tuple = ()) -> list:
        """Run a statement directly against the test database (WAL allows this)."""
        import sqlite3

        connection = sqlite3.connect(self.workdir / "data" / "test.db", timeout=10)
        try:
            cursor = connection.execute(query, params)
            connection.commit()
            return cursor.fetchall()
        finally:
            connection.close()

    # -- HTTP helpers ----------------------------------------------------
    @property
    def base_url(self) -> str:
        return f"http://127.0.0.1:{self.port}"

    def api(
        self,
        method: str,
        path: str,
        json_body: Any = None,
        token: str | None = None,
        api_key: str | None = None,
        headers: dict | None = None,
        timeout: float = 60.0,
    ):
        request_headers = dict(headers or {})
        if token:
            request_headers["Authorization"] = f"Bearer {token}"
        if api_key:
            request_headers["Authorization"] = f"Bearer {api_key}"
        return _http(
            method,
            f"{self.base_url}{path}",
            json_body=json_body,
            headers=request_headers,
            timeout=timeout,
        )

    def login(self) -> str:
        status, body = self.api(
            "POST",
            "/admin/auth/login",
            json_body={"username": ADMIN_USER, "password": ADMIN_PASSWORD},
        )
        assert status == 200, f"login failed: {status} {body}"
        self.admin_token = body["data"]["token"]
        return self.admin_token

    def admin(self, method: str, path: str, json_body: Any = None, timeout: float = 60.0):
        if not self.admin_token:
            self.login()
        return self.api(method, path, json_body=json_body, token=self.admin_token, timeout=timeout)

    def admin_ok(self, method: str, path: str, json_body: Any = None) -> Any:
        status, body = self.admin(method, path, json_body=json_body)
        assert status == 200 and body.get("code") == 0, f"{method} {path} failed: {status} {body}"
        return body.get("data")

    # -- domain helpers --------------------------------------------------
    def create_channel(
        self,
        name: str = "mock",
        channel_type: str = "openai",
        base_url: str | None = None,
        api_keys: str = '["sk-upstream-1"]',
        **extra: Any,
    ) -> int:
        if base_url is None:
            base_url = f"{self.mock.base_url}/v1" if channel_type == "openai" else self.mock.base_url
        payload = {
            "name": name,
            "type": channel_type,
            "base_url": base_url,
            "api_keys": api_keys,
            **extra,
        }
        data = self.admin_ok("POST", "/admin/channels", payload)
        return data["id"]

    def create_model(self, channel_id: int, source_name: str, proxy_name: str) -> int:
        data = self.admin_ok(
            "POST",
            "/admin/models",
            {"channel_id": channel_id, "source_name": source_name, "proxy_name": proxy_name},
        )
        return data["id"]

    def create_user(
        self,
        username: str = "alice",
        password: str = "secret123",
        enabled_models: str | None = None,
        role: str = "user",
    ) -> tuple[int, str]:
        payload: dict[str, Any] = {"username": username, "password": password, "role": role}
        if enabled_models is not None:
            payload["enabled_models"] = enabled_models
        data = self.admin_ok("POST", "/admin/users", payload)
        return data["id"], data["api_key"]

    def create_quota(
        self,
        user_id: int,
        quota_type: str = "requests",
        total_limit: int = 100,
        cycle: str = "permanent",
        channel_id: int | None = None,
    ) -> int:
        data = self.admin_ok(
            "POST",
            "/admin/quotas",
            {
                "user_id": user_id,
                "quota_type": quota_type,
                "total_limit": total_limit,
                "cycle": cycle,
                "channel_id": channel_id,
            },
        )
        return data["id"]

    def user_token(self, username: str, password: str = "secret123") -> str:
        status, body = self.api(
            "POST", "/admin/auth/login", json_body={"username": username, "password": password}
        )
        assert status == 200, f"user login failed: {status} {body}"
        return body["data"]["token"]

    def open_stream(
        self,
        path: str,
        body: dict,
        api_key: str,
        timeout: float = 60.0,
    ) -> "StreamHandle":
        """Open a streaming request, exposing the status and an iterator of chunks."""
        conn = http.client.HTTPConnection("127.0.0.1", self.port, timeout=timeout)
        payload = json.dumps(body).encode()
        conn.request(
            "POST",
            path,
            body=payload,
            headers={
                "Content-Type": "application/json",
                "Authorization": f"Bearer {api_key}",
                "Accept": "text/event-stream",
            },
        )
        response = conn.getresponse()
        return StreamHandle(conn, response)

    def stream(
        self,
        path: str,
        body: dict,
        api_key: str,
        timeout: float = 60.0,
    ) -> Iterator[bytes]:
        """Open a streaming request and yield raw chunks."""
        with self.open_stream(path, body, api_key, timeout) as handle:
            for chunk in handle:
                yield chunk


class StreamHandle:
    def __init__(self, connection, response) -> None:
        self.connection = connection
        self.response = response
        self.status = response.status
        self.reason = response.reason

    def __iter__(self) -> Iterator[bytes]:
        # `read1` returns as soon as any bytes are available; `read` would block until
        # the requested size or EOF, which never happens for a stalled stream.
        while True:
            try:
                chunk = self.response.read1(4096)
            except Exception:
                break
            if not chunk:
                break
            yield chunk

    def read_all(self) -> str:
        return b"".join(self).decode("utf-8", errors="replace")

    def close(self) -> None:
        try:
            self.connection.close()
        except Exception:
            pass

    def __enter__(self) -> "StreamHandle":
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        self.close()
        return None


class Harness:
    """Mock upstream + backend pair with per-test isolation."""

    def __init__(self, health_interval_secs: int | None = None) -> None:
        self.mock = MockUpstream()
        self.health_interval_secs = health_interval_secs
        self.backend: Backend | None = None

    def __enter__(self) -> "Harness":
        self.mock.start()
        self.backend = Backend(self.mock, health_interval_secs=self.health_interval_secs).start()
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        if self.backend is not None:
            if exc_type is not None:
                # Keep the log on failure to make debugging easier.
                print("\n--- backend log (tail) ---")
                print("\n".join(self.backend.read_log().splitlines()[-40:]))
            self.backend.cleanup()
        self.mock.stop()
        return None


def custom_harness(func):
    """Mark a test as managing its own Harness (e.g. custom health-check interval)."""
    func.custom_harness = True
    return func
