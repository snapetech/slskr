#!/usr/bin/env python3
"""Check deterministic source provenance for duplicate frozen route entries."""

from __future__ import annotations

import json
import subprocess
import tempfile
from pathlib import Path

SCRIPT = Path(__file__).resolve().parent / 'audit-slskdn-controller-routes.mjs'


def main() -> None:
    with tempfile.TemporaryDirectory(prefix='slskr-controller-inventory-order-') as directory:
        root = Path(directory)
        controllers = root / 'src/slskd'
        controllers.mkdir(parents=True)
        # Create in reverse lexical order. Both declarations expose the same
        # route, so discovery order must not select random provenance.
        for name in ['ZController.cs', 'AController.cs']:
            (controllers / name).write_text('''
[Route("api/shared")]
public class ExampleController {
    [HttpGet]
    public string Get() => "ok";
}
''')
        outputs = []
        for _ in range(3):
            result = subprocess.run(
                ['node', str(SCRIPT), '--slskdn-root', str(root), '--json'],
                check=True, capture_output=True, text=True, timeout=10,
            )
            report = json.loads(result.stdout)
            rows = report
            assert len(rows) == 1, rows
            assert rows[0]['controller'].replace('\\', '/') == 'src/slskd/ZController.cs', rows
            outputs.append(report)
        assert outputs[0] == outputs[1] == outputs[2]
    print('deterministic duplicate-route source inventory passed')


if __name__ == '__main__':
    main()
