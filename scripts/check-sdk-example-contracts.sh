#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

require_text() {
  local text="$1"
  local file="$2"
  if ! rg -n -F -- "$text" "$repo_root/$file" >/dev/null; then
    printf 'SDK example contract check failed: %s missing from %s\n' "$text" "$file" >&2
    exit 1
  fi
}

python3 - "$repo_root" <<'PY'
import asyncio
import contextlib
import importlib.util
import io
import sys
import types
from pathlib import Path

repo_root = Path(sys.argv[1])

class _ApiError(Exception):
    pass

slskr_stub = types.ModuleType("slskr")
slskr_stub.SlskrClient = object
slskr_stub.ApiError = _ApiError
slskr_stub.NetworkError = _ApiError
slskr_stub.TimeoutError = _ApiError
sys.modules["slskr"] = slskr_stub

def load_example(name: str):
    path = repo_root / "client-python" / "examples" / name
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module

advanced = load_example("advanced_usage.py")
websocket_events = load_example("websocket_events.py")

class FakeClient:
    def __init__(self):
        self.offsets = []

    async def get_search_details(self, search_id, *, limit, offset):
        self.offsets.append(offset)
        pages = {
            0: [{"id": "first"}, {"id": "second"}],
            2: [{"id": "third"}],
        }
        return {"results": pages[offset]}

async def smoke():
    client = FakeClient()
    results = await advanced.fetch_all_results(client, "search-1", batch_size=2)
    assert client.offsets == [0, 2], client.offsets
    assert [item["id"] for item in results] == ["first", "second", "third"]

    output = io.StringIO()
    with contextlib.redirect_stdout(output):
        await websocket_events.handle_message(
            {"data": {"username": "alice", "body": "hello"}}
        )
        await websocket_events.handle_transfer_update(
            {"data": {"id": "transfer-1", "status": "active", "progress_percent": 42}}
        )
    assert "hello" in output.getvalue()
    assert "42%" in output.getvalue()

asyncio.run(smoke())
PY

require_text 'GetSearchDetails(ctx, searchID, limit, offset)' client-go/examples/advanced_usage.go
require_text 'progress_percent' client-go/examples/integration_example.go
require_text 'progress_percent' client-go/examples/websocket_events.go
require_text 'body' client-go/examples/basic_usage.go
require_text 'body' client-ts/examples/basic-usage.ts
require_text 'data: {topics:' docs/http-api-features.md

if rg -n -F 'ListSearches(ctx, limit, offset)' \
  "$repo_root/client-go/examples/advanced_usage.go" >/dev/null; then
  printf 'SDK example contract check failed: Go pagination still lists search records\n' >&2
  exit 1
fi
if rg -n -F 'm["content"]' "$repo_root/client-go/examples" >/dev/null; then
  printf 'SDK example contract check failed: Go examples still read content\n' >&2
  exit 1
fi
if rg -n -F "msg.get('content')" "$repo_root/client-python/examples" >/dev/null; then
  printf 'SDK example contract check failed: Python examples still read content\n' >&2
  exit 1
fi
if rg -n -F 't["progress"]' "$repo_root/client-go/examples" >/dev/null; then
  printf 'SDK example contract check failed: Go examples still read progress\n' >&2
  exit 1
fi

printf 'SDK example contract checks passed\n'
