"""P0 regression tests: the blocking defects found in the 2026-11 code review.

Every test starts a real backend with a mock upstream; nothing is stubbed inside the
Rust code, so these assertions cover the same paths production traffic uses.
"""

from __future__ import annotations

import json
import threading
import time

from support import Harness, MockUpstream, custom_harness

OPENAI_PATH = "/v1/chat/completions"
ANTHROPIC_PATH = "/v1/messages"


# ---------------------------------------------------------------------------
# helpers
# ---------------------------------------------------------------------------

def setup_basic(harness: Harness, model: str = "mock-model", source: str = "mock-source"):
    backend = harness.backend
    channel_id = backend.create_channel()
    backend.create_model(channel_id, source, model)
    user_id, api_key = backend.create_user()
    return channel_id, user_id, api_key


def wait_for_log(backend, predicate, timeout: float = 8.0):
    deadline = time.time() + timeout
    last = None
    while time.time() < deadline:
        data = backend.admin_ok("GET", "/admin/logs?page=1&page_size=20")
        last = data
        if any(predicate(item) for item in data["items"]):
            return data["items"]
        time.sleep(0.1)
    raise AssertionError(f"no matching request log within {timeout}s (last={last})")


def channel_row(backend, channel_id: int) -> dict:
    data = backend.admin_ok("GET", "/admin/channels?page=1&page_size=100")
    for item in data["items"]:
        if item["id"] == channel_id:
            return item
    raise AssertionError(f"channel {channel_id} not listed")


def sse_text(chunks) -> str:
    return b"".join(chunks).decode("utf-8", errors="replace")


# ---------------------------------------------------------------------------
# P0-5: protocol shape and status codes
# ---------------------------------------------------------------------------

def test_proxy_non_stream_passthrough(harness: Harness):
    backend, mock = harness.backend, harness.mock
    _, _, api_key = setup_basic(harness)

    status, body = backend.api(
        "POST",
        OPENAI_PATH,
        json_body={"model": "mock-model", "messages": [{"role": "user", "content": "hi"}]},
        api_key=api_key,
    )

    assert status == 200, f"expected 200, got {status}: {body}"
    assert isinstance(body, dict), body
    assert "code" not in body, f"proxy response must not be wrapped in the admin envelope: {body}"
    assert body["choices"][0]["message"]["content"] == "pong"

    # The upstream must receive the mapped source model, not the proxy alias.
    upstream = mock.requests()[-1]
    sent = json.loads(upstream["body"])
    assert sent["model"] == "mock-source", sent

    logs = wait_for_log(backend, lambda item: item["total_tokens"] == 18)
    assert logs[0]["status_code"] == 200


def test_proxy_error_status_codes(harness: Harness):
    backend = harness.backend
    setup_basic(harness)

    # Unknown model -> 404 with an OpenAI-style error object.
    status, body = backend.api(
        "POST", OPENAI_PATH, json_body={"model": "nope", "messages": []}, api_key="sk-invalid"
    )
    assert status == 401, f"invalid key must be 401, got {status}"

    _, api_key = backend.create_user("bob")
    status, body = backend.api(
        "POST", OPENAI_PATH, json_body={"model": "nope", "messages": []}, api_key=api_key
    )
    assert status == 404, f"unknown model must be 404, got {status}: {body}"
    assert body["error"]["message"] and "nope" in body["error"]["message"], body


def test_anthropic_error_shape(harness: Harness):
    backend = harness.backend
    backend.create_channel("anthropic-chan", channel_type="anthropic")
    backend.create_model(1, "claude-source", "claude-proxy")
    _, api_key = backend.create_user()

    status, body = backend.api(
        "POST", ANTHROPIC_PATH, json_body={"model": "missing", "messages": []}, api_key=api_key
    )
    assert status == 404, (status, body)
    assert body["type"] == "error", body
    assert "error" in body and body["error"]["message"], body


def test_missing_api_key_is_unauthorized(harness: Harness):
    backend = harness.backend
    status, body = backend.api("POST", OPENAI_PATH, json_body={"model": "x", "messages": []})
    assert status == 401, (status, body)
    assert body["error"]["code"] == "missing_api_key", body


def test_upstream_4xx_is_passed_through(harness: Harness):
    backend, mock = harness.backend, harness.mock
    _, _, api_key = setup_basic(harness)
    mock.configure(chat_status=400, error_body="bad prompt")

    status, body = backend.api(
        "POST", OPENAI_PATH, json_body={"model": "mock-model", "messages": []}, api_key=api_key
    )
    assert status == 400, f"upstream 400 must reach the client, got {status}: {body}"
    assert "bad prompt" in json.dumps(body), body

    # Nothing was charged for a failed request.
    logs = wait_for_log(backend, lambda item: item["status_code"] == 400)
    assert logs[0]["total_tokens"] == 0


def test_proxy_quota_exceeded_returns_402(harness: Harness):
    backend = harness.backend
    channel_id, user_id, api_key = setup_basic(harness)
    backend.create_quota(user_id, quota_type="requests", total_limit=1, cycle="permanent")

    first = backend.api(
        "POST", OPENAI_PATH, json_body={"model": "mock-model", "messages": []}, api_key=api_key
    )
    assert first[0] == 200, first

    second = backend.api(
        "POST", OPENAI_PATH, json_body={"model": "mock-model", "messages": []}, api_key=api_key
    )
    assert second[0] == 402, f"quota exhaustion must be 402, got {second[0]}: {second[1]}"
    assert "quota" in json.dumps(second[1]).lower()


def test_model_permission_is_enforced(harness: Harness):
    backend = harness.backend
    channel_id = backend.create_channel()
    backend.create_model(channel_id, "allowed-source", "allowed-model")
    backend.create_model(channel_id, "other-source", "other-model")

    _, api_key = backend.create_user("carol", enabled_models='["allowed-model"]')

    status, body = backend.api(
        "POST", OPENAI_PATH, json_body={"model": "other-model", "messages": []}, api_key=api_key
    )
    assert status == 403, f"model outside the allow-list must be 403, got {status}: {body}"

    status, body = backend.api(
        "POST", OPENAI_PATH, json_body={"model": "allowed-model", "messages": []}, api_key=api_key
    )
    assert status == 200, (status, body)

    # /v1/models must apply the same restriction.
    status, body = backend.api("GET", "/v1/models", api_key=api_key)
    assert status == 200, (status, body)
    ids = [model["id"] for model in body["data"]]
    assert ids == ["allowed-model"], ids


def test_key_level_model_permission(harness: Harness):
    backend = harness.backend
    channel_id = backend.create_channel()
    backend.create_model(channel_id, "a-source", "a-model")
    backend.create_model(channel_id, "b-source", "b-model")

    user_id, primary_key = backend.create_user("dave")
    token = backend.user_token("dave")

    # A second key restricted to a single model.
    data = backend.admin_ok(
        "POST",
        "/user/keys",
        {"name": "restricted", "enabled_models": '["a-model"]'},
    )
    restricted = data["key_value"]

    assert backend.api(
        "POST", OPENAI_PATH, json_body={"model": "b-model", "messages": []}, api_key=restricted
    )[0] == 403

    # The primary key is unrestricted and still works.
    assert backend.api(
        "POST", OPENAI_PATH, json_body={"model": "b-model", "messages": []}, api_key=primary_key
    )[0] == 200

    # /v1/models reflects the key restriction, not the user's list.
    status, body = backend.api("GET", "/v1/models", api_key=restricted)
    assert status == 200
    assert [m["id"] for m in body["data"]] == ["a-model"]


def test_disabled_key_is_rejected(harness: Harness):
    backend = harness.backend
    channel_id = backend.create_channel()
    backend.create_model(channel_id, "src", "m")
    _, primary_key = backend.create_user("erin")

    token = backend.user_token("erin")
    status, body = backend.api("GET", "/user/keys", token=token)
    assert status == 200, (status, body)
    default_key = body["data"][0]

    # Disabling the mirrored primary key row must revoke the key.
    backend.api("PUT", f"/user/keys/{default_key['id']}", json_body={"status": "disabled"}, token=token)

    status, body = backend.api(
        "POST", OPENAI_PATH, json_body={"model": "m", "messages": []}, api_key=primary_key
    )
    assert status == 401, f"disabled key must be rejected, got {status}: {body}"
    assert body["error"]["code"] == "api_key_revoked", body

    # Re-enabling restores access.
    status, _ = backend.api(
        "PUT", f"/user/keys/{default_key['id']}", json_body={"status": "active"}, token=token
    )
    assert status == 200
    assert backend.api(
        "POST", OPENAI_PATH, json_body={"model": "m", "messages": []}, api_key=primary_key
    )[0] == 200


def test_primary_key_cannot_be_deleted(harness: Harness):
    backend = harness.backend
    backend.create_user("frank")
    token = backend.user_token("frank")

    status, body = backend.api("GET", "/user/keys", token=token)
    key_id = body["data"][0]["id"]

    status, body = backend.api("DELETE", f"/user/keys/{key_id}", token=token)
    assert status == 400, f"primary key deletion must be refused, got {status}: {body}"


# ---------------------------------------------------------------------------
# P0-6: streaming
# ---------------------------------------------------------------------------

def test_streaming_passthrough_and_usage(harness: Harness):
    backend, mock = harness.backend, harness.mock
    _, user_id, api_key = setup_basic(harness)
    backend.create_quota(user_id, quota_type="tokens", total_limit=1000, cycle="permanent")

    started = time.time()
    text = sse_text(
        backend.stream(
            OPENAI_PATH,
            {"model": "mock-model", "messages": [], "stream": True},
            api_key,
        )
    )
    elapsed = time.time() - started

    assert "chunk0" in text and "[DONE]" in text, text[:400]
    assert elapsed < 10, f"stream took too long: {elapsed:.1f}s"

    logs = wait_for_log(backend, lambda item: item["total_tokens"] == 18)
    entry = logs[0]
    assert entry["status_code"] == 200, entry
    assert entry["completion_tokens"] == 7, entry
    assert entry["elapsed_ms"] > 0, f"stream elapsed must be measured, got {entry}"

    quota = backend.sql("SELECT used FROM quotas WHERE user_id = ?", (user_id,))
    assert quota and quota[0][0] == 18, f"token quota must use real usage, got {quota}"

    # The proxy asks the upstream for usage on streaming responses.
    sent = json.loads(mock.requests()[-1]["body"])
    assert sent["stream_options"]["include_usage"] is True, sent


def test_streaming_error_is_not_charged(harness: Harness):
    backend, mock = harness.backend, harness.mock
    _, user_id, api_key = setup_basic(harness)
    backend.create_quota(user_id, quota_type="tokens", total_limit=1000, cycle="permanent")
    mock.configure(stream_status=401)

    status, body = backend.api(
        "POST",
        OPENAI_PATH,
        json_body={"model": "mock-model", "messages": [], "stream": True},
        api_key=api_key,
    )
    assert status == 401, f"upstream stream error must surface, got {status}: {body}"

    quota = backend.sql("SELECT used FROM quotas WHERE user_id = ?", (user_id,))
    assert quota and quota[0][0] == 0, f"failed stream must not be charged: {quota}"

    logs = wait_for_log(backend, lambda item: item["status_code"] == 401)
    assert logs[0]["total_tokens"] == 0


def test_streaming_falls_back_to_second_channel(harness: Harness):
    backend = harness.backend
    fallback = MockUpstream().start()
    try:
        fallback.configure(stream_status=503)
        # High weight so the failing channel is attempted first.
        primary = backend.create_channel(
            "primary", base_url=f"{fallback.base_url}/v1", weight=100
        )
        backend.create_model(primary, "mock-source", "mock-model")
        secondary = backend.create_channel(
            "secondary", base_url=f"{harness.mock.base_url}/v1", weight=1
        )
        backend.create_model(secondary, "mock-source", "mock-model")
        _, api_key = backend.create_user()

        text = sse_text(
            backend.stream(
                OPENAI_PATH,
                {"model": "mock-model", "messages": [], "stream": True},
                api_key,
            )
        )
        assert "chunk0" in text, text[:400]
        assert len([r for r in fallback.requests() if r["method"] == "POST"]) >= 1, (
            "the failing channel must have been attempted"
        )
    finally:
        fallback.stop()


def test_anthropic_stream_records_cache_tokens(harness: Harness):
    backend, mock = harness.backend, harness.mock
    channel_id = backend.create_channel("anthropic", channel_type="anthropic")
    backend.create_model(channel_id, "claude-3", "claude-alias")
    _, api_key = backend.create_user()

    text = sse_text(
        backend.stream(
            ANTHROPIC_PATH,
            {"model": "claude-alias", "messages": [], "stream": True, "max_tokens": 16},
            api_key,
        )
    )
    assert "message_start" in text and "message_stop" in text, text[:400]

    # prompt = input 10 + cache_read 2 = 12, completion = 5
    logs = wait_for_log(backend, lambda item: item["total_tokens"] == 17)
    assert logs[0]["prompt_tokens"] == 12, logs[0]

    headers = mock.requests()[-1]["headers"]
    assert headers.get("anthropic-version") == "2023-06-01", headers


def test_anthropic_accepts_x_api_key(harness: Harness):
    backend = harness.backend
    channel_id = backend.create_channel("anthropic", channel_type="anthropic")
    backend.create_model(channel_id, "claude-3", "claude-alias")
    _, api_key = backend.create_user()

    status, body = backend.api(
        "POST",
        ANTHROPIC_PATH,
        json_body={"model": "claude-alias", "messages": [], "max_tokens": 8},
        headers={"x-api-key": api_key},
    )
    assert status == 200, (status, body)
    assert body["content"][0]["text"] == "pong"


# ---------------------------------------------------------------------------
# P0-7: timeouts
# ---------------------------------------------------------------------------

def test_upstream_timeout_is_bounded(harness: Harness):
    backend, mock = harness.backend, harness.mock
    channel_id = backend.create_channel(timeout=1000)
    backend.create_model(channel_id, "slow-source", "slow-model")
    _, api_key = backend.create_user()
    mock.configure(chat_delay=5)

    started = time.time()
    status, body = backend.api(
        "POST", OPENAI_PATH, json_body={"model": "slow-model", "messages": []}, api_key=api_key
    )
    elapsed = time.time() - started

    assert elapsed < 10, f"timeout not enforced, took {elapsed:.1f}s"
    assert status >= 500, f"timed out upstream must be a gateway error, got {status}: {body}"


def test_stream_idle_timeout_terminates(harness: Harness):
    backend, mock = harness.backend, harness.mock
    channel_id = backend.create_channel(timeout=1000)
    backend.create_model(channel_id, "stall-source", "stall-model")
    _, api_key = backend.create_user()
    mock.configure(stream_stall_after=5)

    started = time.time()
    received = []
    handle = backend.open_stream(
        OPENAI_PATH,
        {"model": "stall-model", "messages": [], "stream": True},
        api_key,
        timeout=20,
    )
    try:
        assert handle.status == 200, f"stream must start successfully: {handle.status} {handle.reason}"
        for chunk in handle:
            received.append(chunk)
            if time.time() - started > 15:
                break
    except Exception:  # noqa: BLE001 - a broken stream is acceptable here
        pass
    finally:
        handle.close()
    elapsed = time.time() - started

    assert elapsed < 12, f"stalled stream was not cut off, took {elapsed:.1f}s"
    assert received, "the first chunk should still be forwarded"


# ---------------------------------------------------------------------------
# P0-4: concurrency accounting
# ---------------------------------------------------------------------------

def set_global_limit(backend, qps: int, concurrency: int):
    backend.admin_ok(
        "PUT",
        "/admin/system/config/rate-limit",
        {"qps": qps, "concurrency": concurrency, "action": "reject"},
    )


def test_global_concurrency_slots_are_released(harness: Harness):
    backend = harness.backend
    _, _, api_key = setup_basic(harness)
    set_global_limit(backend, qps=1000, concurrency=1)

    def call():
        return backend.api(
            "POST", OPENAI_PATH, json_body={"model": "mock-model", "messages": []}, api_key=api_key
        )

    # Sequential calls must keep working: a leaked slot would make the second one 429.
    for index in range(5):
        status, body = call()
        assert status == 200, f"request {index} failed after slot release: {status} {body}"


def test_cancelled_stream_releases_slot(harness: Harness):
    backend, mock = harness.backend, harness.mock
    _, _, api_key = setup_basic(harness)
    set_global_limit(backend, qps=1000, concurrency=1)
    mock.configure(stream_chunks=20, stream_delay=0.2)

    # Start a stream and abandon it after the first chunk.
    stream = backend.stream(
        OPENAI_PATH, {"model": "mock-model", "messages": [], "stream": True}, api_key, timeout=10
    )
    next(stream)
    stream.close()

    # The slot must come back once the upstream stream is aborted.
    deadline = time.time() + 10
    last = None
    while time.time() < deadline:
        status, body = backend.api(
            "POST", OPENAI_PATH, json_body={"model": "mock-model", "messages": []}, api_key=api_key
        )
        last = (status, body)
        if status == 200:
            return
        time.sleep(0.3)
    raise AssertionError(f"concurrency slot leaked after stream cancellation: {last}")


def test_concurrent_stream_holds_slot(harness: Harness):
    backend, mock = harness.backend, harness.mock
    _, _, api_key = setup_basic(harness)
    set_global_limit(backend, qps=1000, concurrency=1)
    mock.configure(stream_chunks=4, stream_delay=0.3)

    result = {}

    def run_stream():
        result["text"] = sse_text(
            backend.stream(
                OPENAI_PATH, {"model": "mock-model", "messages": [], "stream": True}, api_key
            )
        )

    thread = threading.Thread(target=run_stream)
    thread.start()
    time.sleep(0.4)  # let the stream start and take the only slot

    status, body = backend.api(
        "POST", OPENAI_PATH, json_body={"model": "mock-model", "messages": []}, api_key=api_key
    )
    thread.join(timeout=15)

    assert status == 429, f"a running stream must hold the concurrency slot: {status} {body}"
    assert "chunk0" in result.get("text", "")


# ---------------------------------------------------------------------------
# P0-3: channel quota
# ---------------------------------------------------------------------------

def test_channel_request_quota_counts_requests_and_resets(harness: Harness):
    backend = harness.backend
    channel_id = backend.create_channel(
        quota_type="requests", quota_limit=2, quota_cycle="hourly"
    )
    backend.create_model(channel_id, "src", "m")
    _, api_key = backend.create_user()

    def call():
        return backend.api(
            "POST", OPENAI_PATH, json_body={"model": "m", "messages": []}, api_key=api_key
        )

    assert call()[0] == 200
    assert call()[0] == 200
    status, body = call()
    assert status == 402, f"request quota must stop at the limit, got {status}: {body}"

    stats = backend.admin_ok("GET", f"/admin/channels/{channel_id}/stats")
    assert stats["quota_used"] == 2, f"requests must count as 1 each, got {stats}"

    # Simulate the period elapsing: the next request must reset and succeed.
    backend.sql("UPDATE channels SET quota_period_end = datetime('now', '-1 hour') WHERE id = ?", (channel_id,))
    assert call()[0] == 200
    stats = backend.admin_ok("GET", f"/admin/channels/{channel_id}/stats")
    assert stats["quota_used"] == 1, f"quota must reset before charging, got {stats}"


def test_channel_token_quota_counts_tokens(harness: Harness):
    backend = harness.backend
    channel_id = backend.create_channel(quota_type="tokens", quota_limit=30, quota_cycle="hourly")
    backend.create_model(channel_id, "src", "m")
    _, api_key = backend.create_user()

    def call():
        return backend.api(
            "POST", OPENAI_PATH, json_body={"model": "m", "messages": []}, api_key=api_key
        )

    # 18 tokens per call, limit 30: the first two fit (the check only knows the
    # estimated cost, so accounting may overshoot by a single call).
    assert call()[0] == 200
    assert call()[0] == 200
    status, body = call()
    assert status == 402, f"token quota must stop once usage reaches the limit: {status} {body}"

    stats = backend.admin_ok("GET", f"/admin/channels/{channel_id}/stats")
    assert stats["quota_used"] == 36, f"token quota must count real tokens, got {stats}"


def test_channel_quota_exhaustion_falls_back_to_other_channel(harness: Harness):
    backend = harness.backend
    fallback = MockUpstream().start()
    try:
        limited = backend.create_channel(
            "limited",
            base_url=f"{fallback.base_url}/v1",
            weight=100,
            quota_type="requests",
            quota_limit=1,
            quota_cycle="hourly",
        )
        backend.create_model(limited, "mock-source", "mock-model")
        backup = backend.create_channel(
            "backup", base_url=f"{harness.mock.base_url}/v1", weight=1
        )
        backend.create_model(backup, "mock-source", "mock-model")
        _, api_key = backend.create_user()

        for index in range(4):
            status, body = backend.api(
                "POST",
                OPENAI_PATH,
                json_body={"model": "mock-model", "messages": []},
                api_key=api_key,
            )
            assert status == 200, f"request {index} should fall back, got {status}: {body}"

        assert len([r for r in harness.mock.requests() if r["method"] == "POST"]) >= 1
    finally:
        fallback.stop()


# ---------------------------------------------------------------------------
# P0-2: health checks must not disable encrypted channels
# ---------------------------------------------------------------------------

def wait_for_channel_state(backend, channel_id: int, predicate, timeout: float = 60.0):
    deadline = time.time() + timeout
    last = None
    while time.time() < deadline:
        last = channel_row(backend, channel_id)
        if predicate(last):
            return last
        time.sleep(0.5)
    raise AssertionError(f"channel state never matched within {timeout}s: {last}")


@custom_harness
def test_health_check_keeps_encrypted_channels():
    with Harness(health_interval_secs=10) as harness:
        backend, mock = harness.backend, harness.mock
        channel_id = backend.create_channel()

        # At least one probe must reach the upstream with the *decrypted* key.
        deadline = time.time() + 30
        probe_seen = False
        while time.time() < deadline:
            for request in mock.requests():
                if request["method"] == "GET" and request["path"].endswith("/models"):
                    auth = request["headers"].get("authorization", "")
                    assert "sk-upstream-1" in auth, f"probe sent a non-plaintext key: {auth}"
                    probe_seen = True
            if probe_seen:
                break
            time.sleep(0.5)

        assert probe_seen, "health checker never probed the channel"

        # Give it two more sweeps: the channel must stay active.
        time.sleep(12)
        row = channel_row(backend, channel_id)
        assert row["status"] == "active", row
        assert row["consecutive_failures"] == 0, row


@custom_harness
def test_health_check_disables_then_recovers():
    with Harness(health_interval_secs=10) as harness:
        backend, mock = harness.backend, harness.mock
        # No valid key => every probe is rejected.
        mock.configure(valid_keys=[])
        channel_id = backend.create_channel()

        row = wait_for_channel_state(
            backend, channel_id, lambda item: item["status"] == "disabled", timeout=45
        )
        assert row["auto_disabled"] is True, row

        # Restore provider access: the checker must re-probe and re-enable.
        mock.configure(valid_keys=["sk-upstream-1"])
        row = wait_for_channel_state(
            backend, channel_id, lambda item: item["status"] == "active", timeout=60
        )
        assert row["auto_disabled"] is False, row
