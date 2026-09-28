import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import { mkdtemp, mkdir, readFile, writeFile, utimes, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import * as path from 'node:path';
import { MultiPeerHarness } from './harness/MultiPeerHarness';
import { buildOutputIsFresh, nativePeerEnvironment, SlskrNode } from './harness/SlskrNode';
import { runOwnedCommand, SharedBuildOwner } from './harness/BuildOwner';
import { NodeProcessLogs } from './harness/NodeProcessLogs';

test('node logs drain actual child output and close their file handles', async () => {
  const root = await mkdtemp(path.join(tmpdir(), 'slskr-log-owner-'));
  const stdoutPath = path.join(root, 'stdout.log');
  const stderrPath = path.join(root, 'stderr.log');
  const logs = await NodeProcessLogs.open(stdoutPath, stderrPath);
  const handles = (logs as unknown as { handles: import('node:fs/promises').FileHandle[] }).handles;
  const child = spawn(process.execPath, ['-e', "process.stdout.write('out\\n'.repeat(32768)); process.stderr.write('err\\n'.repeat(32768));"], { stdio: ['ignore', 'pipe', 'pipe'] });
  logs.connect(child);
  const node = new SlskrNode({ nodeName: 'log-owner', shareDir: '' });
  (node as unknown as { process: ChildProcess; logs: NodeProcessLogs }).process = child;
  (node as unknown as { logs: NodeProcessLogs }).logs = logs;
  try {
    await new Promise<void>((resolve, reject) => {
      child.once('close', resolve);
      child.once('error', reject);
    });
    await node.stop();
    expect(await readFile(stdoutPath, 'utf8')).toBe('out\n'.repeat(32768));
    expect(await readFile(stderrPath, 'utf8')).toBe('err\n'.repeat(32768));
    expect(handles.map((handle) => handle.fd)).toEqual([-1, -1]);
    await node.stop();
  } finally {
    await node.stop();
    await rm(root, { recursive: true, force: true });
  }
});

test('unlaunched node logs close explicitly and failed startup releases resources', async () => {
  const root = await mkdtemp(path.join(tmpdir(), 'slskr-log-startup-'));
  const logs = await NodeProcessLogs.open(path.join(root, 'stdout.log'), path.join(root, 'stderr.log'));
  const handles = (logs as unknown as { handles: import('node:fs/promises').FileHandle[] }).handles;
  await logs.close();
  expect(handles.map((handle) => handle.fd)).toEqual([-1, -1]);
  await logs.close();
  const node = new SlskrNode({ nodeName: 'failed-spawn', shareDir: '' });
  (node as unknown as { getBinaryPath: () => Promise<string> }).getBinaryPath = async () => path.join(root, 'missing-slskr');
  try {
    await expect(node.start()).rejects.toThrow('Failed to start slskr process');
    const internal = node as unknown as { process: ChildProcess | null; logs?: NodeProcessLogs; startupPromise?: Promise<void> };
    expect(internal.process).toBeNull();
    expect(internal.logs).toBeUndefined();
    expect(internal.startupPromise).toBeUndefined();
    await node.stop();
  } finally {
    await node.stop();
    await rm(root, { recursive: true, force: true });
  }
});

test('owned commands preserve failures, bound deadlines, and accept retries', async () => {
  const signal = new AbortController().signal;
  await expect(runOwnedCommand(process.execPath, ['-e', 'process.exit(7)'], process.cwd(), signal)).rejects.toThrow('code 7');
  await expect(runOwnedCommand('slskr-intentionally-missing-command', [], process.cwd(), signal)).rejects.toThrow();
  await expect(runOwnedCommand(process.execPath, ['-e', 'setInterval(() => {}, 1000)'], process.cwd(), signal, 100)).rejects.toThrow('build deadline');
  await runOwnedCommand(process.execPath, ['-e', 'process.exit(0)'], process.cwd(), signal);
});

test('shared builds cancel only after their last consumer leaves and can rebuild', async () => {
  const owner = new SharedBuildOwner();
  const first = new AbortController();
  const second = new AbortController();
  let executions = 0;
  let buildSignal: AbortSignal | undefined;
  let complete!: () => void;
  const build = async (signal: AbortSignal) => {
    executions += 1;
    buildSignal = signal;
    await new Promise<void>((resolve) => { complete = resolve; });
  };
  const a = owner.run(first.signal, build);
  const b = owner.run(second.signal, build);
  const aRejected = expect(a).rejects.toThrow('first cancelled');
  await expect.poll(() => executions).toBe(1);
  first.abort(new Error('first cancelled'));
  await aRejected;
  expect(buildSignal?.aborted).toBe(false);
  const bRejected = expect(b).rejects.toThrow('last cancelled');
  second.abort(new Error('last cancelled'));
  expect(buildSignal?.aborted).toBe(true);
  let settled = false;
  void b.catch(() => { settled = true; });
  await new Promise((resolve) => setTimeout(resolve, 20));
  expect(settled).toBe(false);
  complete();
  await bRejected;
  await owner.run(new AbortController().signal, async () => { executions += 1; });
  expect(executions).toBe(2);
});

test('stopping a node cancels and joins its actual pending build command', async () => {
  const node = new SlskrNode({ nodeName: 'owned-build', shareDir: '' });
  const root = await mkdtemp(path.join(tmpdir(), 'slskr-command-owner-'));
  const pidFile = path.join(root, 'pid');
  const internal = node as unknown as {
    awaitBuild(owner: SharedBuildOwner, build: (signal: AbortSignal) => Promise<void>): Promise<void>;
  };
  const pending = internal.awaitBuild(new SharedBuildOwner(), (signal) => runOwnedCommand(process.execPath, ['-e', `require('fs').writeFileSync(${JSON.stringify(pidFile)}, String(process.pid)); setInterval(() => {}, 1000)`], process.cwd(), signal));
  const rejected = expect(pending).rejects.toThrow('Node stopped during build');
  try {
    await expect.poll(async () => {
      try { return await readFile(pidFile, 'utf8'); } catch { return ''; }
    }).not.toBe('');
    const pid = Number(await readFile(pidFile, 'utf8'));
    await node.stop();
    await rejected;
    expect(() => process.kill(pid, 0)).toThrow();
    await node.stop();
  } finally {
    await node.stop();
    await rejected;
    await rm(root, { recursive: true, force: true });
  }
});

test('POSIX cancellation terminates the owned command descendant', async () => {
  test.skip(process.platform === 'win32', 'POSIX process group proof; Windows uses taskkill /T');
  const root = await mkdtemp(path.join(tmpdir(), 'slskr-command-tree-'));
  const pidFile = path.join(root, 'descendant-pid');
  const cancellation = new AbortController();
  const descendant = `require('fs').writeFileSync(${JSON.stringify(pidFile)}, String(process.pid)); setInterval(() => {}, 1000)`;
  const parent = `const {spawn} = require('child_process'); const child = spawn(process.execPath, ['-e', ${JSON.stringify(descendant)}], {stdio: 'inherit'}); process.on('SIGTERM', () => child.kill('SIGTERM')); child.on('close', () => process.exit(0));`;
  const pending = runOwnedCommand(process.execPath, ['-e', parent], process.cwd(), cancellation.signal);
  const rejected = expect(pending).rejects.toThrow('cancel tree');
  try {
    await expect.poll(async () => {
      try { return await readFile(pidFile, 'utf8'); } catch { return ''; }
    }).not.toBe('');
    const pid = Number(await readFile(pidFile, 'utf8'));
    cancellation.abort(new Error('cancel tree'));
    await rejected;
    expect(() => process.kill(pid, 0)).toThrow();
  } finally {
    cancellation.abort(new Error('cancel tree'));
    await rejected;
    await rm(root, { recursive: true, force: true });
  }
});

test('ignored build directories do not invalidate a fresh binary', async () => {
  const root = await mkdtemp(path.join(tmpdir(), 'slskr-build-freshness-'));
  try {
    const inputs = path.join(root, 'crates');
    await mkdir(inputs);
    const source = path.join(inputs, 'source.rs');
    const output = path.join(root, 'slskr');
    await writeFile(source, 'source');
    await writeFile(output, 'binary');
    await utimes(source, 100, 100);
    await utimes(output, 200, 200);
    await mkdir(path.join(inputs, 'target'));
    await writeFile(path.join(inputs, 'target', 'generated'), 'ignored');
    expect(await buildOutputIsFresh(output, [inputs])).toBe(true);
    await utimes(source, 300, 300);
    expect(await buildOutputIsFresh(output, [inputs])).toBe(false);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('native harness uses one loopback peer port for TCP and UDP transports', () => {
  expect(nativePeerEnvironment(43123)).toEqual({
    SLSKR_DHT_PORT: '43123',
    SLSKR_LISTENER_BIND: '127.0.0.1:43123',
    SLSKD_SLSK_LISTEN_PORT: '43123',
    SLSKR_OVERLAY_BIND: '127.0.0.1:43123',
  });
  for (const port of [0, -1, 65536, 1.5, NaN]) {
    expect(() => nativePeerEnvironment(port)).toThrow('Invalid native peer port');
  }
});

test('failed startup is registered for cleanup and preserves the startup error', async () => {
  const harness = new MultiPeerHarness();
  const originalStart = SlskrNode.prototype.start;
  const originalStop = SlskrNode.prototype.stop;
  let stopped = 0;
  const failure = new Error('intentional startup failure');
  SlskrNode.prototype.start = async function () {
    expect(harness.getNode('A')).toBe(this);
    throw failure;
  };
  SlskrNode.prototype.stop = async () => { stopped += 1; };
  try {
    await expect(harness.startNode('A', '')).rejects.toBe(failure);
    expect(stopped).toBe(1);
    expect(harness.getNodeNames()).toEqual([]);
  } finally {
    SlskrNode.prototype.start = originalStart;
    SlskrNode.prototype.stop = originalStop;
    await harness.stopAll();
  }
});

test('stop joins its actual child and repeated stop returns immediately', async () => {
  const node = new SlskrNode({ nodeName: 'owned-child', shareDir: '' });
  const child = spawn(process.execPath, ['-e', 'setInterval(() => {}, 1000)'], {
    stdio: 'ignore',
  });
  await new Promise<void>((resolve, reject) => {
    child.once('spawn', resolve);
    child.once('error', reject);
  });
  (node as unknown as { process: ChildProcess }).process = child;
  try {
    await node.stop();
    expect(child.exitCode !== null || child.signalCode !== null).toBe(true);
    await node.stop();
  } finally {
    if (child.exitCode === null && child.signalCode === null) {
      const closed = new Promise<void>((resolve) => child.once('close', resolve));
      child.kill('SIGKILL');
      await closed;
    }
  }
});
