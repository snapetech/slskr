#!/usr/bin/env python3
"""Run bounded real-browser mock workflows and retain source-bound results."""

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess

SCENARIOS = ('success', 'rendered-loading-and-empty',
             'rendered-validation-and-server-error', 'authorization-reconnect-and-restart')


def run_owned(command, root, environment, log, timeout=300):
    """Reap the entire owned process group on errors, timeout, or interruption."""
    with log.open('wb') as output:
        process = subprocess.Popen(command, cwd=root, env=environment,
                                   stdout=output, stderr=subprocess.STDOUT,
                                   start_new_session=True)
        try:
            code = process.wait(timeout=timeout)
            if code:
                raise RuntimeError(f'{command[0]} exited {code}; see {log}')
        finally:
            # Let the memory guard stop its systemd unit before hard cleanup.
            try:
                os.killpg(process.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                pass
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            process.wait(timeout=5)


def scenario_environment(base, scenario, output):
    environment = {key: value for key, value in base.items()
                   if not key.startswith('SLSKR_REACT_WEB_AUDIT_')}
    environment.update(SLSKR_REACT_WEB_AUDIT_DIR=str(output),
                       SLSKR_REACT_WEB_AUDIT_SCENARIO=scenario, HEADLESS='true')
    if scenario != 'success':
        environment.update(SLSKR_REACT_WEB_AUDIT_SKIP_NAVIGATION='1',
                           SLSKR_REACT_WEB_AUDIT_SKIP_SCREENSHOTS='1',
                           SLSKR_REACT_WEB_AUDIT_ROUTES='/',
                           SLSKR_REACT_WEB_AUDIT_VIEWPORTS='desktop')
    return environment


def validate_audit(audit, scenario):
    if (audit.get('evidenceMode') != 'mock' or audit.get('scenario') != scenario
            or type(audit.get('endpointSweepCount')) is not int
            or audit['endpointSweepCount'] != 0 or audit.get('errors') != []):
        raise RuntimeError(f'invalid or failed {scenario} browser evidence')
    routes = audit.get('routes')
    if not isinstance(routes, list) or not routes:
        raise RuntimeError(f'{scenario} has no observed routes')
    viewports = {route.get('viewport') for route in routes}
    if scenario == 'success' and viewports != {'desktop', 'mobile'}:
        raise RuntimeError('success audit must cover desktop and mobile')
    responses = [response for route in routes for response in route.get('apiResponses', [])]
    if not responses:
        raise RuntimeError(f'{scenario} has no observed UI requests')
    return {'renderChecks': len(routes), 'observedUiResponses': len(responses),
            'viewports': sorted(viewports)}


def interrupt(signum, _frame):
    raise InterruptedError(f'browser runner interrupted by signal {signum}')


def main():
    signal.signal(signal.SIGTERM, interrupt)
    root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=root / 'target/react-nightly-audit')
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    environment = os.environ.copy()
    if not environment.get('SLSKR_PLAYWRIGHT_EXECUTABLE_PATH'):
        chromium = shutil.which('chromium') or shutil.which('chromium-browser')
        if chromium:
            environment['SLSKR_PLAYWRIGHT_EXECUTABLE_PATH'] = chromium
    record = {
        'schemaVersion': 1, 'evidenceMode': 'real Chromium against deterministic mock responses',
        'sourceCommit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip(),
        'worktreeDirty': bool(subprocess.check_output(['git', 'status', '--porcelain'], cwd=root, text=True)),
        'startedUtc': datetime.now(timezone.utc).isoformat(),
        'harnessSha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'scenarios': [], 'success': False,
    }
    guard = str(root / 'scripts/with-process-memory-guard.sh')
    try:
        run_owned([guard, 'npm', '--prefix', 'web', 'run', 'build'], root,
                  environment, output / 'build.log')
        for scenario in SCENARIOS:
            directory = output / scenario
            directory.mkdir(parents=True, exist_ok=True)
            # Never accept evidence left by a previous run after a launch failure.
            audit_path = directory / 'audit.json'
            audit_path.unlink(missing_ok=True)
            run_owned([guard, 'node', 'web/scripts/audit-react-webui.mjs'], root,
                      scenario_environment(environment, scenario, directory),
                      output / f'{scenario}.log',
                      timeout=480 if scenario == 'success' else 300)
            audit = json.loads(audit_path.read_text())
            observed = validate_audit(audit, scenario)
            record['scenarios'].append({'scenario': scenario, **observed,
                                       'auditSha256': hashlib.sha256(audit_path.read_bytes()).hexdigest()})
        record['success'] = True
    except Exception as error:
        record['error'] = str(error)
        raise
    finally:
        (output / 'receipt.json').write_text(json.dumps(record, indent=2) + '\n')
    print(json.dumps(record, indent=2))


if __name__ == '__main__':
    main()
