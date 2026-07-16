import asyncio
import inspect
import json
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from slskr import BatchClient, BatchBuilder, SlskrClient, WebSocketClient
from slskr.batch import BatchOperation, BatchResponse, BatchResult
from slskr.exceptions import ApiError, NetworkError


def test_client_url_and_path_segments_are_safe():
    client = SlskrClient("http://localhost:8080/", "token")

    assert client.base_url == "http://localhost:8080"
    assert client._build_url("api/health") == "http://localhost:8080/api/health"
    assert client._path_segment("../peer name/track") == "..%2Fpeer%20name%2Ftrack"


def test_client_validates_and_normalizes_rest_base_url():
    with pytest.raises(ValueError, match="absolute HTTP or HTTPS"):
        SlskrClient("ftp://example.test", "token")
    with pytest.raises(ValueError, match="without credentials"):
        SlskrClient("https://user:password@example.test", "token")

    client = SlskrClient("https://example.test/slskr/?debug=true#fragment", "token")
    assert client.base_url == "https://example.test/slskr"


def test_batch_builder_serializes_and_limits_operations():
    client = SlskrClient("http://localhost:8080", "token")
    builder = BatchBuilder(client)

    builder.get("/api/health").post("/api/searches", {"query": "ambient"})

    operations = builder.get_operations()
    assert [operation.to_dict()["method"] for operation in operations] == ["GET", "POST"]
    assert operations[1].to_dict()["body"] == {"query": "ambient"}
    assert builder.size() == 2


def test_batch_objects_copy_mutable_inputs():
    body = {"query": "ambient", "filters": ["lossless"]}
    operation = BatchOperation("op", "POST", "/api/searches", body)
    body["filters"].append("mutated")

    serialized = operation.to_dict()
    serialized["body"]["filters"].append("serialized")

    assert operation.to_dict()["body"] == {"query": "ambient", "filters": ["lossless"]}

    results = [BatchResult("ok", 200, {"items": ["one"]})]
    response = BatchResponse(results, 5)
    results.append(BatchResult("late", 200, {}))
    assert [result.id for result in response.results] == ["ok"]


def test_batch_response_helpers_classify_results():
    response = BatchResponse(
        [
            BatchResult("ok", 200, {"value": True}),
            BatchResult("bad", 404, {"error": "missing"}),
        ],
        12,
    )

    assert not response.all_successful()
    assert [result.id for result in response.get_successful()] == ["ok"]
    assert [result.id for result in response.get_failed()] == ["bad"]


def test_websocket_client_uses_event_endpoint_and_tracks_topics():
    client = WebSocketClient("https://example.test/base/", "token")

    assert client.url == "wss://example.test/base/api/events/ws"
    client.subscribe("transfers", "searches", "transfers")
    assert sorted(client.get_subscribed_topics()) == ["searches", "transfers"]
    client.unsubscribe("searches")
    assert client.get_subscribed_topics() == ["transfers"]


def test_websocket_client_rejects_non_http_base_urls():
    with pytest.raises(ValueError, match="absolute HTTP or HTTPS"):
        WebSocketClient("ftp://example.test", "token")

    with pytest.raises(ValueError, match="absolute HTTP or HTTPS"):
        WebSocketClient("example.test", "token")

    with pytest.raises(ValueError, match="without credentials"):
        WebSocketClient("https://user:password@example.test", "token")


@pytest.mark.asyncio
async def test_websocket_client_cleans_up_when_connect_is_cancelled():
    dial_started = asyncio.Event()

    async def blocked_connect(*_args, **_kwargs):
        dial_started.set()
        await asyncio.Event().wait()

    session = MagicMock()
    session.ws_connect = AsyncMock(side_effect=blocked_connect)
    session.close = AsyncMock()

    with patch("slskr.websocket.aiohttp.ClientSession", return_value=session):
        client = WebSocketClient("https://example.test", "token")
        connecting = asyncio.create_task(client.connect())
        await dial_started.wait()
        connecting.cancel()

        with pytest.raises(asyncio.CancelledError):
            await connecting

    session.close.assert_awaited_once()
    assert client.session is None
    assert client.ws is None
    assert not client.is_connected()


@pytest.mark.asyncio
async def test_websocket_client_restores_subscriptions_on_connect():
    ws = MagicMock()
    ws.closed = False
    ws.send_json = AsyncMock()
    ws.close = AsyncMock()
    ws.__aiter__.return_value = iter(())
    session = MagicMock()
    session.ws_connect = AsyncMock(return_value=ws)
    session.close = AsyncMock()

    with patch("slskr.websocket.aiohttp.ClientSession", return_value=session):
        client = WebSocketClient("https://example.test", "token")
        client.subscribe("transfers", "searches")
        await client.connect()

        ws.send_json.assert_awaited_once_with(
            {
                "type": "subscribe",
                "data": {"topics": ["searches", "transfers"]},
            }
        )
        await client.disconnect()


@pytest.mark.asyncio
async def test_websocket_client_rejects_duplicate_connect_and_bounds_messages():
    ws = MagicMock()
    ws.closed = False
    ws.close = AsyncMock()
    ws.__aiter__.return_value = iter(())
    session = MagicMock()
    session.ws_connect = AsyncMock(return_value=ws)
    session.close = AsyncMock()

    with patch("slskr.websocket.aiohttp.ClientSession", return_value=session):
        client = WebSocketClient("https://example.test", "token")
        await client.connect()
        with pytest.raises(RuntimeError, match="already connected"):
            await client.connect()

        assert session.ws_connect.await_args.kwargs["max_msg_size"] == 64 * 1024
        await client.disconnect()

    ws.close.assert_awaited_once()
    session.close.assert_awaited_once()


@pytest.mark.asyncio
async def test_websocket_client_cleans_up_after_remote_close():
    ws = MagicMock()
    ws.closed = False
    ws.close = AsyncMock()
    ws.__aiter__.return_value = iter(())
    session = MagicMock()
    session.ws_connect = AsyncMock(return_value=ws)
    session.close = AsyncMock()
    connection_changes = []

    with patch("slskr.websocket.aiohttp.ClientSession", return_value=session):
        client = WebSocketClient("https://example.test", "token")
        client.on_connection_change(connection_changes.append)
        await client.connect()
        await client._message_task

    assert client.ws is None
    assert client.session is None
    assert connection_changes == [True, False]
    session.close.assert_awaited_once()


@pytest.mark.asyncio
async def test_websocket_subscription_tasks_report_errors_and_deduplicate_transitions():
    ws = MagicMock()
    ws.closed = False
    ws.send_json = AsyncMock(side_effect=[None, OSError("send failed")])
    client = WebSocketClient("https://example.test", "token")
    client.ws = ws
    errors = []
    client.on_error(errors.append)

    client.subscribe("searches", "searches")
    await asyncio.sleep(0)
    client.unsubscribe("missing", "searches", "searches")
    for _ in range(5):
        await asyncio.sleep(0)

    assert ws.send_json.await_count == 2
    assert ws.send_json.await_args_list[0].args[0]["data"]["topics"] == ["searches"]
    assert ws.send_json.await_args_list[1].args[0]["data"]["topics"] == ["searches"]
    assert len(errors) == 1
    assert str(errors[0]) == "send failed"
    assert client.get_subscribed_topics() == ["searches"]
    assert not client._outbound_tasks


@pytest.mark.asyncio
async def test_websocket_failed_subscribe_task_rolls_back_topics():
    ws = MagicMock()
    ws.closed = False
    ws.send_json = AsyncMock(side_effect=OSError("send failed"))
    client = WebSocketClient("https://example.test", "token")
    client.ws = ws
    errors = []
    client.on_error(errors.append)

    client.subscribe("searches")
    for _ in range(5):
        await asyncio.sleep(0)

    assert client.get_subscribed_topics() == []
    assert [str(error) for error in errors] == ["send failed"]


@pytest.mark.asyncio
async def test_websocket_disconnect_cancels_pending_subscription_writes():
    send_started = asyncio.Event()

    async def blocked_send(_message):
        send_started.set()
        await asyncio.Event().wait()

    ws = MagicMock()
    ws.closed = False
    ws.send_json = AsyncMock(side_effect=blocked_send)
    ws.close = AsyncMock()
    session = MagicMock()
    session.close = AsyncMock()
    client = WebSocketClient("https://example.test", "token")
    client.ws = ws
    client.session = session

    client.subscribe("searches")
    await send_started.wait()
    await client.disconnect()

    assert not client._outbound_tasks
    ws.close.assert_awaited_once()
    session.close.assert_awaited_once()


@pytest.mark.asyncio
async def test_websocket_dispatches_sync_and_async_listeners_without_false_errors():
    client = WebSocketClient("https://example.test", "token")
    received = []
    errors = []

    def sync_listener(event):
        received.append(("sync", event["data"]))

    async def async_listener(event):
        received.append(("async", event["data"]))

    client.on("search.completed", sync_listener)
    client.on("search.completed", async_listener)
    client.on_error(errors.append)

    await client._process_message(
        json.dumps({"type": "search.completed", "data": {"id": "search-1"}})
    )
    await asyncio.sleep(0)

    assert sorted(received) == [
        ("async", {"id": "search-1"}),
        ("sync", {"id": "search-1"}),
    ]
    assert errors == []


def test_api_error_helpers():
    not_found = ApiError(404, "not_found")
    server_error = ApiError(503, "unavailable")

    assert not_found.is_client_error()
    assert not_found.is_not_found()
    assert server_error.is_server_error()


def test_public_exports_are_available():
    assert BatchClient is not None
    assert BatchOperation("id", "GET", "/api/health").to_dict()["id"] == "id"
    assert inspect.iscoroutinefunction(SlskrClient.close)


class FakeContent:
    def __init__(self, chunks):
        self.chunks = chunks

    async def iter_chunked(self, _size):
        for chunk in self.chunks:
            yield chunk


class FakeResponse:
    def __init__(self, chunks, content_length=None):
        self.content = FakeContent(chunks)
        self.content_length = content_length


@pytest.mark.asyncio
async def test_python_client_rejects_oversized_declared_response():
    client = SlskrClient("https://example.test", "token")
    response = FakeResponse([], content_length=8 * 1024 * 1024 + 1)

    with pytest.raises(NetworkError, match="exceeds"):
        await client._read_json(response, 8 * 1024 * 1024)


@pytest.mark.asyncio
async def test_python_client_bounds_chunked_response():
    client = SlskrClient("https://example.test", "token")
    response = FakeResponse([b"x" * 65, b"y" * 64])

    with pytest.raises(NetworkError, match="exceeds"):
        await client._read_json(response, 128)


@pytest.mark.asyncio
async def test_python_client_rejects_trailing_json():
    client = SlskrClient("https://example.test", "token")
    response = FakeResponse([b'{"status":"ok"} {"unexpected":true}'])

    with pytest.raises(json.JSONDecodeError):
        await client._read_json(response, 1024)


@pytest.mark.asyncio
async def test_python_client_does_not_retry_oversized_response():
    response = MagicMock(status=200)
    context = MagicMock()
    context.__aenter__ = AsyncMock(return_value=response)
    context.__aexit__ = AsyncMock(return_value=False)
    session = MagicMock()
    session.request.return_value = context
    client = SlskrClient("https://example.test", "token", retries=3)
    client.session = session
    client._read_json = AsyncMock(side_effect=NetworkError("response too large"))

    with pytest.raises(NetworkError, match="too large"):
        await client._request("GET", "/api/health")

    session.request.assert_called_once()


@pytest.mark.asyncio
async def test_python_client_does_not_replay_mutations_after_transport_failure():
    context = MagicMock()
    context.__aenter__ = AsyncMock(side_effect=OSError("response lost"))
    context.__aexit__ = AsyncMock(return_value=False)
    session = MagicMock()
    session.request.return_value = context
    client = SlskrClient("https://example.test", "token", retries=3, retry_delay=0)
    client.session = session

    with pytest.raises(NetworkError):
        await client._request("POST", "/api/searches", body={"query": "rare"})

    session.request.assert_called_once()


@pytest.mark.asyncio
async def test_python_client_retains_retries_for_reads():
    failed = MagicMock()
    failed.__aenter__ = AsyncMock(side_effect=OSError("network down"))
    failed.__aexit__ = AsyncMock(return_value=False)
    response = MagicMock(status=204)
    succeeded = MagicMock()
    succeeded.__aenter__ = AsyncMock(return_value=response)
    succeeded.__aexit__ = AsyncMock(return_value=False)
    session = MagicMock()
    session.request.side_effect = [failed, succeeded]
    client = SlskrClient("https://example.test", "token", retries=1, retry_delay=0)
    client.session = session

    assert await client._request("GET", "/api/health") is None
    assert session.request.call_count == 2
