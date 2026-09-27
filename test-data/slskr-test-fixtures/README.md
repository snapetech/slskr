# slskr test fixtures (public domain / Creative Commons)

Test content for all domains (book, movie, music, tv) used by slskr CI and local test instances:
- parsing / metadata ingestion
- search query handling
- cover-art and text ingestion
- library indexing
- transport and storage (different sizes and file types)

## Static fixture baseline

- **In repo:** book text, cover/poster/thumb images, license, and metadata files.
- **Not in repo:** binary audio/video is optional and is not part of the deterministic CI baseline.
- `meta/manifest.json` contains the expected SHA-256 and byte count for every tracked fixture.

## Validate fixtures

From the **slskr repo root**:

```bash
./scripts/check-fixture-manifest.sh
```

The compatibility wrapper is also safe to run when an older local fixture cache
contains optional media:

```bash
./scripts/fetch-test-fixtures.sh
```

The current manifest declares no remote downloads, so the wrapper validates the
tracked files and leaves them unchanged. Optional media supplied by a local
test environment remains ignored and is never used to update the committed
manifest hashes.

## slskr share configuration

Use this directory as a **share** so slskr can serve, host, and process the content. In the configured share-directory list:

```yaml
shares:
  directories:
    - /path/to/slskr/test-data/slskr-test-fixtures
```

Example for a dev config next to the repo:

```yaml
shares:
  directories:
    - ./test-data/slskr-test-fixtures
```

After `check-fixture-manifest.sh`, run a share scan (or start slskr with share scan enabled). The tree `book/`, `movie/`, `music/open_goldberg/`, `tv/` will be indexed and browseable.

## Manifest

`meta/manifest.json` lists the tracked fixture paths, expected byte counts, and
SHA-256 digests used by the E2E harness and the manifest smoke check.
