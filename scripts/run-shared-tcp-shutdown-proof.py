#!/usr/bin/env python3
"""Retain bounded live single-peer-port and graceful gateway shutdown proof."""

import argparse
from datetime import datetime, timezone
import hashlib
import http.client
import json
import os
from pathlib import Path
import select
import signal
import socket
import ssl
import subprocess
import tempfile
import time


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def owned_socket_ports(pid, proc_root=Path('/proc')):
    """Intersect kernel socket tables with this daemon's own file descriptors."""
    descriptors = proc_root / str(pid) / 'fd'
    inodes = set()
    for index, entry in enumerate(descriptors.iterdir()):
        if index >= 4096:
            raise RuntimeError('fixture daemon exceeded the file descriptor budget')
        try:
            target = os.readlink(entry)
        except FileNotFoundError:
            continue
        if target.startswith('socket:[') and target.endswith(']'):
            inodes.add(target[8:-1])
    ports = {'tcp': [], 'udp': []}
    for protocol in ('tcp', 'tcp6', 'udp', 'udp6'):
        table = proc_root / str(pid) / 'net' / protocol
        if not table.exists():
            continue
        with table.open() as stream:
            next(stream, None)
            for line in stream:
                fields = line.split()
                if len(fields) < 10 or fields[9] not in inodes:
                    continue
                if protocol.startswith('tcp') and fields[3] != '0A':
                    continue
                ports[protocol[:3]].append(int(fields[1].rsplit(':', 1)[1], 16))
    return {key: sorted(values) for key, values in ports.items()}


def application(http_port, path='/api/v0/application'):
    connection = http.client.HTTPConnection('127.0.0.1', http_port, timeout=2)
    try:
        connection.request('GET', path)
        response = connection.getresponse()
        data = response.read()
        if response.status != 200:
            raise RuntimeError(f'application readiness returned {response.status}')
        return json.loads(data)
    finally:
        connection.close()


def require_closed(client):
    client.settimeout(3)
    try:
        data = client.recv(1)
    except (ConnectionResetError, ConnectionAbortedError, ssl.SSLEOFError):
        return
    if data != b'':
        raise RuntimeError('stalled gateway client received data after shutdown')


def probe_report(output, command):
    report = json.loads(output)
    if (not isinstance(report, dict) or report.get('status') != 'ok'
            or report.get('probe') != command.removesuffix('-probe')
            or type(report.get('duration_ms')) is not int or report['duration_ms'] < 0):
        raise RuntimeError(f'{command} did not report measured success')
    return report


def main():
    repo = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, default=repo / 'target/debug/slskr')
    parser.add_argument('--output', type=Path, default=repo / 'target/rf-shared-tcp-shutdown.json')
    parser.add_argument('--cycles', type=int, default=3)
    parser.add_argument('--all-udp-transports', action='store_true',
                        help='enable DHT and both QUIC ALPNs on the shared peer port')
    args = parser.parse_args()
    if os.name != 'posix' or not Path('/proc/self/fd').exists():
        parser.error('this proof requires Linux procfs and SIGTERM')
    if not 1 <= args.cycles <= 5:
        parser.error('--cycles must be between 1 and 5')
    binary = args.binary.resolve(strict=True)
    record = {
        'schemaVersion': 1,
        'evidenceMode': 'live isolated daemon process; not deployed interop proof',
        'sourceSha': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=repo, text=True).strip(),
        'dirtyWorktree': bool(subprocess.check_output(['git', 'status', '--porcelain'], cwd=repo, text=True)),
        'binarySha256': digest(binary),
        'harnessSha256': digest(Path(__file__)),
        'transportSourceSha256': {str(path): digest(repo / path) for path in (
            Path('crates/slskr/src/daemon_serve.rs'),
            Path('crates/slskr/src/config_parts/startup.rs'),
            Path('crates/slskr/src/config_parts/peer_transport.rs'),
            Path('crates/slskr/src/private_gateway.rs'),
            Path('crates/slskr/src/private_gateway_owners/gateway_models.rs'),
            Path('crates/slskr/src/private_gateway_owners/gateway_transport.rs'),
            Path('crates/slskr/src/private_gateway_owners/gateway_udp_runtime.rs'),
            Path('crates/slskr/src/private_gateway_owners/shared_quic_runtime.rs'),
            Path('crates/slskr/src/dht.rs'), Path('crates/slskr/src/quic_alpn.rs'),
            Path('crates/slskr-client/src/shared_udp.rs'),
            Path('crates/slskr-client/src/shared_quic_server.rs'),
            Path('crates/slskr-client/src/quic_control.rs'),
            Path('crates/slskr-client/src/quic_data.rs'), Path('vendor/mainline/src/lib.rs'),
            Path('vendor/mainline/src/dht.rs'), Path('vendor/mainline/src/rpc/config.rs'),
            Path('vendor/mainline/src/rpc/socket.rs'),
        )},
        'startedUtc': datetime.now(timezone.utc).isoformat(),
        'dhtEnabled': args.all_udp_transports,
        'meshDhtEnabled': args.all_udp_transports,
        'quicEnabled': args.all_udp_transports,
        'quicDataEnabled': args.all_udp_transports,
        'productionPortsAdded': 0,
        'cycles': [],
    }
    try:
        for cycle in range(args.cycles):
            with tempfile.TemporaryDirectory(prefix='slskr-shared-tcp-') as directory:
                root = Path(directory)
                with socket.socket() as http_reservation, socket.socket() as peer_reservation:
                    http_reservation.bind(('127.0.0.1', 0))
                    peer_reservation.bind(('127.0.0.1', 0))
                    http_port = http_reservation.getsockname()[1]
                    peer_port = peer_reservation.getsockname()[1]
                config = root / 'config.toml'
                enabled = 'true' if args.all_udp_transports else 'false'
                config.write_text(
                    f'[dht]\nenabled = {enabled}\nlan_only = true\n'
                    f'[mesh]\nenabled = true\nenable_dht = {enabled}\nenable_overlay = true\nenable_stun = false\n'
                    f'[overlay]\nenable = true\nenable_quic = {enabled}\n'
                    f'[overlay_data]\nenable = {enabled}\n'
                    '[mesh_gateway]\nenabled = false\n'
                )
                environment = {key: value for key, value in os.environ.items() if not key.startswith('SLSK')}
                environment.update(
                    SLSKR_CONFIG=str(config), SLSKR_STATE_DIR=str(root / 'state'),
                    SLSKR_HTTP_BIND=f'127.0.0.1:{http_port}', SLSKR_AUTH_DISABLED='true',
                    SLSKD_NO_HTTPS='true', SLSKR_LISTENER_BIND=f'127.0.0.1:{peer_port}',
                    SLSK_LISTEN_PORT=str(peer_port), SLSKR_CONTROLLER_PROFILE='native',
                    SLSKR_PARITY_PROFILE='current', SLSKD_SLSK_USERNAME='rf-single-port',
                )
                clients = []
                log = root / 'daemon.log'
                with log.open('wb') as output:
                    proc = subprocess.Popen(
                        [str(binary), 'serve', '--no-connect', '--no-share-scan', '--no-logo', '--no-version-check'],
                        cwd=repo, env=environment, stdout=output, stderr=subprocess.STDOUT,
                    )
                    try:
                        deadline = time.monotonic() + 20
                        while True:
                            if proc.poll() is not None:
                                raise RuntimeError('daemon exited before readiness: ' + log.read_text()[-4000:])
                            try:
                                application(http_port)
                                break
                            except (OSError, http.client.HTTPException):
                                if time.monotonic() >= deadline:
                                    raise RuntimeError('daemon readiness deadline exceeded')
                                time.sleep(.02)
                        udp_probes = []
                        if args.all_udp_transports:
                            # CLI probes discover this fixture's self-signed certificate,
                            # then send using the exact discovered public-key pin.
                            probe_env = dict(environment, SLSKR_OVERLAY_ENDPOINT=f'127.0.0.1:{peer_port}',
                                             SLSKR_PROBE_OUTPUT='json')
                            for command in ('overlay-quic-control-probe', 'quic-data-probe'):
                                result = subprocess.run([str(binary), command], cwd=repo, env=probe_env,
                                                        capture_output=True, timeout=25)
                                if result.returncode != 0:
                                    raise RuntimeError(f'{command} failed: {result.stderr[-2000:]!r} {result.stdout[-2000:]!r}')
                                report = probe_report(result.stdout, command)
                                udp_probes.append({'command': command, 'exitCode': result.returncode,
                                                   'result': report,
                                                   'outputSha256': hashlib.sha256(result.stdout).hexdigest()})
                            with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as dht_peer:
                                dht_peer.bind(('127.0.0.1', 0))
                                dht_peer.settimeout(2)
                                dht_peer.sendto(b'd1:ad2:id20:AAAAAAAAAAAAAAAAAAAAe1:q4:ping1:t4:aaaa1:y1:qe',
                                                ('127.0.0.1', peer_port))
                                reply, source = dht_peer.recvfrom(2048)
                                if source != ('127.0.0.1', peer_port) or b'4:aaaa' not in reply or b'1:y1:r' not in reply:
                                    raise RuntimeError('DHT did not reply to the original peer from the public port')
                            udp_probes.append({'command': 'actual DHT ping/reply', 'publicSourcePort': True})
                        context = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
                        context.minimum_version = ssl.TLSVersion.TLSv1_2
                        context.check_hostname = False
                        context.verify_mode = ssl.CERT_NONE
                        capacity_case = cycle % 2 == 1
                        established_count = 3 if capacity_case else 1
                        certificate_hash = None
                        for _ in range(established_count):
                            connected = socket.create_connection(('127.0.0.1', peer_port), timeout=3)
                            clients.append(connected)
                            established = context.wrap_socket(connected, server_hostname='localhost')
                            clients[-1] = established
                            certificate_hash = hashlib.sha256(established.getpeercert(binary_form=True)).hexdigest()
                        if capacity_case:
                            # Successful TLS handshakes establish all three admitted handlers.
                            with socket.create_connection(('127.0.0.1', peer_port), timeout=3) as rejected:
                                rejected.sendall(b'\x16\x03\x03\x00\x80\x01\x00\x00\x7c')
                                require_closed(rejected)
                        else:
                            stalled = socket.create_connection(('127.0.0.1', peer_port), timeout=3)
                            clients.append(stalled)
                            stalled.sendall(b'\x16\x03\x03\x00\x80\x01\x00\x00\x7c')
                            if select.select(clients[established_count:], [], [], .1)[0]:
                                raise RuntimeError('partial TLS client did not remain stalled')
                        for established in clients[:established_count]:
                            # TLS 1.3 tickets can make the encrypted socket readable.
                            established.settimeout(.1)
                            try:
                                established.recv(1)
                            except socket.timeout:
                                pass
                            else:
                                raise RuntimeError('TLS client did not await overlay initialization')
                        ports = owned_socket_ports(proc.pid)
                        if ports['tcp'] != sorted([http_port, peer_port]) or ports['udp'] != [peer_port]:
                            raise RuntimeError(f'unexpected dedicated listener or missing shared socket: {ports}')
                        started = time.monotonic()
                        proc.send_signal(signal.SIGTERM)
                        code = proc.wait(timeout=10)
                        elapsed = round(time.monotonic() - started, 6)
                        if code != 0 or 'shutdown signal received' not in log.read_text():
                            raise RuntimeError(f'graceful shutdown failed with exit {code}')
                        for client in clients:
                            require_closed(client)
                        with socket.socket() as rebound_tcp, socket.socket(type=socket.SOCK_DGRAM) as rebound_udp:
                            rebound_tcp.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
                            rebound_tcp.bind(('127.0.0.1', peer_port))
                            rebound_udp.bind(('127.0.0.1', peer_port))
                        record['cycles'].append({
                            'cycle': cycle + 1, 'tcpListeners': 2, 'peerTcpListeners': 1,
                            'gatewayUdpListeners': 1, 'peerTcpAndUdpSamePort': True,
                            'stalledPartialTlsClients': 0 if capacity_case else 1,
                            'perIpCapacityRejectedFourthClient': True if capacity_case else None,
                            'establishedTlsAwaitingOverlayInit': established_count,
                            'certificateSha256': certificate_hash, 'exitCode': code,
                            'udpTransportProbes': udp_probes,
                            'shutdownSeconds': elapsed, 'allClientSocketsClosed': True,
                            'peerTcpAndUdpRebound': True,
                        })
                    finally:
                        if proc.poll() is None:
                            proc.kill()
                            proc.wait(timeout=5)
                        for client in clients:
                            client.close()
                        args.output.parent.mkdir(parents=True, exist_ok=True)
                        args.output.with_suffix('.daemon.log').write_bytes(log.read_bytes())
        record['success'] = True
    except Exception as error:
        record['success'] = False
        record['error'] = str(error)
        raise
    finally:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(record, indent=2) + '\n')
    print(json.dumps({'success': True, 'cycles': len(record['cycles'])}))


if __name__ == '__main__':
    main()
