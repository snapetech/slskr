import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import { MultiPeerHarness } from './harness/MultiPeerHarness';
import { nativePeerEnvironment, SlskrNode } from './harness/SlskrNode';

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
