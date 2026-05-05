"""
Batch operations client
"""

from copy import deepcopy
from typing import List, Dict, Any


class BatchOperation:
    """Single batch operation"""

    def __init__(self, op_id: str, method: str, path: str, body: Dict = None):
        self.id = op_id
        self.method = method
        self.path = path
        self.body = deepcopy(body) if body is not None else None

    def to_dict(self) -> Dict:
        """Convert to dictionary"""
        op = {"id": self.id, "method": self.method, "path": self.path}
        if self.body:
            op["body"] = deepcopy(self.body)
        return op


class BatchResult:
    """Single batch operation result"""

    def __init__(self, op_id: str, status: int, body: Any):
        self.id = op_id
        self.status = status
        self.body = deepcopy(body)

    def is_success(self) -> bool:
        """Check if operation succeeded"""
        return 200 <= self.status < 300

    def is_error(self) -> bool:
        """Check if operation failed"""
        return self.status >= 400


class BatchResponse:
    """Batch operation response"""

    def __init__(self, results: List[BatchResult], total_time_ms: int):
        self.results = list(results)
        self.total_time_ms = total_time_ms

    def all_successful(self) -> bool:
        """Check if all operations succeeded"""
        return all(r.is_success() for r in self.results)

    def get_successful(self) -> List[BatchResult]:
        """Get successful operations"""
        return [r for r in self.results if r.is_success()]

    def get_failed(self) -> List[BatchResult]:
        """Get failed operations"""
        return [r for r in self.results if r.is_error()]


class BatchBuilder:
    """Builder for batch operations"""

    def __init__(self, client):
        self.client = client
        self.operations: List[BatchOperation] = []
        self.op_counter = 0

    def get(self, path: str, op_id: str = None) -> "BatchBuilder":
        """Add GET operation"""
        op_id = op_id or f"op-{self.op_counter}"
        self.operations.append(BatchOperation(op_id, "GET", path))
        self.op_counter += 1
        return self

    def post(self, path: str, body: Dict, op_id: str = None) -> "BatchBuilder":
        """Add POST operation"""
        op_id = op_id or f"op-{self.op_counter}"
        self.operations.append(BatchOperation(op_id, "POST", path, body))
        self.op_counter += 1
        return self

    def put(self, path: str, body: Dict, op_id: str = None) -> "BatchBuilder":
        """Add PUT operation"""
        op_id = op_id or f"op-{self.op_counter}"
        self.operations.append(BatchOperation(op_id, "PUT", path, body))
        self.op_counter += 1
        return self

    def delete(self, path: str, op_id: str = None) -> "BatchBuilder":
        """Add DELETE operation"""
        op_id = op_id or f"op-{self.op_counter}"
        self.operations.append(BatchOperation(op_id, "DELETE", path))
        self.op_counter += 1
        return self

    def add_operations(self, ops: List[BatchOperation]) -> "BatchBuilder":
        """Add multiple operations"""
        self.operations.extend(ops)
        return self

    def get_operations(self) -> List[BatchOperation]:
        """Get current operations"""
        return self.operations[:]

    def clear(self) -> "BatchBuilder":
        """Clear all operations"""
        self.operations.clear()
        self.op_counter = 0
        return self

    def size(self) -> int:
        """Get operation count"""
        return len(self.operations)

    async def execute(self) -> BatchResponse:
        """Execute batch operations"""
        if not self.operations:
            raise ValueError("Batch is empty")

        if len(self.operations) > 100:
            raise ValueError("Batch cannot exceed 100 operations")

        request = {"operations": [op.to_dict() for op in self.operations]}

        response = await self.client._post("/api/batch", request)

        results = [
            BatchResult(r["id"], r["status"], r["body"]) for r in response.get("results", [])
        ]

        return BatchResponse(results, response.get("total_time_ms", 0))


class BatchClient:
    """Batch operations client"""

    def __init__(self, client):
        self.client = client

    def builder(self) -> BatchBuilder:
        """Create batch builder"""
        return BatchBuilder(self.client)
