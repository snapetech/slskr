#!/usr/bin/env python3
"""Check bounded process/socket observation and strict shutdown evidence."""

import importlib.util
from pathlib import Path
import ssl
import tempfile
import unittest
from unittest.mock import Mock, patch


SPEC = importlib.util.spec_from_file_location("shared_tcp_proof", Path(__file__).with_name("run-shared-tcp-shutdown-proof.py"))
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class SocketEvidence(unittest.TestCase):
    def setUp(self):
        work = tempfile.TemporaryDirectory(prefix="slskr-proc-fixture-")
        self.addCleanup(work.cleanup)
        self.root = Path(work.name)
        self.proc = self.root / "123"
        (self.proc / "fd").mkdir(parents=True)
        (self.proc / "net").mkdir()
        for name in ("7", "8", "9"):
            (self.proc / "fd" / name).touch()
        self.targets = {"7": "socket:[42]", "8": "socket:[43]", "9": "/fixture/log"}

    def table(self, protocol, rows):
        # inode is the tenth field of the kernel TCP/UDP table.
        (self.proc / "net" / protocol).write_text("header\n" + "\n".join(rows) + "\n")

    def observe(self):
        with patch.object(MODULE.os, "readlink", side_effect=lambda path: self.targets[path.name]):
            return MODULE.owned_socket_ports(123, self.root)

    def test_observation_uses_own_descriptors_and_only_tcp_listeners(self):
        self.table("tcp", [
            "0: 0100007F:03E8 00000000:0000 0A 0 0 0 0 0 42",
            "1: 0100007F:03E9 00000000:0000 01 0 0 0 0 0 42",
            "2: 0100007F:FDE8 00000000:0000 0A 0 0 0 0 0 999",
        ])
        self.table("udp", ["0: 0100007F:03E8 00000000:0000 07 0 0 0 0 0 43"])
        self.assertEqual(self.observe(), {"tcp": [1000], "udp": [1000]})

    def test_ipv6_dedicated_listener_is_not_hidden(self):
        self.table("tcp6", ["0: 00000000000000000000000000000000:07D0 0:0 0A 0 0 0 0 0 42"])
        self.assertEqual(self.observe()["tcp"], [2000])

    def test_disappearing_descriptor_is_tolerated(self):
        with patch.object(MODULE.os, "readlink", side_effect=FileNotFoundError):
            self.assertEqual(MODULE.owned_socket_ports(123, self.root), {"tcp": [], "udp": []})

    def test_descriptor_budget_is_bounded(self):
        with patch.object(Path, "iterdir", return_value=iter([Path("7")] * 4097)), \
                patch.object(MODULE.os, "readlink", return_value="/fixture/log"):
            with self.assertRaisesRegex(RuntimeError, "descriptor budget"):
                MODULE.owned_socket_ports(123, self.root)

    def test_eof_and_reset_prove_closure(self):
        for result in (b"", ConnectionResetError(), ConnectionAbortedError(), ssl.SSLEOFError()):
            client = Mock()
            if isinstance(result, Exception):
                client.recv.side_effect = result
            else:
                client.recv.return_value = result
            MODULE.require_closed(client)
            client.settimeout.assert_called_once_with(3)

    def test_data_and_timeout_cannot_be_reported_as_closed(self):
        client = Mock()
        client.recv.return_value = b"x"
        with self.assertRaisesRegex(RuntimeError, "received data"):
            MODULE.require_closed(client)
        client.recv.side_effect = TimeoutError()
        with self.assertRaises(TimeoutError):
            MODULE.require_closed(client)


if __name__ == "__main__":
    unittest.main()
