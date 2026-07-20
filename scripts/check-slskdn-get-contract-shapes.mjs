#!/usr/bin/env node

import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const auditScript = path.join(scriptDir, 'audit-slskdn-controller-routes.mjs');
const args = process.argv.slice(2);

function option(name) {
  const index = args.indexOf(name);
  return index >= 0 ? args[index + 1] : undefined;
}

function usage(message) {
  if (message) process.stderr.write(`${message}\n`);
  process.stderr.write(
    'usage: check-slskdn-get-contract-shapes.mjs ' +
      '(--reference-json FILE | --reference-base URL) ' +
      '(--candidate-json FILE | --candidate-base URL) [--json]\n',
  );
  process.exit(2);
}

const referenceJson = option('--reference-json');
const candidateJson = option('--candidate-json');
const referenceBase = option('--reference-base');
const candidateBase = option('--candidate-base');
const jsonOutput = args.includes('--json');

if (Boolean(referenceJson) === Boolean(referenceBase)) {
  usage('select exactly one reference input');
}
if (Boolean(candidateJson) === Boolean(candidateBase)) {
  usage('select exactly one candidate input');
}

const dynamicShapeRoutes = new Set([
  '/api/v0/telemetry/metrics',
  '/api/v0/telemetry/metrics/kpi',
  '/api/v0/telemetry/prometheus',
  '/api/v0/telemetry/prometheus/kpis',
  '/api/v0/telemetry/reports/transfers/histogram',
]);

function readReport(file) {
  return JSON.parse(readFileSync(file, 'utf8'));
}

function probe(base) {
  const output = execFileSync(
    process.execPath,
    [
      auditScript,
      '--get',
      '--materialize',
      '--include-response',
      '--json',
      '--probe-base',
      base,
    ],
    { encoding: 'utf8', stdio: ['ignore', 'pipe', 'inherit'], maxBuffer: 32 * 1024 * 1024 },
  );
  return JSON.parse(output);
}

const reference = referenceJson ? readReport(referenceJson) : probe(referenceBase);
const candidate = candidateJson ? readReport(candidateJson) : probe(candidateBase);
if (!Array.isArray(reference) || !Array.isArray(candidate)) {
  usage('reports must be JSON arrays emitted by audit-slskdn-controller-routes.mjs');
}

function rowKey(row) {
  return `${row.method} ${row.route}`;
}

function jsonKind(value) {
  if (value === null) return 'null';
  if (Array.isArray(value)) return 'array';
  return typeof value;
}

function parseBody(row) {
  if (typeof row.responseBody !== 'string') return { error: 'response body was not captured' };
  try {
    return { value: JSON.parse(row.responseBody) };
  } catch {
    return { error: 'response body is not JSON' };
  }
}

const candidateByKey = new Map(candidate.map((row) => [rowKey(row), row]));
const gaps = [];
let compared = 0;
let comparedVersionedFailures = 0;
let skippedDynamic = 0;
let skippedReferenceNonSuccess = 0;
let skippedReferenceUnavailable = 0;
let skippedUnversionedReferenceFailure = 0;
let skippedReferenceNonJson = 0;

for (const expected of reference) {
  if (expected.method !== 'GET') continue;
  if (dynamicShapeRoutes.has(expected.route)) {
    skippedDynamic += 1;
    continue;
  }
  const key = rowKey(expected);
  const actual = candidateByKey.get(key);
  if (!actual) {
    gaps.push({ key, problem: 'candidate report is missing the route' });
    continue;
  }
  if (!(expected.status >= 200 && expected.status < 300)) {
    skippedReferenceNonSuccess += 1;
    if (expected.status === 0 || expected.status >= 500) {
      skippedReferenceUnavailable += 1;
      continue;
    }
    if (!/^\/api\/v\d+(?:\/|$)/.test(expected.route)) {
      skippedUnversionedReferenceFailure += 1;
      continue;
    }
    compared += 1;
    comparedVersionedFailures += 1;
    if (actual.status !== expected.status) {
      gaps.push({
        key,
        problem: `candidate status ${actual.status}; expected failure status ${expected.status}`,
      });
    }
    continue;
  }

  compared += 1;
  if (!(actual.status >= 200 && actual.status < 300)) {
    gaps.push({ key, problem: `candidate status ${actual.status}; expected a successful response` });
    continue;
  }
  if (expected.status === 204) {
    if (actual.status !== 204) {
      gaps.push({ key, problem: `candidate status ${actual.status}; expected 204` });
    }
    continue;
  }

  const expectedBody = parseBody(expected);
  const actualBody = parseBody(actual);
  if (expectedBody.error) {
    skippedReferenceNonJson += 1;
    continue;
  }
  if (actualBody.error) {
    gaps.push({ key, problem: `candidate ${actualBody.error}` });
    continue;
  }

  const expectedKind = jsonKind(expectedBody.value);
  const actualKind = jsonKind(actualBody.value);
  if (actualKind !== expectedKind) {
    gaps.push({ key, problem: `top-level JSON type ${actualKind}; expected ${expectedKind}` });
    continue;
  }

  if (expectedKind === 'object') {
    const missing = Object.keys(expectedBody.value).filter(
      (property) => !Object.hasOwn(actualBody.value, property),
    );
    if (missing.length > 0) {
      gaps.push({ key, problem: `missing top-level properties: ${missing.join(', ')}` });
    }
  }
}

const summary = {
  referenceRoutes: reference.length,
  candidateRoutes: candidate.length,
  compared,
  comparedVersionedFailures,
  skippedDynamic,
  skippedReferenceNonSuccess,
  skippedReferenceUnavailable,
  skippedUnversionedReferenceFailure,
  skippedReferenceNonJson,
  gaps: gaps.length,
};

if (jsonOutput) {
  process.stdout.write(`${JSON.stringify({ summary, gaps }, null, 2)}\n`);
} else {
  for (const gap of gaps) process.stdout.write(`${gap.key}\t${gap.problem}\n`);
  process.stdout.write(
    `slskdN GET contract shapes: ${compared} empty-state contracts compared; ` +
      `${comparedVersionedFailures} deterministic versioned failure statuses compared; ` +
      `${gaps.length} gaps; ${skippedDynamic} dynamic shapes excluded; ` +
      `${skippedReferenceUnavailable} reference timeout/5xx, ` +
      `${skippedUnversionedReferenceFailure} unversioned failure, and ` +
      `${skippedReferenceNonJson} non-JSON reference routes excluded\n`,
  );
}

if (gaps.length > 0) process.exitCode = 1;
