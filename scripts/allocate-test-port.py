#!/usr/bin/env python3
"""Choose a free test port outside the operating system's ephemeral range."""

from __future__ import annotations

import argparse
import random
import socket
from pathlib import Path


def candidate_range() -> tuple[int, int]:
    """Prefer unprivileged ports below Linux's ephemeral client range."""
    first_ephemeral = 32768
    try:
        values = Path("/proc/sys/net/ipv4/ip_local_port_range").read_text().split()
        first_ephemeral = int(values[0])
    except (OSError, ValueError, IndexError):
        pass

    low = 20000
    high = min(32767, first_ephemeral - 1)
    if high - low < 64:
        low = 1024
        high = first_ephemeral - 1
    if high < low:
        raise RuntimeError("no unprivileged TCP test ports are available below the ephemeral range")
    return low, high


def bind_probe(port: int, sock_type: int) -> list[socket.socket] | None:
    """Hold IPv4 and IPv6 wildcard probes simultaneously while checking a port."""
    probes: list[socket.socket] = []
    try:
        ipv4 = socket.socket(socket.AF_INET, sock_type)
        probes.append(ipv4)
        ipv4.bind(("127.0.0.1", port))

        if socket.has_ipv6:
            try:
                ipv6 = socket.socket(socket.AF_INET6, sock_type)
                probes.append(ipv6)
                if hasattr(socket, "IPV6_V6ONLY"):
                    ipv6.setsockopt(socket.IPPROTO_IPV6, socket.IPV6_V6ONLY, 1)
                ipv6.bind(("::1", port))
            except OSError:
                if probes:
                    for probe in probes:
                        probe.close()
                return None
        return probes
    except OSError:
        for probe in probes:
            probe.close()
        return None


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("registry", type=Path, help="directory used to keep allocations unique for this test run")
    parser.add_argument("--udp", action="store_true", help="allocate a UDP port")
    parser.add_argument("--adjacent", action="store_true", help="allocate two adjacent TCP ports and print the first")
    parser.add_argument("excluded", nargs="*", type=int, help="ports that the caller has reserved explicitly")
    args = parser.parse_args()
    if args.udp and args.adjacent:
        parser.error("--udp and --adjacent cannot be combined")

    args.registry.mkdir(parents=True, exist_ok=True)
    low, high = candidate_range()
    if args.adjacent:
        high -= 1
    excluded = set(args.excluded)
    chooser = random.SystemRandom()
    sock_type = socket.SOCK_DGRAM if args.udp else socket.SOCK_STREAM

    for _ in range(20_000):
        port = chooser.randint(low, high)
        ports = (port, port + 1) if args.adjacent else (port,)
        if excluded.intersection(ports):
            continue

        claims: list[Path] = []
        for candidate in ports:
            claim = args.registry / str(candidate)
            try:
                claim.mkdir()
            except FileExistsError:
                break
            claims.append(claim)
        if len(claims) != len(ports):
            for claim in claims:
                claim.rmdir()
            continue

        probes: list[socket.socket] = []
        for candidate in ports:
            result = bind_probe(candidate, sock_type)
            if result is None:
                break
            probes.extend(result)
        if len(probes) == len(ports) * (2 if socket.has_ipv6 else 1):
            for probe in probes:
                probe.close()
            print(port)
            return

        for probe in probes:
            probe.close()
        for claim in claims:
            claim.rmdir()

    kind = "UDP" if args.udp else "TCP"
    raise SystemExit(f"unable to allocate a unique free {kind} test port below the ephemeral range")


if __name__ == "__main__":
    main()
