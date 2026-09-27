import { exec } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync } from 'node:fs';
import * as fs from 'node:fs/promises';
import * as path from 'node:path';
import { promisify } from 'node:util';

const execAsync = promisify(exec);
const OPTIONAL_MEDIA_FILES = [
  'music/open_goldberg/01_aria.ogg',
  'movie/sintel_512kb_stereo.mp4',
  'tv/pioneer_one_s01e01_sample.mp4',
];

function getRepoRootFromCwd(cwd: string = process.cwd()): string {
  return path.join(cwd, '..', '..', '..');
}

function getFullFixturesPath(
  fixturesDir: string,
  cwd: string = process.cwd(),
): string {
  const repoRoot = getRepoRootFromCwd(cwd);
  return path.isAbsolute(fixturesDir)
    ? fixturesDir
    : path.join(repoRoot, fixturesDir);
}

async function validateManifestFiles(
  fixturesPath: string,
  manifestPath: string,
): Promise<void> {
  let manifest: { files?: unknown };
  try {
    manifest = JSON.parse(await fs.readFile(manifestPath, 'utf8')) as {
      files?: unknown;
    };
  } catch (error) {
    throw new Error(`Test fixtures manifest is not valid JSON: ${error}`);
  }

  if (!Array.isArray(manifest.files) || manifest.files.length === 0) {
    throw new Error('Test fixtures manifest must contain a non-empty files array');
  }

  const seen = new Set<string>();
  for (const entry of manifest.files) {
    if (
      !entry ||
      typeof entry !== 'object' ||
      Array.isArray(entry) ||
      typeof (entry as { path?: unknown }).path !== 'string' ||
      typeof (entry as { sha256?: unknown }).sha256 !== 'string' ||
      typeof (entry as { bytes?: unknown }).bytes !== 'number'
    ) {
      throw new Error('Test fixtures manifest contains an invalid files entry');
    }

    const file = entry as { path: string; sha256: string; bytes: number };
    if (
      !file.path ||
      path.isAbsolute(file.path) ||
      file.path.split(/[\\/]/u).includes('..') ||
      seen.has(file.path) ||
      !/^[0-9a-f]{64}$/iu.test(file.sha256) ||
      !Number.isSafeInteger(file.bytes) ||
      file.bytes < 0
    ) {
      throw new Error(`Test fixtures manifest contains an invalid entry for ${file.path}`);
    }
    seen.add(file.path);

    const fullPath = path.join(fixturesPath, file.path);
    let stat;
    try {
      stat = await fs.stat(fullPath);
    } catch {
      throw new Error(`Required fixture file missing: ${fullPath}`);
    }
    if (!stat.isFile()) {
      throw new Error(`Required fixture path is not a file: ${fullPath}`);
    }
    if (stat.size !== file.bytes) {
      throw new Error(
        `Fixture byte count mismatch for ${file.path}: expected ${file.bytes}, got ${stat.size}`,
      );
    }

    const digest = createHash('sha256')
      .update(await fs.readFile(fullPath))
      .digest('hex');
    if (digest !== file.sha256.toLowerCase()) {
      throw new Error(
        `Fixture SHA-256 mismatch for ${file.path}: expected ${file.sha256}, got ${digest}`,
      );
    }
  }
}

export function hasDownloadedMediaFixtures(
  fixturesDir: string = 'test-data/slskr-test-fixtures',
  cwd: string = process.cwd(),
): boolean {
  const fullFixturesPath = getFullFixturesPath(fixturesDir, cwd);

  return OPTIONAL_MEDIA_FILES.every((mediaFile) =>
    existsSync(path.join(fullFixturesPath, mediaFile)),
  );
}

/**
 * Ensure test fixtures are available.
 *
 * Checks if fixture directories exist and have content.
 * If media files are missing, attempts to fetch them (optional).
 * @param fixturesDir Path to test-data/slskr-test-fixtures
 * @param fetchIfMissing If true, run fetch script if media files are missing
 */
export async function ensureFixtures(
  fixturesDir: string = 'test-data/slskr-test-fixtures',
  fetchIfMissing: boolean = false,
): Promise<void> {
  const repoRoot = getRepoRootFromCwd();
  const fullFixturesPath = getFullFixturesPath(fixturesDir);

  // Check if fixtures directory exists
  try {
    await fs.access(fullFixturesPath);
  } catch {
    throw new Error(`Test fixtures directory not found: ${fullFixturesPath}`);
  }

  // Check for manifest
  const manifestPath = path.join(fullFixturesPath, 'meta', 'manifest.json');
  try {
    await fs.access(manifestPath);
  } catch {
    throw new Error(`Test fixtures manifest not found: ${manifestPath}`);
  }

  await validateManifestFiles(fullFixturesPath, manifestPath);

  // Check if key directories exist (static files should always be present)
  const requiredDirectories = ['book', 'music', 'movie', 'tv'];
  for (const dir of requiredDirectories) {
    const dirPath = path.join(fullFixturesPath, dir);
    try {
      await fs.access(dirPath);
    } catch {
      throw new Error(`Required fixture directory missing: ${dirPath}`);
    }
  }

  // Optional media is intentionally outside the checked-in manifest.
  // Streaming specs skip when these locally supplied files are absent.
  const missingMedia: string[] = [];
  for (const mediaFile of OPTIONAL_MEDIA_FILES) {
    const filePath = path.join(fullFixturesPath, mediaFile);
    try {
      const stat = await fs.stat(filePath);
      if (stat.size === 0) {
        missingMedia.push(mediaFile);
      }
    } catch {
      missingMedia.push(mediaFile);
    }
  }

  if (missingMedia.length > 0 && fetchIfMissing) {
    console.log(
      `[E2E] Missing ${missingMedia.length} media files, fetching...`,
    );
    try {
      const fetchScript = path.join(
        repoRoot,
        'scripts',
        'fetch-test-fixtures.sh',
      );
      await execAsync(`bash "${fetchScript}"`, {
        cwd: repoRoot,
        env: { ...process.env, FIXTURES_DIR: fullFixturesPath },
      });
      const remainingMedia: string[] = [];
      for (const mediaFile of OPTIONAL_MEDIA_FILES) {
        try {
          const stat = await fs.stat(path.join(fullFixturesPath, mediaFile));
          if (!stat.isFile() || stat.size === 0) {
            remainingMedia.push(mediaFile);
          }
        } catch {
          remainingMedia.push(mediaFile);
        }
      }
      if (remainingMedia.length === 0) {
        console.log('[E2E] Optional media files are available');
      } else {
        console.warn(
          '[E2E] No downloadable media assets are declared; optional streaming files remain unavailable.',
        );
      }
    } catch (error) {
      console.warn(`[E2E] Failed to fetch media files: ${error}`);
      console.warn('[E2E] Tests will continue with static fixtures only');
    }
  } else if (missingMedia.length > 0) {
    console.warn(
      `[E2E] ${missingMedia.length} media files missing (optional): ${missingMedia.join(', ')}`,
    );
    console.warn(
      '[E2E] Tests will use static fixtures only; optional media must be supplied separately for streaming specs.',
    );
  }
}
