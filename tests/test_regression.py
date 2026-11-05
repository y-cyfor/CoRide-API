"""End-to-end regression scenarios.

These cover cross-cutting flows (full request lifecycle, persistence across restarts,
encryption key rotation, concurrency) rather than a single review finding.
"""

from __future__ import annotations

import concurrent.futures
import json
import time

from support import Harness

OPENAI_PATH = "/v1/chat/completions"


def setup_basic(harness: Harness, model: str = "mock-model", source: str = "mock-source"):
    backend = harness.backend
    channel_id = backend.create_channel()
    backend.create_model(channel_id, source, model)
    user_id, api_key = backend.create_user()
    return channel_id, user_id, api_key


def call(backend, api_key: str, model: str = "mock-model", stream: bool = False):
    return backend.api(
        "POST",
        OPENAI_PATH,
        json_body={"model": model, "messages": [{"role": "user", "content": "hi"}], "stream": stream},
        api_key=api_key,
    )


def wait_until(predicate, timeout: float = 10.0, interval: float = 0.1):
    deadline = time.time() + timeout
    last = None
    while time.time() < deadline:
        last = predicate()
        if last:
            return last
        time.sleep(interval)
    raise AssertionError(f"condition not met within {timeout}s (last={last})")


# ---------------------------------------------------------------------------
# Full lifecycle
# ---------------------------------------------------------------------------

def test_full_lifecycle_non_stream_and_stream(harness: Harness):
    backend = harness.backend
    _, user_id, api_key = setup_basic(harness)
    backend.create_quota(user_id, quota_type="tokens", total_limit=10_000, cycle="daily")

    status, body = call(backend, api_key)
    assert status == 200 and body["choices"][0]["message"]["content"] == "pong", body

    with backend.open_stream(
        OPENAI_PATH, {"model": "mock-model", "messages": [], "stream": True}, api_key
    ) as handle:
        assert handle.status == 200
        text = handle.read_all()
    assert "chunk0" in text and "[DONE]" in text, text[:200]

    wait_until(
        lambda: backend.admin_ok("GET", "/admin/logs?page=1&page_size=50")["total"] == 2
    )

    dashboard = backend.admin_ok("GET", "/admin/stats/dashboard")
    assert dashboard["total_requests"] == 2, dashboard
    assert dashboard["total_tokens"] == 36, dashboard
    assert dashboard["success_count"] == 2, dashboard
    # Streamed requests must record real latency (they used to log elapsed_ms = 0).
    assert dashboard["p95_latency_ms"] > 0, dashboard

    usage = backend.admin_ok("GET", "/admin/stats/usage?days=7")
    assert usage["token_daily"], usage
    assert usage["model_usage"][0]["name"] == "mock-model", usage["model_usage"]
    assert usage["top_users"][0]["username"] == "alice", usage["top_users"]

    quota_used = backend.sql("SELECT used FROM quotas WHERE user_id = ?", (user_id,))[0][0]
    assert quota_used == 36, quota_used


def test_parallel_requests_are_accounted_once(harness: Harness):
    backend = harness.backend
    _, user_id, api_key = setup_basic(harness)
    backend.create_quota(user_id, quota_type="tokens", total_limit=100_000, cycle="permanent")

    with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
        results = list(pool.map(lambda _: call(backend, api_key), range(16)))

    assert all(status == 200 for status, _ in results), [r[0] for r in results]

    wait_until(
        lambda: backend.admin_ok("GET", "/admin/logs?page=1&page_size=100")["total"] == 16
    )
    used = backend.sql("SELECT used FROM quotas WHERE user_id = ?", (user_id,))[0][0]
    assert used == 16 * 18, f"quota must reflect every request exactly once: {used}"


# ---------------------------------------------------------------------------
# Persistence and encryption
# ---------------------------------------------------------------------------

def test_data_survives_restart(harness: Harness):
    backend = harness.backend
    channel_id, _, api_key = setup_basic(harness)

    backend.restart()

    channels = backend.admin_ok("GET", "/admin/channels?page=1&page_size=50")["items"]
    assert any(item["id"] == channel_id for item in channels), channels

    # The encrypted keys must still decrypt after a restart.
    status, body = call(backend, api_key)
    assert status == 200, (status, body)


def test_encryption_key_rotation_keeps_channels_working(harness: Harness):
    backend = harness.backend
    channel_id = backend.create_channel()
    backend.create_model(channel_id, "src", "m")
    _, api_key = backend.create_user()
    assert call(backend, api_key, model="m")[0] == 200

    # Rotate: the new secret encrypts future writes, the old one stays in the ring.
    backend.restart(
        {"CORIDE_ENCRYPTION_SECRETS": "rotated-secret,integration-test-secret"}
    )
    assert call(backend, api_key, model="m")[0] == 200

    # Re-saving the channel re-encrypts with the new primary secret.
    channel = next(
        item
        for item in backend.admin_ok("GET", "/admin/channels?page=1&page_size=50")["items"]
        if item["id"] == channel_id
    )
    backend.admin_ok(
        "PUT",
        f"/admin/channels/{channel_id}",
        {"api_keys": channel["api_keys"], "weight": channel["weight"]},
    )

    row = backend.sql("SELECT api_keys FROM channels WHERE id = ?", (channel_id,))[0][0]
    assert row.startswith("enc:v1:"), row[:20]

    assert call(backend, api_key, model="m")[0] == 200


def test_wrong_encryption_secret_fails_loudly(harness: Harness):
    backend = harness.backend
    channel_id = backend.create_channel()
    backend.create_model(channel_id, "src", "m")
    _, api_key = backend.create_user()
    assert call(backend, api_key, model="m")[0] == 200

    backend.restart({"CORIDE_ENCRYPTION_SECRETS": "completely-different-secret"})

    status, body = call(backend, api_key, model="m")
    assert status >= 500, f"undecryptable channels must fail instead of sending ciphertext: {status} {body}"
    assert "decrypt" in json.dumps(body).lower(), body

    # Editing the channel must not silently re-encrypt the ciphertext.
    status, body = backend.admin("PUT", f"/admin/channels/{channel_id}", {"weight": 2})
    assert status == 400, f"editing must refuse to corrupt keys: {status} {body}"

    # Providing the plaintext keys recovers the channel.
    status, _ = backend.admin(
        "PUT",
        f"/admin/channels/{channel_id}",
        {"api_keys": '["sk-upstream-1"]', "weight": 2},
    )
    assert status == 200
    assert call(backend, api_key, model="m")[0] == 200


# ---------------------------------------------------------------------------
# Request validation
# ---------------------------------------------------------------------------

def test_invalid_requests_are_rejected(harness: Harness):
    backend = harness.backend
    _, _, api_key = setup_basic(harness)

    status, body = backend.api(
        "POST", OPENAI_PATH, json_body={"messages": []}, api_key=api_key
    )
    assert status == 400, (status, body)
    assert "model" in json.dumps(body).lower()

    # Invalid JSON body.
    status, body = backend.api(
        "POST",
        OPENAI_PATH,
        json_body=None,
        api_key=api_key,
        headers={"Content-Type": "application/json"},
    )
    assert status == 400, (status, body)

    # Body larger than the 10 MiB limit.
    big = {"model": "mock-model", "messages": [{"role": "user", "content": "x" * (11 * 1024 * 1024)}]}
    status, body = backend.api("POST", OPENAI_PATH, json_body=big, api_key=api_key, timeout=120)
    assert status == 413, f"oversized body must be rejected: {status} {body}"


def test_disabled_model_and_channel_are_not_routed(harness: Harness):
    backend = harness.backend
    channel_id, _, api_key = setup_basic(harness)

    models = backend.admin_ok("GET", "/admin/models?page=1&page_size=50")["items"]
    model_id = models[0]["id"]

    backend.admin_ok("PUT", f"/admin/models/{model_id}", {"enabled": False})
    status, _ = call(backend, api_key)
    assert status == 404, "disabled models must not be routed"

    backend.admin_ok("PUT", f"/admin/models/{model_id}", {"enabled": True})
    backend.admin_ok("PUT", f"/admin/channels/{channel_id}", {"status": "disabled"})
    status, _ = call(backend, api_key)
    assert status == 404, "disabled channels must not be routed"

    backend.admin_ok("PUT", f"/admin/channels/{channel_id}", {"status": "active"})
    assert call(backend, api_key)[0] == 200


# ---------------------------------------------------------------------------
# Traffic plans / app profiles
# ---------------------------------------------------------------------------

def test_traffic_plan_forwards_app_profile_headers(harness: Harness):
    backend, mock = harness.backend, harness.mock
    channel_id, _, api_key = setup_basic(harness)

    profile_id = backend.admin_ok(
        "POST",
        "/admin/app-profiles",
        {
            "name": "Test Client",
            "identifier": "test-client",
            "user_agent": "test-client/9.9",
            "extra_headers": '{"x-test-header":"yes"}',
        },
    )["id"]

    backend.admin_ok(
        "PUT",
        f"/admin/traffic-plan/channel/{channel_id}",
        {"slots": [{"start_hour": 0, "end_hour": 24, "app_profile_id": profile_id, "weight": 1}]},
    )

    before = len(mock.requests())
    assert call(backend, api_key)[0] == 200

    upstream = [r for r in mock.requests()[before:] if r["method"] == "POST"]
    assert upstream, "no upstream request recorded"
    headers = upstream[0]["headers"]
    assert headers.get("user-agent") == "test-client/9.9", headers
    assert headers.get("x-test-header") == "yes", headers


def test_traffic_plan_validation(harness: Harness):
    backend = harness.backend
    status, body = backend.admin(
        "PUT",
        "/admin/traffic-plan/global",
        {"slots": [{"start_hour": 0, "end_hour": 24, "app_profile_id": 9999, "weight": 1}]},
    )
    assert status == 400, f"unknown app profile must be rejected: {status} {body}"

    status, body = backend.admin(
        "PUT",
        "/admin/traffic-plan/global",
        {
            "slots": [
                {"start_hour": 0, "end_hour": 12, "app_profile_id": 1, "weight": 1},
                {"start_hour": 6, "end_hour": 18, "app_profile_id": 1, "weight": 1},
            ]
        },
    )
    assert status == 400, f"overlapping slots must be rejected: {status} {body}"


# ---------------------------------------------------------------------------
# Rate limiting behaviour
# ---------------------------------------------------------------------------

def test_login_rate_limit_blocks_brute_force(harness: Harness):
    backend = harness.backend
    statuses = []
    for _ in range(7):
        status, _ = backend.api(
            "POST", "/admin/auth/login", json_body={"username": "admin", "password": "wrong"}
        )
        statuses.append(status)

    assert statuses[0] == 401, statuses
    assert 429 in statuses, f"login attempts must be throttled: {statuses}"


def test_queue_action_waits_instead_of_rejecting(harness: Harness):
    backend = harness.backend
    _, user_id, api_key = setup_basic(harness)
    backend.admin_ok(
        "POST",
        "/admin/ratelimits",
        {
            "target_type": "user",
            "target_id": user_id,
            "qps": 1,
            "concurrency": 5,
            "action": "queue",
        },
    )

    assert call(backend, api_key)[0] == 200
    started = time.time()
    status, body = call(backend, api_key)
    elapsed = time.time() - started

    assert status == 200, f"queue action must wait then succeed: {status} {body}"
    assert elapsed >= 0.3, f"the second call should have waited: {elapsed:.2f}s"


# ---------------------------------------------------------------------------
# Misc endpoints
# ---------------------------------------------------------------------------

def test_health_endpoint_reports_database(harness: Harness):
    backend = harness.backend
    status, body = backend.api("GET", "/health")
    assert status == 200
    assert body["status"] == "ok" and body["database"] == "connected", body


def test_sqlite_runs_in_wal_mode(harness: Harness):
    backend = harness.backend
    mode = backend.sql("PRAGMA journal_mode")[0][0]
    assert str(mode).lower() == "wal", f"concurrent writes require WAL, got {mode}"


def test_models_endpoint_shape(harness: Harness):
    backend = harness.backend
    _, _, api_key = setup_basic(harness)
    status, body = backend.api("GET", "/v1/models", api_key=api_key)
    assert status == 200
    assert body["object"] == "list", body
    assert body["data"][0]["id"] == "mock-model", body
    assert body["data"][0]["object"] == "model", body


def test_user_info_endpoint(harness: Harness):
    backend = harness.backend
    _, user_id, api_key = setup_basic(harness)
    backend.create_quota(user_id, quota_type="requests", total_limit=10, cycle="permanent")

    status, body = backend.api("GET", "/v1/user/info", api_key=api_key)
    assert status == 200, (status, body)
    data = body["data"]
    assert data["username"] == "alice", data
    assert data["quotas"][0]["remaining"] == 10, data["quotas"]
    assert data["available_models"] == ["mock-model"], data["available_models"]
    assert data["api_key"].endswith("****"), data


def test_ip_whitelist_accepts_cidr(harness: Harness):
    backend = harness.backend
    _, user_id, api_key = setup_basic(harness)
    backend.admin_ok(
        "POST", "/admin/ip/whitelist", {"user_id": user_id, "ip_address": "127.0.0.0/8"}
    )
    status, body = call(backend, api_key)
    assert status == 200, f"CIDR whitelist must allow the local address: {status} {body}"
