#!/usr/bin/env python3
"""Integration test runner.

Usage:
    python tests/run_tests.py            # run every test
    python tests/run_tests.py p0         # run test modules matching a substring

Each test receives a fresh `Harness` (backend + mock upstream) and must raise
AssertionError on failure. Exit code is non-zero when anything fails.
"""

from __future__ import annotations

import importlib
import inspect
import pathlib
import sys
import time
import traceback

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

from support import Harness  # noqa: E402

TEST_MODULES = ["test_p0", "test_p1", "test_regression", "test_upgrade"]


def discover(selector: str | None):
    tests = []
    for module_name in TEST_MODULES:
        module = importlib.import_module(module_name)
        for name, func in inspect.getmembers(module, inspect.isfunction):
            if not name.startswith("test_"):
                continue
            display = f"{module_name}.{name}"
            if selector and selector not in display:
                continue
            tests.append((display, func))
    return tests


def main() -> int:
    selector = sys.argv[1] if len(sys.argv) > 1 else None
    tests = discover(selector)
    if not tests:
        print(f"no tests matched {selector!r}")
        return 1

    passed, failed = 0, []
    started = time.time()

    for display_name, func in tests:
        print(f"\n=== {display_name} ===", flush=True)
        case_started = time.time()
        try:
            if getattr(func, "custom_harness", False):
                func()
            else:
                with Harness() as harness:
                    func(harness)
            elapsed = time.time() - case_started
            passed += 1
            print(f"PASS {display_name} ({elapsed:.1f}s)", flush=True)
        except Exception as error:  # noqa: BLE001 - report every failure
            elapsed = time.time() - case_started
            failed.append((display_name, error))
            print(f"FAIL {display_name} ({elapsed:.1f}s): {error}", flush=True)
            traceback.print_exc()

    total_time = time.time() - started
    print("\n" + "=" * 70)
    print(f"{passed} passed, {len(failed)} failed in {total_time:.1f}s")
    for name, error in failed:
        print(f"  FAILED {name}: {error}")
    return 0 if not failed else 1


if __name__ == "__main__":
    raise SystemExit(main())
