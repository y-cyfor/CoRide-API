"""Upgrade test: start the current binary on a database created by the previous schema.

The 016 migration repairs channels whose request quota was exhausted by the old
token-based accounting and initialises missing channel quota periods, so an existing
deployment must come back healthy without manual SQL.
"""

from __future__ import annotations

import hashlib
import pathlib
import sqlite3

from support import Backend, MockUpstream, ROOT, custom_harness

MIGRATIONS = sorted(
    (ROOT / "backend" / "src" / "db" / "migrations").glob("0*.sql")
)
# Everything up to and including 015 matches the schema shipped before this round.
LEGACY_MIGRATIONS = [path for path in MIGRATIONS if int(path.name.split("_")[0]) <= 15]


def build_legacy_database(db_path: pathlib.Path) -> None:
    """Create a database that looks like one produced by the previous release."""
    db_path.parent.mkdir(parents=True, exist_ok=True)
    connection = sqlite3.connect(db_path)
    try:
        connection.executescript(
            """
            CREATE TABLE IF NOT EXISTS _sqlx_migrations (
                version BIGINT PRIMARY KEY,
                description TEXT NOT NULL,
                installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                success BOOLEAN NOT NULL,
                checksum BLOB NOT NULL,
                execution_time BIGINT NOT NULL
            );
            """
        )
        for path in LEGACY_MIGRATIONS:
            sql = path.read_text(encoding="utf-8")
            connection.executescript(sql)
            version = int(path.name.split("_")[0])
            description = path.stem.split("_", 1)[1]
            checksum = hashlib.sha384(path.read_bytes()).digest()
            connection.execute(
                "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)"
                " VALUES (?, ?, 1, ?, 0)",
                (version, description, checksum),
            )

        # A channel in the exact state the old accounting produced: a request quota of
        # 10 that has been charged in tokens (500), with no period configured.
        connection.execute(
            """
            INSERT INTO channels (name, type, base_url, api_keys, status, weight, timeout,
                                  retry_count, quota_type, quota_limit, quota_used, quota_cycle)
            VALUES ('legacy', 'openai', 'http://127.0.0.1:1/v1', '["sk-upstream-1"]', 'active',
                    1, 30000, 1, 'requests', 10, 500, 'daily')
            """
        )
        connection.commit()
    finally:
        connection.close()


@custom_harness
def test_legacy_database_is_repaired_on_startup():
    mock = MockUpstream().start()
    backend = Backend(mock)
    try:
        (backend.workdir / "data").mkdir(parents=True, exist_ok=True)
        build_legacy_database(backend.workdir / "data" / "test.db")

        backend.start()

        # Migration 016 cleared the impossible usage counter and set a period.
        row = backend.sql(
            "SELECT quota_used, quota_period_end FROM channels WHERE name = 'legacy'"
        )
        assert row, "legacy channel disappeared"
        quota_used, period_end = row[0]
        assert quota_used == 0, f"exhausted legacy quota was not repaired: {quota_used}"
        assert period_end, "channel quota period was not initialised"

        # The channel is routable again: point it at the mock and call it.
        channel_id = backend.admin_ok("GET", "/admin/channels?page=1&page_size=10")["items"][0]["id"]
        backend.admin_ok(
            "PUT",
            f"/admin/channels/{channel_id}",
            {"base_url": f"{mock.base_url}/v1"},
        )
        backend.create_model(channel_id, "mock-source", "mock-model")
        _, api_key = backend.create_user("legacy-user")

        status, body = backend.api(
            "POST",
            "/v1/chat/completions",
            json_body={"model": "mock-model", "messages": []},
            api_key=api_key,
        )
        assert status == 200, f"legacy deployment must serve requests after upgrade: {status} {body}"
        assert body["choices"][0]["message"]["content"] == "pong", body

        # New columns exist and the legacy rows are attributed lazily.
        assert backend.sql("SELECT COUNT(*) FROM request_logs")[0][0] == 1
        assert backend.sql("PRAGMA journal_mode")[0][0].lower() == "wal"
    finally:
        backend.cleanup()
        mock.stop()
