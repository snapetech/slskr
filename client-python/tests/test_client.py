import inspect

from slskr import BatchClient, BatchBuilder, SlskrClient, WebSocketClient
from slskr.batch import BatchOperation, BatchResponse, BatchResult
from slskr.exceptions import ApiError


def test_client_url_and_path_segments_are_safe():
    client = SlskrClient("http://localhost:8080/", "token")

    assert client.base_url == "http://localhost:8080"
    assert client._build_url("api/health") == "http://localhost:8080/api/health"
    assert client._path_segment("../peer name/track") == "..%2Fpeer%20name%2Ftrack"


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
