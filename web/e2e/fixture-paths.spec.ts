import { test, expect } from '@playwright/test';
import { createHash } from 'node:crypto';
import { mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import * as path from 'node:path';
import {
  ensureFixtures,
  getRepoRootFromCwd,
  hasDownloadedMediaFixtures,
} from './fixtures/ensure-fixtures';

const media = [
  'music/open_goldberg/01_aria.ogg',
  'movie/sintel_512kb_stereo.mp4',
  'tv/pioneer_one_s01e01_sample.mp4',
];

async function fixtureRoot(): Promise<{ root: string; fixtures: string }> {
  const root = await mkdtemp(path.join(tmpdir(), 'slskr-fixture-paths-'));
  const fixtures = path.join(root, 'test-data', 'slskr-test-fixtures');
  await mkdir(path.join(root, 'crates', 'slskr'), { recursive: true });
  await writeFile(path.join(root, 'Cargo.toml'), '[workspace]\n');
  await writeFile(path.join(root, 'crates', 'slskr', 'Cargo.toml'), '[package]\n');
  for (const directory of ['meta', 'book', 'movie', 'music/open_goldberg', 'tv']) {
    await mkdir(path.join(fixtures, directory), { recursive: true });
  }
  const content = 'static fixture validation\n';
  await writeFile(path.join(fixtures, 'book', 'fixture.txt'), content);
  await writeFile(path.join(fixtures, 'meta', 'manifest.json'), JSON.stringify({
    files: [{ path: 'book/fixture.txt', bytes: Buffer.byteLength(content),
      sha256: createHash('sha256').update(content).digest('hex') }],
  }));
  return { root, fixtures };
}

async function presentMedia(fixtures: string): Promise<void> {
  // Presence-test stubs only; these are not encoded playback fixtures.
  for (const name of media) {
    await writeFile(path.join(fixtures, name), 'presence-only fixture stub');
  }
}

test('repository and fixture discovery work from root, web, and nested E2E directories', async () => {
  const { root, fixtures } = await fixtureRoot();
  try {
    await presentMedia(fixtures);
    for (const suffix of ['', 'web', 'web/e2e', 'web/e2e/fixtures', 'web/e2e/harness']) {
      const cwd = path.join(root, suffix);
      await mkdir(cwd, { recursive: true });
      expect(getRepoRootFromCwd(cwd)).toBe(root);
      expect(hasDownloadedMediaFixtures('test-data/slskr-test-fixtures', cwd)).toBe(true);
    }
    expect(hasDownloadedMediaFixtures(fixtures, tmpdir())).toBe(true);
    await ensureFixtures(fixtures);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('missing, empty, and directory media paths are unavailable', async () => {
  const { root, fixtures } = await fixtureRoot();
  try {
    expect(hasDownloadedMediaFixtures(fixtures)).toBe(false);
    await presentMedia(fixtures);
    expect(hasDownloadedMediaFixtures(fixtures)).toBe(true);
    const file = path.join(fixtures, media[0]);
    await writeFile(file, '');
    expect(hasDownloadedMediaFixtures(fixtures)).toBe(false);
    await rm(file);
    await mkdir(file);
    expect(hasDownloadedMediaFixtures(fixtures)).toBe(false);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('root discovery fails explicitly outside a repository', async () => {
  const directory = await mkdtemp(path.join(tmpdir(), 'slskr-no-fixture-root-'));
  try {
    expect(() => getRepoRootFromCwd(directory)).toThrow('Cannot locate slskr repository root');
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
