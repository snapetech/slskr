import asyncio
import inspect
import json
from unittest.mock import AsyncMock, MagicMock, call, patch

import pytest

from slskr import BatchClient, BatchBuilder, SlskrClient, WebSocketClient
from slskr.batch import BatchOperation, BatchResponse, BatchResult
from slskr.exceptions import ApiError, NetworkError, ResponseContractError


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


@pytest.mark.asyncio
async def test_python_client_uses_daemon_wire_contracts():
    client = SlskrClient("https://example.test", "token")
    client._get = AsyncMock(
        side_effect=[
            [{"id": "search-1"}],
            {"id": "search-1", "results": []},
            {"entries": [{"id": 1}]},
            {"entries": [{"id": 2}]},
            {"entries": [{"id": 3}]},
        ]
    )
    client._post = AsyncMock(side_effect=[{"id": 4}, {"id": 5}, None])
    client._put = AsyncMock(return_value=None)

    assert await client.list_searches() == [{"id": "search-1"}]
    assert await client.get_search_details("search-1", limit=10, offset=2) == {
        "id": "search-1",
        "results": [],
    }
    assert await client.list_messages() == [{"id": 1}]
    assert await client.get_user_messages("alice", offset=2) == [{"id": 2}]
    assert await client.list_transfers(direction="download") == [{"id": 3}]
    assert await client.create_transfer("download", "alice", "track.flac") == {"id": 4}
    assert await client.send_message("alice", "hello") == {"id": 5}
    await client.acknowledge_message("7")

    assert client._get.await_args_list == [
        call("/api/searches", params={"limit": 50, "offset": 0}),
        call("/api/searches/search-1", params={"limit": 10, "offset": 2}),
        call("/api/messages", params={"limit": 50, "offset": 0}),
        call("/api/messages/alice", params={"limit": 50, "offset": 2}),
        call(
            "/api/transfers",
            params={"limit": 50, "offset": 0, "direction": 0},
        ),
    ]
    assert client._post.await_args_list == [
        call(
            "/api/transfers",
            {"direction": 0, "peer_username": "alice", "filename": "track.flac"},
        ),
        call("/api/messages", {"username": "alice", "body": "hello"}),
        call("/api/messages/7/ack", {}),
    ]
    client._post.assert_any_await("/api/messages/7/ack", {})


@pytest.mark.asyncio
async def test_python_client_covers_session_and_extended_api_routes():
    client = SlskrClient("https://example.test", "token")
    get_calls = []
    post_calls = []
    put_calls = []
    delete_calls = []

    async def fake_get(path, params=None, authenticated=True):
        get_calls.append((path, params, authenticated))
        responses = {
            "/api/session": {
                "state": "connected",
                "username": "alice",
                "privileges_seconds": 120,
            },
            "/api/users": {"entries": [{"username": "bob"}]},
            "/api/users/bob/info": {"username": "bob", "status": "online"},
            "/api/rooms": {"rooms": [{"name": "lounge"}]},
            "/api/rooms/lounge%20room": {"name": "lounge room"},
            "/api/users/bob/browse": {"entries": [{"filename": "track.flac"}]},
            "/api/browse/requests": {"requests": [{"username": "bob"}]},
            "/api/events": {"events": [{"type": "search.completed"}]},
            "/api/shares": {"local": [{"filename": "track.flac"}]},
            "/api/config/download-filter": {"enabled": True},
            "/api/mediacore/retrieve/stats": {"cacheHits": 1},
        }
        return responses.get(path, {})

    async def fake_post(path, body, authenticated=True):
        post_calls.append((path, body, authenticated))
        return {"accepted": True} if path.startswith("/api/session/") else {}

    async def fake_put(path, body, authenticated=True):
        put_calls.append((path, body, authenticated))
        return {"enabled": body.get("enabled", True)}

    async def fake_delete(path, authenticated=True):
        delete_calls.append((path, authenticated))

    client._get = AsyncMock(side_effect=fake_get)
    client._post = AsyncMock(side_effect=fake_post)
    client._put = AsyncMock(side_effect=fake_put)
    client._delete = AsyncMock(side_effect=fake_delete)

    assert (await client.get_sessions())[0]["username"] == "alice"
    assert (await client.create_session())["state"] == "connected"
    assert (await client.create_session(parameters={"username": "alice"}))["state"] == "connected"
    assert await client.ping_session() == {"accepted": True}
    await client.disconnect_session()
    assert await client.get_session_privileges() == {
        "user_id": "alice",
        "privileges": ["privileged"],
    }
    with pytest.raises(ValueError, match="Unsupported session type"):
        await client.create_session("peer")

    assert await client.list_users() == [{"username": "bob"}]
    assert await client.get_user("bob") == {"username": "bob", "status": "online"}
    assert await client.list_rooms() == [{"name": "lounge"}]
    assert await client.get_room("lounge room") == {"name": "lounge room"}
    assert await client.join_room("lounge room") == {}
    await client.leave_room("lounge room")

    assert await client.browse_user("bob", folder="Albums") == {
        "entries": [{"filename": "track.flac"}]
    }
    assert await client.request_browse("bob") == {}
    assert await client.request_browse("bob", folder="Albums") == {}
    assert await client.get_browse_requests(status="pending") == [{"username": "bob"}]
    assert await client.respond_to_browse_request("bob", "reject") == {}
    assert await client.respond_to_browse_request("bob", "accept", folder="Albums") == {}
    with pytest.raises(ValueError, match="accept.*reject"):
        await client.respond_to_browse_request("bob", "ignore")

    assert await client.get_events(event_type="search.completed") == [
        {"type": "search.completed"}
    ]
    assert await client.get_events(
        event_type="search.completed",
        topic="searches",
        query="ambient & live",
    ) == [{"type": "search.completed"}]
    assert await client.list_shares() == [{"filename": "track.flac"}]
    assert await client.refresh_shares() == {}
    assert await client.get_filters() == {"enabled": True}
    assert await client.update_filters({"enabled": False}) == {"enabled": False}
    assert await client.get_cache_stats() == {"cacheHits": 1}
    assert await client.invalidate_cache(["content:track"]) == {}

    assert ("/api/rooms/lounge%20room", None, True) in get_calls
    assert ("/api/users/bob/browse", {"limit": 50, "offset": 0, "folder": "Albums"}, True) in get_calls
    assert ("/api/events", {"limit": 50, "offset": 0, "kind": "search.completed"}, True) in get_calls
    assert (
        "/api/events",
        {
            "limit": 50,
            "offset": 0,
            "kind": "search.completed",
            "topic": "searches",
            "q": "ambient & live",
        },
        True,
    ) in get_calls
    assert ("/api/users/bob/browse/cancel", {"reason": "rejected by client"}, True) in post_calls
    assert ("/api/users/bob/browse/request", {}, True) in post_calls
    assert ("/api/users/bob/browse/folder", {"folder": "Albums"}, True) in post_calls
    assert ("/api/mediacore/retrieve/cache/clear", {"keys": ["content:track"]}, True) in post_calls
    assert ("/api/rooms/lounge%20room/join", True) in delete_calls


@pytest.mark.asyncio
async def test_create_search_exposes_compatibility_search_id_as_id():
    client = SlskrClient("https://example.test", "token")
    client._post = AsyncMock(
        return_value={"searchId": "search-123", "query": "ambient", "results": []}
    )

    result = await client.create_search("ambient")

    assert result["id"] == "search-123"
    assert result["searchId"] == "search-123"


@pytest.mark.asyncio
async def test_python_client_rejects_malformed_success_response_contracts():
    client = SlskrClient("https://example.test", "token")

    client._get = AsyncMock(return_value=[])
    with pytest.raises(ResponseContractError, match="invalid health response"):
        await client.health()

    client._get = AsyncMock(return_value={})
    with pytest.raises(ResponseContractError, match="invalid users response"):
        await client.list_users()

    client._get = AsyncMock(return_value=None)
    with pytest.raises(ResponseContractError, match="invalid session response"):
        await client.get_sessions()

    client._post = AsyncMock(return_value={"query": "ambient"})
    with pytest.raises(ResponseContractError, match="invalid search response"):
        await client.create_search("ambient")

    client._get = AsyncMock(return_value={"entries": [None]})
    with pytest.raises(ResponseContractError, match="invalid events response"):
        await client.get_events()


@pytest.mark.asyncio
async def test_python_batch_client_rejects_malformed_success_response():
    client = SlskrClient("https://example.test", "token")
    client._post = AsyncMock(return_value={"results": [{"id": "op-1", "status": 200}]})

    builder = BatchBuilder(client).get("/api/health", op_id="op-1")
    with pytest.raises(ResponseContractError, match="invalid batch response"):
        await builder.execute()


@pytest.mark.asyncio
@pytest.mark.parametrize(
    "total_time_ms",
    [None, -1, 1.5, "4"],
    ids=["missing", "negative", "fractional", "text"],
)
async def test_python_batch_client_rejects_invalid_total_time(total_time_ms):
    client = SlskrClient("https://example.test", "token")
    response = {"results": [{"id": "op-1", "status": 200, "body": None}]}
    if total_time_ms is not None:
        response["total_time_ms"] = total_time_ms
    client._post = AsyncMock(return_value=response)

    builder = BatchBuilder(client).get("/api/health", op_id="op-1")
    with pytest.raises(ResponseContractError, match="invalid batch response"):
        await builder.execute()


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


def test_batch_helpers_treat_redirects_as_failures_and_copy_operations():
    response = BatchResponse([BatchResult("redirect", 302, {})], 1)
    assert not response.all_successful()
    assert [result.id for result in response.get_failed()] == ["redirect"]

    body = {"options": {"filters": ["lossless"]}}
    builder = BatchBuilder(SlskrClient("https://example.test", "token"))
    operation = BatchOperation("op", "POST", "/api/searches", body)
    builder.add_operations([operation])
    body["options"]["filters"].append("mutated")
    snapshot = builder.get_operations()
    snapshot[0].body["options"]["filters"].append("snapshot")
    assert builder.get_operations()[0].to_dict()["body"] == {
        "options": {"filters": ["lossless"]}
    }


def test_batch_builder_avoids_generated_id_collisions():
    builder = BatchBuilder(SlskrClient("https://example.test", "token"))
    builder.add_operations([BatchOperation("op-0", "GET", "/api/health")])
    builder.get("/api/version")

    assert [operation.id for operation in builder.get_operations()] == ["op-0", "op-1"]


@pytest.mark.asyncio
async def test_batch_builder_rejects_duplicate_ids_before_sending():
    client = SlskrClient("https://example.test", "token")
    client._post = AsyncMock()
    builder = BatchBuilder(client)
    builder.add_operations(
        [
            BatchOperation("same", "GET", "/api/health"),
            BatchOperation("same", "GET", "/api/version"),
        ]
    )

    with pytest.raises(ValueError, match="duplicate operation ID"):
        await builder.execute()
    client._post.assert_not_awaited()


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
async def test_websocket_client_bounds_a_stalled_handshake():
    dial_started = asyncio.Event()

    async def blocked_connect(*_args, **_kwargs):
        dial_started.set()
        await asyncio.Event().wait()

    session = MagicMock()
    session.ws_connect = AsyncMock(side_effect=blocked_connect)
    session.close = AsyncMock()

    with patch("slskr.websocket.aiohttp.ClientSession", return_value=session):
        client = WebSocketClient(
            "https://example.test", "token", connect_timeout=0.01
        )
        connecting = asyncio.create_task(client.connect())
        await dial_started.wait()

        with pytest.raises(asyncio.TimeoutError):
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
        client.max_reconnect_attempts = 0
        client.on_connection_change(connection_changes.append)
        await client.connect()
        await client._message_task

    assert client.ws is None
    assert client.session is None
    assert connection_changes == [True, False]
    session.close.assert_awaited_once()


@pytest.mark.asyncio
async def test_websocket_client_reconnects_after_unexpected_close():
    async def closed_messages():
        if False:
            yield None

    ws = MagicMock()
    ws.closed = False
    ws.close = AsyncMock()
    ws.__aiter__.side_effect = lambda: closed_messages()
    session = MagicMock()
    session.close = AsyncMock()
    client = WebSocketClient("https://example.test", "token")
    client.max_reconnect_attempts = 1
    client.reconnect_delay = 0
    reconnect = AsyncMock()
    client.connect = reconnect
    client.ws = ws
    client.session = session

    await client._handle_messages(ws)
    await asyncio.sleep(0)
    await asyncio.sleep(0)

    reconnect.assert_awaited_once()
    assert client.reconnect_attempts == 1


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


@pytest.mark.asyncio
async def test_websocket_listener_cleanup_and_mutation_are_safe():
    client = WebSocketClient("https://example.test", "token")
    received = []
    unsubscribe_first = None

    def first_listener(_event):
        received.append("first")
        if unsubscribe_first is not None:
            unsubscribe_first()

    def second_listener(_event):
        received.append("second")

    unsubscribe_first = client.on("search.completed", first_listener)
    client.on("search.completed", second_listener)

    await client._process_message(json.dumps({"type": "search.completed"}))
    assert sorted(received) == ["first", "second"]

    client.remove_all_listeners()
    unsubscribe_first()


@pytest.mark.asyncio
async def test_websocket_connection_and_error_listener_failures_are_consumed(caplog):
    client = WebSocketClient("https://example.test", "token")
    errors = []

    def bad_connection_listener(_connected):
        raise RuntimeError("connection callback failed")

    def good_error_listener(error):
        errors.append(error)

    def bad_error_listener(_error):
        raise RuntimeError("error callback failed")

    client.on_connection_change(bad_connection_listener)
    client.on_error(good_error_listener)
    client.on_error(bad_error_listener)

    with caplog.at_level("ERROR"):
        client._notify_connection_listeners(True)
        for _ in range(5):
            await asyncio.sleep(0)

    assert [str(error) for error in errors] == ["connection callback failed"]
    assert "error callback failed" in caplog.text


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


@pytest.mark.asyncio
async def test_python_client_context_reuses_and_detaches_sessions():
    first_session = MagicMock()
    first_session.closed = False
    first_session.close = AsyncMock()
    second_session = MagicMock()
    second_session.closed = False
    second_session.close = AsyncMock()

    client = SlskrClient("https://example.test", "token")
    with patch(
        "slskr.client.aiohttp.ClientSession",
        side_effect=[first_session, second_session],
    ) as create_session:
        await client._ensure_session()
        async with client:
            assert client.session is first_session
        assert client.session is None
        await client._ensure_session()
        assert client.session is second_session

    assert create_session.call_count == 2
    first_session.close.assert_awaited_once()
    await client.close()
    second_session.close.assert_awaited_once()


@pytest.mark.asyncio
async def test_python_client_coalesces_concurrent_websocket_connects():
    connect_started = asyncio.Event()
    release_connect = asyncio.Event()
    websocket = MagicMock()
    websocket.is_connected.side_effect = [False, True]

    async def connect():
        connect_started.set()
        await release_connect.wait()

    websocket.connect = AsyncMock(side_effect=connect)
    websocket.disconnect = AsyncMock()

    client = SlskrClient("https://example.test", "token")
    with patch("slskr.client.WebSocketClient", return_value=websocket) as constructor:
        first = asyncio.create_task(client.connect_ws())
        await connect_started.wait()
        second = asyncio.create_task(client.connect_ws())
        release_connect.set()
        first_result, second_result = await asyncio.gather(first, second)

    assert first_result is websocket
    assert second_result is websocket
    constructor.assert_called_once()
    websocket.connect.assert_awaited_once()
    await client.close()
    websocket.disconnect.assert_awaited_once()


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
async def test_python_client_accepts_empty_success_body_as_no_content():
    client = SlskrClient("https://example.test", "token")
    response = FakeResponse([b"  \n"])

    assert await client._read_json(response, 8 * 1024 * 1024) is None


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
async def test_python_client_rejects_redirects_without_following_them():
    response = MagicMock(status=302)
    context = MagicMock()
    context.__aenter__ = AsyncMock(return_value=response)
    context.__aexit__ = AsyncMock(return_value=False)
    session = MagicMock()
    session.request.return_value = context
    client = SlskrClient("https://example.test", "token", retries=3)
    client.session = session

    with pytest.raises(NetworkError, match="redirect"):
        await client._request("GET", "/api/health")

    session.request.assert_called_once()
    assert session.request.call_args.kwargs["allow_redirects"] is False


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
async def test_python_client_preserves_structured_api_errors_and_non_object_bodies():
    response = MagicMock(status=422)
    context = MagicMock()
    context.__aenter__ = AsyncMock(return_value=response)
    context.__aexit__ = AsyncMock(return_value=False)
    session = MagicMock()
    session.request.return_value = context
    client = SlskrClient("https://example.test", "token", retries=3)
    client.session = session
    client._read_json = AsyncMock(
        return_value={
            "error": "validation_failed",
            "message": "Invalid query",
            "details": "query is required",
        }
    )

    with pytest.raises(ApiError) as raised:
        await client._request("GET", "/api/health")

    assert raised.value.code == "validation_failed"
    assert str(raised.value) == "Invalid query"
    assert raised.value.details == "query is required"

    client._read_json = AsyncMock(return_value=["invalid"])
    with pytest.raises(ApiError) as raised:
        await client._request("GET", "/api/health")
    assert raised.value.code == "HTTP 422"


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
