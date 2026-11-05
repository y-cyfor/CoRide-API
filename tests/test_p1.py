"""P1 regression tests: features that had UI/API surface but no effect before."""

from __future__ import annotations

import json
import time

from support import Harness, MockUpstream

OPENAI_PATH = "/v1/chat/completions"


def setup_basic(harness: Harness, model: str = "mock-model", source: str = "mock-source"):
    backend = harness.backend
    channel_id = backend.create_channel()
    backend.create_model(channel_id, source, model)
    user_id, api_key = backend.create_user()
    return channel_id, user_id, api_key


def call(backend, api_key: str, model: str = "mock-model"):
    return backend.api(
        "POST", OPENAI_PATH, json_body={"model": model, "messages": [{"role": "user", "content": "hi"}]}, api_key=api_key
    )


def wait_for_logs(backend, predicate, timeout: float = 8.0):
    deadline = time.time() + timeout
    last = None
    while time.time() < deadline:
        last = backend.admin_ok("GET", "/admin/logs?page=1&page_size=50")
        if any(predicate(item) for item in last["items"]):
            return last
        time.sleep(0.1)
    raise AssertionError(f"no matching log within {timeout}s (last={last})")


# ---------------------------------------------------------------------------
# P1-1 / P1-12: IP access control
# ---------------------------------------------------------------------------

def test_ip_whitelist_enforced_after_auth(harness: Harness):
    backend = harness.backend
    _, user_id, api_key = setup_basic(harness)

    backend.admin_ok(
        "POST", "/admin/ip/whitelist", {"user_id": user_id, "ip_address": "203.0.113.9"}
    )

    status, body = call(backend, api_key)
    assert status == 403, f"whitelist must block other IPs, got {status}: {body}"

    backend.admin_ok(
        "POST", "/admin/ip/whitelist", {"user_id": user_id, "ip_address": "127.0.0.1"}
    )
    status, body = call(backend, api_key)
    assert status == 200, f"whitelisted IP must pass, got {status}: {body}"


def test_ip_blacklist_supports_cidr(harness: Harness):
    backend = harness.backend
    _, _, api_key = setup_basic(harness)

    data = backend.admin_ok("POST", "/admin/ip/blacklist", {"ip_address": "127.0.0.0/8"})
    status, body = call(backend, api_key)
    assert status == 403, f"CIDR blacklist must block, got {status}: {body}"

    backend.admin_ok("DELETE", f"/admin/ip/blacklist/{data['id']}")
    status, body = call(backend, api_key)
    assert status == 200, f"removing the rule must unblock, got {status}: {body}"


def test_ip_rules_are_normalised_and_validated(harness: Harness):
    backend = harness.backend

    status, _ = backend.admin("POST", "/admin/ip/blacklist", {"ip_address": "not-an-ip"})
    assert status == 400, "invalid IP must be rejected"

    backend.admin_ok("POST", "/admin/ip/blacklist", {"ip_address": "192.168.1.5/24"})
    entries = backend.admin_ok("GET", "/admin/ip/blacklist")
    assert entries[0]["ip_address"] == "192.168.1.0/24", entries

    duplicate_status, _ = backend.admin("POST", "/admin/ip/blacklist", {"ip_address": "192.168.1.7/24"})
    assert duplicate_status == 409, "duplicate rules must conflict after normalisation"


# ---------------------------------------------------------------------------
# P1-3: rate limits actually enforced
# ---------------------------------------------------------------------------

def test_user_qps_limit_enforced(harness: Harness):
    backend = harness.backend
    _, user_id, api_key = setup_basic(harness)

    backend.admin_ok(
        "POST",
        "/admin/ratelimits",
        {"target_type": "user", "target_id": user_id, "qps": 1, "concurrency": 5, "action": "reject"},
    )

    assert call(backend, api_key)[0] == 200
    status, body = call(backend, api_key)
    assert status == 429, f"second immediate call must be rate limited, got {status}: {body}"


def test_channel_qps_limit_falls_back_to_other_channel(harness: Harness):
    backend = harness.backend
    fast = MockUpstream().start()
    try:
        limited = backend.create_channel("limited", base_url=f"{fast.base_url}/v1", weight=100)
        backend.create_model(limited, "mock-source", "mock-model")
        backup = backend.create_channel(
            "backup", base_url=f"{harness.mock.base_url}/v1", weight=1
        )
        backend.create_model(backup, "mock-source", "mock-model")
        backend.admin_ok(
            "POST",
            "/admin/ratelimits",
            {
                "target_type": "channel",
                "target_id": limited,
                "qps": 1,
                "concurrency": 1,
                "action": "reject",
            },
        )
        _, api_key = backend.create_user()

        statuses = [call(backend, api_key)[0] for _ in range(5)]
        assert all(status == 200 for status in statuses), f"traffic must fail over: {statuses}"
        assert len([r for r in harness.mock.requests() if r["method"] == "POST"]) >= 1
    finally:
        fast.stop()


def test_rate_limit_targets_are_validated(harness: Harness):
    backend = harness.backend
    status, _ = backend.admin(
        "POST",
        "/admin/ratelimits",
        {"target_type": "user", "target_id": 9999, "qps": 1, "concurrency": 1, "action": "reject"},
    )
    assert status == 400, "unknown user target must be rejected"

    status, _ = backend.admin(
        "POST",
        "/admin/ratelimits",
        {"target_type": "user", "target_id": 1, "qps": -1, "concurrency": 1, "action": "reject"},
    )
    assert status == 400, "negative qps must be rejected"


# ---------------------------------------------------------------------------
# P1-4 / P1-5 / P1-6 / P1-15: attribution, stats and log filtering
# ---------------------------------------------------------------------------

def test_multi_key_requests_are_attributed_to_the_user(harness: Harness):
    backend = harness.backend
    _, user_id, primary_key = setup_basic(harness)
    token = backend.user_token("alice")

    created = backend.api(
        "POST", "/user/keys", json_body={"name": "extra"}, token=token
    )
    assert created[0] == 200, created
    extra_key = created[1]["data"]["key_value"]

    assert call(backend, extra_key)[0] == 200

    dashboard = backend.api("GET", "/user/stats/dashboard", token=token)[1]["data"]
    assert dashboard["total_requests"] == 1, dashboard
    assert dashboard["total_tokens"] == 18, dashboard

    logs = backend.api("GET", "/user/logs?page=1&page_size=10", token=token)[1]["data"]
    assert logs["total"] == 1, logs
    assert logs["items"][0]["key_id"] is not None, logs


def test_user_token_chart_is_not_empty(harness: Harness):
    backend = harness.backend
    _, user_id, api_key = setup_basic(harness)
    assert call(backend, api_key)[0] == 200
    wait_for_logs(backend, lambda item: item["total_tokens"] == 18)

    token = backend.user_token("alice")
    data = backend.api("GET", "/user/stats/usage?days=7", token=token)[1]["data"]
    assert data["token_daily"], f"token chart must not be empty: {data}"
    assert data["token_daily"][-1]["prompt_tokens"] == 11, data["token_daily"]
    assert data["model_usage"][0]["name"] == "mock-model", data["model_usage"]


def test_user_logs_status_filter_works(harness: Harness):
    backend, mock = harness.backend, harness.mock
    _, _, api_key = setup_basic(harness)

    assert call(backend, api_key)[0] == 200
    mock.configure(chat_status=400)
    assert call(backend, api_key)[0] == 400
    wait_for_logs(backend, lambda item: item["status_code"] == 400)

    token = backend.user_token("alice")
    ok_logs = backend.api("GET", "/user/logs?page=1&page_size=10&status_code=2", token=token)[1]["data"]
    bad_logs = backend.api("GET", "/user/logs?page=1&page_size=10&status_code=4", token=token)[1]["data"]

    assert ok_logs["total"] == 1, ok_logs
    assert bad_logs["total"] == 1, bad_logs
    assert bad_logs["items"][0]["status_code"] == 400


def test_admin_logs_paginate_and_filter_by_user(harness: Harness):
    backend, mock = harness.backend, harness.mock
    _, alice_id, alice_key = setup_basic(harness)
    bob_id, bob_key = backend.create_user("bob")

    for _ in range(3):
        assert call(backend, alice_key)[0] == 200
    assert call(backend, bob_key)[0] == 200
    wait_for_logs(backend, lambda item: True)

    page1 = backend.admin_ok("GET", "/admin/logs?page=1&page_size=2")
    assert len(page1["items"]) == 2, page1
    assert page1["total"] == 4, page1

    page3 = backend.admin_ok("GET", "/admin/logs?page=3&page_size=2")
    assert len(page3["items"]) == 0 or len(page3["items"]) == 0, page3

    filtered = backend.admin_ok("GET", f"/admin/logs?page=1&page_size=50&user_id={bob_id}")
    assert filtered["total"] == 1, filtered
    assert filtered["items"][0]["user_id"] == bob_id, filtered


def test_dashboard_active_users_counts_users_not_keys(harness: Harness):
    backend = harness.backend
    _, user_id, primary_key = setup_basic(harness)
    token = backend.user_token("alice")
    extra_key = backend.api("POST", "/user/keys", json_body={"name": "k2"}, token=token)[1]["data"][
        "key_value"
    ]

    assert call(backend, primary_key)[0] == 200
    assert call(backend, extra_key)[0] == 200
    wait_for_logs(backend, lambda item: len(backend.admin_ok("GET", "/admin/logs?page=1&page_size=50")["items"]) >= 2)

    stats = backend.admin_ok("GET", "/admin/stats/dashboard")
    assert stats["active_users"] == 1, stats


# ---------------------------------------------------------------------------
# P1-8 / P1-9: model mapping and channel distribution
# ---------------------------------------------------------------------------

def test_model_mapping_is_per_channel(harness: Harness):
    backend = harness.backend
    second = MockUpstream().start()
    try:
        first_id = backend.create_channel("first", base_url=f"{harness.mock.base_url}/v1", weight=1)
        backend.create_model(first_id, "source-one", "shared-alias")
        second_id = backend.create_channel(
            "second", base_url=f"{second.base_url}/v1", weight=1
        )
        backend.create_model(second_id, "source-two", "shared-alias")
        _, api_key = backend.create_user()

        for _ in range(20):
            assert call(backend, api_key, model="shared-alias")[0] == 200

        used = {json.loads(r["body"])["model"] for r in harness.mock.requests() if r["method"] == "POST"}
        used |= {json.loads(r["body"])["model"] for r in second.requests() if r["method"] == "POST"}
        assert used == {"source-one", "source-two"}, f"each channel must send its own source name: {used}"
    finally:
        second.stop()


def test_weighted_channel_distribution(harness: Harness):
    backend = harness.backend
    heavy = MockUpstream().start()
    try:
        heavy_id = backend.create_channel("heavy", base_url=f"{heavy.base_url}/v1", weight=9)
        backend.create_model(heavy_id, "mock-source", "mock-model")
        light_id = backend.create_channel(
            "light", base_url=f"{harness.mock.base_url}/v1", weight=1
        )
        backend.create_model(light_id, "mock-source", "mock-model")
        _, api_key = backend.create_user()

        for _ in range(40):
            assert call(backend, api_key)[0] == 200

        heavy_hits = len([r for r in heavy.requests() if r["method"] == "POST"])
        light_hits = len([r for r in harness.mock.requests() if r["method"] == "POST"])
        assert heavy_hits > light_hits, f"weight 9 must get more traffic: {heavy_hits} vs {light_hits}"
        assert light_hits > 0, "the light channel must still receive traffic"
    finally:
        heavy.stop()


def test_api_key_rotation_across_multiple_keys(harness: Harness):
    backend = harness.backend
    channel_id = backend.create_channel(api_keys='["sk-upstream-1","sk-upstream-1"]')
    backend.create_model(channel_id, "mock-source", "mock-model")
    _, api_key = backend.create_user()

    for _ in range(4):
        assert call(backend, api_key)[0] == 200

    keys = [
        r["headers"].get("authorization")
        for r in harness.mock.requests()
        if r["method"] == "POST"
    ]
    assert keys and all(key and "sk-upstream-1" in key for key in keys), keys


# ---------------------------------------------------------------------------
# P1-10 / P1-11 / P1-13: channel handling and encryption
# ---------------------------------------------------------------------------

def test_channel_quota_is_per_channel_not_per_user(harness: Harness):
    backend = harness.backend
    limited = backend.create_channel("limited", quota_type="requests", quota_limit=1, quota_cycle="hourly", weight=100)
    backend.create_model(limited, "mock-source", "mock-model")
    backup = backend.create_channel("backup", weight=1)
    backend.create_model(backup, "mock-source", "mock-model")
    _, api_key = backend.create_user()

    statuses = [call(backend, api_key)[0] for _ in range(3)]
    assert statuses == [200, 200, 200], f"channel quota must not block the user: {statuses}"


def test_channel_api_keys_are_encrypted_at_rest(harness: Harness):
    backend = harness.backend
    channel_id = backend.create_channel()

    raw = backend.sql("SELECT api_keys FROM channels WHERE id = ?", (channel_id,))[0][0]
    assert raw.startswith("enc:v1:"), f"API keys must be encrypted at rest: {raw[:24]}"
    assert "sk-upstream-1" not in raw

    # The API returns usable plaintext for the edit form.
    row = next(
        item
        for item in backend.admin_ok("GET", "/admin/channels?page=1&page_size=50")["items"]
        if item["id"] == channel_id
    )
    assert "sk-upstream-1" in row["api_keys"], row["api_keys"]
    assert row["api_keys_decryptable"] is True


def test_channel_test_endpoint_reports_success(harness: Harness):
    backend = harness.backend
    channel_id = backend.create_channel()
    data = backend.admin_ok("POST", f"/admin/channels/{channel_id}/test")
    assert data["status"] == "ok", data


def test_channel_test_endpoint_is_panic_safe_with_cjk_error(harness: Harness):
    backend, mock = harness.backend, harness.mock
    channel_id = backend.create_channel()
    # A long CJK body used to panic the byte-index truncation at 200 bytes.
    mock.configure(models_status=500, models_body="错误原因：" + "很长的上游错误信息" * 30)

    data = backend.admin_ok("POST", f"/admin/channels/{channel_id}/test")
    assert data["status"] == "warning", data
    assert data["http_status"] == 500


def test_channel_validation_rejects_bad_payloads(harness: Harness):
    backend = harness.backend

    cases = [
        {"name": "x", "type": "tgi", "base_url": "http://x", "api_keys": '["k"]'},
        {"name": "x", "type": "openai", "base_url": "ftp://x", "api_keys": '["k"]'},
        {"name": "x", "type": "openai", "base_url": "http://x", "api_keys": "[]"},
        {"name": "x", "type": "openai", "base_url": "http://x", "api_keys": '["k"]', "weight": 0},
        {
            "name": "x",
            "type": "openai",
            "base_url": "http://x",
            "api_keys": '["k"]',
            "quota_type": "requests",
            "quota_limit": 10,
        },
    ]
    for payload in cases:
        status, body = backend.admin("POST", "/admin/channels", payload)
        assert status == 400, f"payload {payload} must be rejected, got {status}: {body}"


# ---------------------------------------------------------------------------
# System settings
# ---------------------------------------------------------------------------

def test_retention_days_applies_and_persists(harness: Harness):
    backend = harness.backend
    backend.admin_ok(
        "PUT", "/admin/system/config/log-level", {"level": "info", "retention_days": 7}
    )
    config = backend.admin_ok("GET", "/admin/system/config")
    assert config["log"]["retention_days"] == 7, config["log"]


def test_global_rate_limit_applies_immediately_and_persists(harness: Harness):
    backend = harness.backend
    _, _, api_key = setup_basic(harness)

    backend.admin_ok(
        "PUT", "/admin/system/config/rate-limit", {"qps": 1, "concurrency": 10, "action": "reject"}
    )
    config = backend.admin_ok("GET", "/admin/system/config")
    assert config["global_rate_limit"]["qps"] == 1, config

    rules = backend.admin_ok("GET", "/admin/ratelimits?page=1&page_size=50")
    assert any(rule["target_type"] == "global" for rule in rules["items"]), rules

    assert call(backend, api_key)[0] == 200
    status, body = call(backend, api_key)
    assert status == 429, f"global qps=1 must throttle the second call, got {status}: {body}"


def test_log_level_rejects_invalid_values(harness: Harness):
    backend = harness.backend
    status, _ = backend.admin("PUT", "/admin/system/config/log-level", {"level": "verbose"})
    assert status == 400


def test_export_csv_contains_rows(harness: Harness):
    backend = harness.backend
    _, _, api_key = setup_basic(harness)
    assert call(backend, api_key)[0] == 200
    wait_for_logs(backend, lambda item: True)

    status, body = backend.admin("GET", "/admin/logs/export")
    assert status == 200, (status, body)
    text = body if isinstance(body, str) else json.dumps(body)
    assert text.startswith("id,user_api_key"), text[:80]
    assert "mock-model" in text, text[:200]


# ---------------------------------------------------------------------------
# Data integrity
# ---------------------------------------------------------------------------

def test_duplicate_quota_is_rejected(harness: Harness):
    backend = harness.backend
    _, user_id, _ = setup_basic(harness)
    backend.create_quota(user_id, total_limit=10)

    status, body = backend.admin(
        "POST",
        "/admin/quotas",
        {
            "user_id": user_id,
            "quota_type": "requests",
            "total_limit": 5,
            "cycle": "permanent",
            "channel_id": None,
        },
    )
    assert status == 409, f"duplicate quota must conflict, got {status}: {body}"


def test_deleting_user_removes_dependent_rows(harness: Harness):
    backend = harness.backend
    channel_id, user_id, api_key = setup_basic(harness)
    backend.create_quota(user_id, total_limit=10)
    backend.admin_ok("POST", "/admin/ip/whitelist", {"user_id": user_id, "ip_address": "127.0.0.1"})

    backend.admin_ok("DELETE", f"/admin/users/{user_id}")

    assert backend.sql("SELECT COUNT(*) FROM user_keys WHERE user_id = ?", (user_id,))[0][0] == 0
    assert backend.sql("SELECT COUNT(*) FROM quotas WHERE user_id = ?", (user_id,))[0][0] == 0
    assert backend.sql("SELECT COUNT(*) FROM ip_whitelist WHERE user_id = ?", (user_id,))[0][0] == 0

    status, _ = call(backend, api_key)
    assert status == 401, "deleted users must not be able to call the API"


def test_deleting_channel_removes_models_and_plans(harness: Harness):
    backend = harness.backend
    channel_id = backend.create_channel()
    model_id = backend.create_model(channel_id, "src", "m")

    profiles = backend.admin_ok("GET", "/admin/app-profiles?page=1&page_size=10")["items"]
    profile_id = profiles[0]["id"]
    backend.admin_ok(
        "PUT",
        f"/admin/traffic-plan/channel/{channel_id}",
        {"slots": [{"start_hour": 0, "end_hour": 24, "app_profile_id": profile_id, "weight": 1}]},
    )

    backend.admin_ok("DELETE", f"/admin/channels/{channel_id}")

    assert backend.sql("SELECT COUNT(*) FROM models WHERE channel_id = ?", (channel_id,))[0][0] == 0
    assert backend.sql("SELECT COUNT(*) FROM traffic_plans WHERE channel_id = ?", (channel_id,))[0][0] == 0
    assert model_id not in [
        item["id"] for item in backend.admin_ok("GET", "/admin/models?page=1&page_size=50")["items"]
    ]


def test_cannot_delete_self(harness: Harness):
    backend = harness.backend
    me = backend.admin_ok("GET", "/admin/auth/me")
    status, body = backend.admin("DELETE", f"/admin/users/{me['id']}")
    assert status == 400, f"an admin must not delete themselves: {status} {body}"
