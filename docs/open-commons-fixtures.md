# Open commons test fixtures

The repository does not commit binary media fixtures. Use `scripts/fetch-open-commons-fixtures.sh` to download a small, hash-pinned fixture set into ignored local storage:

```bash
scripts/fetch-open-commons-fixtures.sh
```

The default destination is `target/open-commons-fixtures`. Override it with either a positional destination or `SLSKR_COMMONS_FIXTURE_DIR`.

Validate the downloaded fixture set, hashes, and generated license summary with:

```bash
scripts/verify-open-commons-fixtures.sh
```

## Fixture policy

- Use only files with clear open licensing from stable public sources.
- Prefer public domain or CC0 fixtures for protocol tests to avoid attribution/share-alike ambiguity in redistributed test artifacts.
- Pin every fixture by byte size and SHA-256.
- Keep source URLs, license URLs, and attribution in `fixtures/open-commons/manifest.tsv`.
- Do not commit downloaded binaries.

## Current fixtures

| ID | File | Type | License | Source |
| --- | --- | --- | --- | --- |
| `commons-click-track` | `commons-click-track.ogg` | audio/ogg | Public domain | https://commons.wikimedia.org/wiki/File:Audacity_click_track_one_per_second_for_eight_seconds_mono88khz32bitfloat.ogg |
| `commons-example-sound` | `commons-example-sound.ogg` | audio/ogg | CC0-1.0 | https://commons.wikimedia.org/wiki/File:Example_sound_file_in_Ogg_Vorbis_format.ogg |
| `commons-gif-sample` | `commons-gif-sample.gif` | image/gif | Public domain | https://commons.wikimedia.org/wiki/File:GifSample.gif |
| `commons-example-image` | `commons-example-image.png` | image/png | Public domain | https://commons.wikimedia.org/wiki/File:Example_image.png |

The downloader emits `LICENSES.tsv` beside the downloaded files so live interoperability runs can publish or inspect the source/license metadata with their artifacts.
