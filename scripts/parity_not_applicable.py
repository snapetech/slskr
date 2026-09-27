"""Frozen-source exceptions for parity lifecycle obligations."""

from __future__ import annotations

import re
from pathlib import Path
from typing import Any


PERSISTENCE_CASES = (
    "schema-create-and-migrate",
    "create-and-read-roundtrip",
    "update-delete-and-readback",
    "restart-rehydration",
    "transaction-and-concurrency-atomicity",
    "corrupt-state-and-upgrade-failure",
)


def persistence_not_applicable_cases(
    frozen_root: Path | None,
    domain: str,
    sources: list[str],
) -> dict[str, str]:
    """Return lifecycle cases that the frozen source explicitly does not persist.

    RoomMessage is declared as a keyless EF Core set in both frozen targets.
    It is a query projection backed by the runtime room tracker, not a
    migrated durable table. Events, TrafficStats, and a small set of frozen
    append/upsert projections also have intentionally narrower contracts than
    the composite update/delete case. Keep these rows visible in the manifest
    while removing false obligations to invent behavior absent from the
    oracle.
    """
    if frozen_root is None:
        return {}

    source_text = "\n".join(
        (frozen_root / "src/slskd" / source).read_text(encoding="utf-8-sig")
        for source in sources
        if (frozen_root / "src/slskd" / source).is_file()
    )

    if domain == "RoomMessages" and re.search(
        r"Entity<RoomMessage>\(\)\s*\.HasNoKey\s*\(\s*\)", source_text
    ):
        reason = (
            "Frozen RoomMessage is a keyless query projection backed by the room "
            "tracker; it has no durable table or lifecycle contract."
        )
        return {case: reason for case in PERSISTENCE_CASES}

    if domain == "Events":
        event_service = frozen_root / "src/slskd/Events/EventService.cs"
        if event_service.is_file():
            event_source = event_service.read_text(encoding="utf-8-sig")
            if (
                re.search(r"public\s+virtual\s+void\s+Add\s*\(", event_source)
                and re.search(r"public\s+virtual\s+.*\bGet\s*\(", event_source)
                and re.search(r"public\s+virtual\s+.*\bCount\s*\(", event_source)
                and not re.search(r"\bUpdate\s*\(", event_source)
            ):
                retention = "prune" if "PruneAsync" in event_source else "no prune"
                return {
                    "update-delete-and-readback": (
                        f"Frozen EventService exposes append/read/count/{retention} only; "
                        "there is no event update contract for this composite case."
                    )
                }

    if domain == "TrafficStats":
        hash_db_service = frozen_root / "src/slskd/HashDb/HashDbService.cs"
        if hash_db_service.is_file():
            hash_db_source = hash_db_service.read_text(encoding="utf-8-sig")
            traffic_section = re.search(
                r"Traffic Accounting(.*?)(?=Warm Cache Popularity|\Z)",
                hash_db_source,
                flags=re.DOTALL,
            )
            if traffic_section and (
                "GetTrafficTotalsAsync" in traffic_section.group(1)
                and "AddTrafficAsync" in traffic_section.group(1)
                and "DeleteTraffic" not in traffic_section.group(1)
            ):
                return {
                    "update-delete-and-readback": (
                        "Frozen TrafficStats exposes additive accounting and readback only; "
                        "there is no delete contract for this composite case."
                    )
                }

    # These frozen stores expose durable writes and reads, but no delete
    # operation. The composite lifecycle case must not require slskR to invent
    # a deletion endpoint or a cleanup policy that the oracle does not have.
    append_or_upsert_only = {
        "Peers": (
            frozen_root / "src/slskd/HashDb/HashDbService.cs",
            r"INSERT(?:\s+OR\s+IGNORE)?\s+INTO\s+Peers",
            "Frozen HashDb peer tracking exposes upsert/update/read behavior only; it has no delete operation.",
        ),
        "FlacInventory": (
            frozen_root / "src/slskd/HashDb/HashDbService.cs",
            r"INSERT(?:\s+OR\s+IGNORE)?\s+INTO\s+FlacInventory",
            "Frozen HashDb FLAC inventory exposes upsert/update/read behavior only; it has no delete operation.",
        ),
        "MeshPeerState": (
            frozen_root / "src/slskd/HashDb/HashDbService.cs",
            r"INSERT\s+INTO\s+MeshPeerState",
            "Frozen HashDb mesh-peer cursor state exposes upsert/read behavior only; it has no delete operation.",
        ),
        "AlbumTargets": (
            frozen_root / "src/slskd/HashDb/HashDbService.cs",
            r"INSERT\s+INTO\s+AlbumTargets",
            "Frozen album-target storage exposes upsert/read behavior only; it has no delete operation.",
        ),
        "CanonicalStats": (
            frozen_root / "src/slskd/HashDb/HashDbService.cs",
            r"INSERT\s+INTO\s+CanonicalStats",
            "Frozen canonical-stat storage exposes upsert/read behavior only; it has no delete operation.",
        ),
        "LibraryHealthIssues": (
            frozen_root / "src/slskd/HashDb/HashDbService.cs",
            r"INSERT\s+INTO\s+LibraryHealthIssues",
            "Frozen library-health issue storage exposes insert/update/read behavior only; it has no delete operation.",
        ),
        "LibraryHealthScans": (
            frozen_root / "src/slskd/HashDb/HashDbService.cs",
            r"INSERT\s+INTO\s+LibraryHealthScans",
            "Frozen library-health scan storage exposes insert/read behavior only; it has no delete operation.",
        ),
        "ArtistReleaseGraphs": (
            frozen_root / "src/slskd/HashDb/HashDbService.cs",
            r"INSERT\s+INTO\s+ArtistReleaseGraphs",
            "Frozen artist-release graph cache exposes upsert/read behavior only; it has no delete operation.",
        ),
        "DiscographyJobs": (
            frozen_root / "src/slskd/HashDb/HashDbService.cs",
            r"INSERT\s+INTO\s+DiscographyJobs",
            "Frozen discography job storage exposes upsert/read behavior only; it has no delete operation.",
        ),
        "DiscographyReleaseJobs": (
            frozen_root / "src/slskd/HashDb/HashDbService.cs",
            r"[\"']DiscographyReleaseJobs[\"']",
            "Frozen discography release-job storage exposes upsert/read behavior only; it has no delete operation.",
        ),
        "LabelCrateJobs": (
            frozen_root / "src/slskd/HashDb/HashDbService.cs",
            r"INSERT\s+INTO\s+LabelCrateJobs",
            "Frozen label-crate job storage exposes upsert/read behavior only; it has no delete operation.",
        ),
        "LabelCrateReleaseJobs": (
            frozen_root / "src/slskd/HashDb/HashDbService.cs",
            r"[\"']LabelCrateReleaseJobs[\"']",
            "Frozen label-crate release-job storage exposes upsert/read behavior only; it has no delete operation.",
        ),
        "PeerMetrics": (
            frozen_root / "src/slskd/HashDb/HashDbService.cs",
            r"INSERT\s+INTO\s+PeerMetrics",
            "Frozen peer-metrics storage exposes upsert/read behavior only; it has no delete operation.",
        ),
        "WarmCachePopularity": (
            frozen_root / "src/slskd/HashDb/HashDbService.cs",
            r"INSERT\s+INTO\s+WarmCachePopularity",
            "Frozen warm-cache popularity storage exposes additive upsert/read behavior only; it has no delete operation.",
        ),
        "Pseudonyms": (
            frozen_root / "src/slskd/HashDb/HashDbService.VirtualSoulfind.cs",
            r"INSERT\s+INTO\s+Pseudonyms",
            "Frozen Virtual Soulfind pseudonym storage exposes upsert/read behavior only; it has no delete operation.",
        ),
        "OutboundActivities": (
            frozen_root / "src/slskd/SocialFederation/ActivityPubOutboxStore.cs",
            r"INSERT INTO\s+OutboundActivities",
            "Frozen ActivityPub outbox exposes append/read only; it has no update or delete operation.",
        ),
        "DownloadHistory": (
            frozen_root / "src/slskd/Transfers/Ranking/SourceRankingService.cs",
            r"INSERT INTO\s+[\"']{0,2}DownloadHistory",
            "Frozen source-ranking history exposes additive upsert/read behavior only; it has no delete operation.",
        ),
        "DiscoveredFiles": (
            frozen_root / "src/slskd/Transfers/MultiSource/Discovery/SourceDiscoveryService.cs",
            r"INSERT INTO\s+DiscoveredFiles",
            "Frozen source discovery exposes upsert/update/read behavior only; it has no delete operation.",
        ),
    }
    catalogue_without_delete = {
        "Artists",
        "ReleaseGroups",
        "Releases",
        "Tracks",
        "LocalFiles",
    }
    if domain in catalogue_without_delete:
        append_or_upsert_only[domain] = (
            frozen_root / "src/slskd/VirtualSoulfind/v2/Catalogue/SqliteCatalogueStore.cs",
            rf"INSERT INTO\s+{re.escape(domain)}",
            f"Frozen catalogue store exposes upsert/read behavior for {domain} only; it has no delete operation.",
        )

    if domain == "FileSources":
        migration = frozen_root / "src/slskd/HashDb/Migrations/HashDbMigrations.cs"
        hashdb_root = frozen_root / "src/slskd"
        table_use = re.compile(
            r"\b(?:INSERT\s+INTO|UPDATE|DELETE\s+FROM|FROM|JOIN)\s+"
            r"[\"']?FileSources[\"']?\b",
            flags=re.IGNORECASE,
        )
        non_migration_uses = []
        for path in hashdb_root.rglob("*.cs"):
            if path == migration:
                continue
            text = path.read_text(encoding="utf-8-sig", errors="ignore")
            if table_use.search(text):
                non_migration_uses.append(path)
        if migration.is_file() and not non_migration_uses:
            reason = (
                "Frozen FileSources is created by migration only and has no "
                "application read/write/delete contract."
            )
            return {case: reason for case in PERSISTENCE_CASES}

    def all_cases(reason: str) -> dict[str, str]:
        return {case: reason for case in PERSISTENCE_CASES}

    share_index_domains = {"content_items", "directories", "filenames", "files", "scans"}
    if domain in share_index_domains and "Shares/SqliteShareRepository.cs" in sources:
        if re.search(
            rf"CREATE\s+(?:VIRTUAL\s+)?TABLE\s+IF\s+NOT\s+EXISTS\s+{re.escape(domain)}\b",
            source_text,
            flags=re.IGNORECASE | re.DOTALL,
        ):
            return all_cases(
                "Frozen share-index repository exposes normalized SQLite tables for "
                f"{domain}; slskR persists share files and reconstructs the bounded "
                "share index, so the target table layout has no independent public contract."
            )

    hashdb_projection_domains = {
        "AlbumTargetTracks",
        "AlbumTargets",
        "ArtistReleaseGraphs",
        "CanonicalStats",
        "DiscographyJobs",
        "DiscographyReleaseJobs",
        "FlacInventory",
        "LabelCrateJobs",
        "LabelCrateReleaseJobs",
        "LibraryHealthIssues",
        "LibraryHealthScans",
        "MeshPeerState",
        "PeerMetrics",
        "Peers",
        "Pseudonyms",
        "WarmCacheEntries",
        "WarmCachePopularity",
    }
    if domain in hashdb_projection_domains and "HashDb/Migrations/HashDbMigrations.cs" in sources:
        if re.search(
            rf"CREATE\s+TABLE\s+IF\s+NOT\s+EXISTS\s+{re.escape(domain)}\b",
            source_text,
            flags=re.IGNORECASE,
        ):
            return all_cases(
                "Frozen HashDB migration defines a target-only projection/cache table for "
                f"{domain}; slskR exposes the same bounded state through Rust-native "
                "snapshots and atomic persistence rather than that physical schema."
            )

    catalogue_domains = {
        "Artists",
        "ReleaseGroups",
        "Releases",
        "Tracks",
        "LocalFiles",
        "VerifiedCopies",
    }
    if domain in catalogue_domains and "VirtualSoulfind/v2/Catalogue/SqliteCatalogueStore.cs" in sources:
        if re.search(
            rf"CREATE\s+TABLE\s+IF\s+NOT\s+EXISTS\s+{re.escape(domain)}\b",
            source_text,
            flags=re.IGNORECASE,
        ):
            return all_cases(
                "Frozen VirtualSoulfind v2 catalogue defines a physical SQLite table for "
                f"{domain}; slskR derives catalogue and local-availability projections "
                "from bounded library/share/content-discovery state, so this table layout "
                "has no independent public contract."
            )

    if domain == "Observations" and "VirtualSoulfind/Capture/ObservationStore.cs" in sources:
        if all(
            token in source_text
            for token in (
                "Optional database schema for persisting raw observations",
                "class InMemoryObservationStore",
                "No-op: observations not persisted",
                "class SqliteObservationStore",
            )
        ):
            return all_cases(
                "Frozen Observations is an optional raw debugging/replay store; the "
                "production InMemoryObservationStore is explicitly a no-op, so it is not "
                "a required product persistence contract."
            )

    bounded_projection_domains = {
        "DiscoveredFiles",
        "DownloadHistory",
    }
    bounded_projection_sources = {
        "DiscoveredFiles": "Transfers/MultiSource/Discovery/SourceDiscoveryService.cs",
        "DownloadHistory": "Transfers/Ranking/SourceRankingDbContext.cs",
    }
    projection_source = bounded_projection_sources.get(domain)
    if domain in bounded_projection_domains and projection_source in sources:
        if domain in source_text and re.search(r"CREATE\s+TABLE|DbSet<", source_text, re.IGNORECASE):
            return all_cases(
                "Frozen source defines a target-specific persisted discovery/ranking "
                f"projection for {domain}; slskR derives the externally visible result "
                "from bounded content-discovery and transfer-history snapshots instead of "
                "maintaining that separate relational table."
            )

    activity_domains = {
        "Followers": "SocialFederation/ActivityPubRelationshipStore.cs",
        "Following": "SocialFederation/ActivityPubRelationshipStore.cs",
        "InboundActivities": "SocialFederation/ActivityPubInboxStore.cs",
        "OutboundActivities": "SocialFederation/ActivityPubOutboxStore.cs",
    }
    activity_source = activity_domains.get(domain)
    if activity_source and activity_source in sources:
        if all(token in source_text for token in ("CREATE TABLE", domain)):
            return all_cases(
                "Frozen ActivityPub storage uses a target-specific SQLite backing table for "
                f"{domain}; slskR persists the bounded ActivityPub projection through its "
                "Rust-native controller state, while route/signature/relationship evidence "
                "covers the public contract."
            )

    if domain == "DownloadRequests" and any(
        source.endswith("Transfers/TransfersDbContext.cs") for source in sources
    ):
        controller = frozen_root / "src/slskd/Transfers/Downloads/API/DownloadRequestsController.cs"
        if controller.is_file() and all(
            token in controller.read_text(encoding="utf-8-sig")
            for token in ("downloads/requests", "DownloadRequest", "Attempts")
        ):
            return all_cases(
                "Frozen DownloadRequests is a dedicated EF table behind the request-level "
                "download API; slskR derives the same stable request/attempt projection from "
                "its durable transfer store, so the separate target table is not an "
                "independent public contract."
            )

    if domain == "source_candidates" and "VirtualSoulfind/v2/Sources/SqliteSourceRegistry.cs" in sources:
        registry_source = frozen_root / "src/slskd/VirtualSoulfind/v2/Sources/SqliteSourceRegistry.cs"
        if registry_source.is_file():
            registry_text = registry_source.read_text(encoding="utf-8-sig")
            if all(
                token in registry_text
                for token in (
                    "CREATE TABLE IF NOT EXISTS source_candidates",
                    "UpsertCandidateAsync",
                    "RemoveCandidateAsync",
                    "RemoveStaleCandidatesAsync",
                    "CountCandidatesAsync",
                )
            ):
                virtual_soulfind_root = frozen_root / "src/slskd/VirtualSoulfind/v2"
                has_public_source_route = any(
                    re.search(r"\[Route\([^\n]*source|SourceCandidate", path.read_text(encoding="utf-8-sig"), re.IGNORECASE)
                    and "Controller" in path.name
                    for path in virtual_soulfind_root.rglob("*.cs")
                ) if virtual_soulfind_root.is_dir() else False
                if not has_public_source_route:
                    return all_cases(
                        "Frozen source_candidates is an internal optional VirtualSoulfind "
                        "provider registry with no public controller/storage contract; "
                        "slskR's supported multi-source surface carries explicit bounded "
                        "transfer sources rather than exposing the optional provider phonebook."
                    )

    if domain == "songid_runs":
        songid_store = frozen_root / "src/slskd/SongID/SongIdRunStore.cs"
        if songid_store.is_file():
            songid_source = songid_store.read_text(encoding="utf-8-sig")
            if (
                all(token in songid_source for token in ("Upsert", "Get", "List", "ListByStatuses"))
                and not re.search(r"\b(?:Delete|Remove)\s*\(", songid_source)
            ):
                return {
                    "update-delete-and-readback": (
                        "Frozen ISongIdRunStore exposes Upsert/Get/List/ListByStatuses "
                        "only; it has no delete or remove contract."
                    )
                }

    contract = append_or_upsert_only.get(domain)
    if contract is not None:
        contract_path, write_pattern, reason = contract
        if contract_path.is_file():
            contract_source = contract_path.read_text(encoding="utf-8-sig")
            table_name = re.escape(domain)
            if re.search(write_pattern, contract_source, flags=re.IGNORECASE) and not re.search(
                rf"DELETE\s+FROM\s+[\"']?{table_name}[\"']?", contract_source, flags=re.IGNORECASE
            ):
                return {"update-delete-and-readback": reason}

    return {}


FILE_LIFECYCLE_CASES = (
    "path-and-default-selection",
    "nominal-bytes-and-metadata",
    "existing-missing-and-overwrite",
    "permissions-symlink-and-path-confinement",
    "partial-cancel-and-cleanup",
    "restart-reload-retention-and-corruption",
)


def file_lifecycle_not_applicable_cases(
    frozen_root: Path | None,
    source: str,
) -> dict[str, str]:
    """Return file-lifecycle cases absent from a frozen source's contract.

    The inventory intentionally includes source files that call a file API as
    part of validation or build tooling. Those calls are not durable product
    file writers and must not create six artificial lifecycle obligations.
    This allowlist is source-backed and exact; durable writers remain open
    until an executed differential proves them.
    """
    if frozen_root is None:
        return {}

    source_path = frozen_root / "src/slskd" / source
    if not source_path.is_file():
        return {}
    source_text = source_path.read_text(encoding="utf-8-sig", errors="ignore")

    if source.endswith("Search/API/Controllers/SearchActionsController.cs") and all(
        token in source_text
        for token in (
            "private const int PodDownloadChunkBytes = 2048",
            "private readonly IMeshContentFetcher _meshContentFetcher",
            'if (primarySource == "pod" && response.PodContentRef != null)',
            "System.IO.File.Create(localFilename)",
            "TryDeletePartialPodDownload(localFilename)",
        )
    ):
        return {
            case: (
                "Frozen SearchActionsController's only direct file writer is the "
                "optional PodContentRef fallback: it creates an incomplete file, "
                "fetches bounded mesh chunks, and deletes the partial on failure. "
                "slskR's materialized search-result model has no PodContentRef or "
                "primary-source field; its reachable search action delegates "
                "Soulseek downloads to the transfer writer, whose complete "
                "lifecycle is proven separately."
            )
            for case in FILE_LIFECYCLE_CASES
        }

    if source.endswith("Sharing/API/SharesController.cs") and all(
        token in source_text
        for token in (
            "var useHttpDownload = !string.IsNullOrWhiteSpace(ownerEndpoint) && !string.IsNullOrWhiteSpace(grant.ShareToken)",
            "using var fileStream = new System.IO.FileStream(filePath, FileMode.Create, FileAccess.Write, FileShare.None)",
            "CopyContentToFileWithLimitAsync(response.Content, fileStream",
            "TryDeletePartialBackfillFile(filePath)",
        )
    ):
        return {
            case: (
                "Frozen SharesController's direct writer belongs only to the "
                "cross-node HTTP backfill branch guarded by OwnerEndpoint and a "
                "share token. slskR's materialized share-grant contract has no "
                "owner endpoint or remote stream field; its backfill is a bounded "
                "local acknowledgement or transfer-queue delegation, so this "
                "controller-local HTTP file lifecycle is not an independent "
                "slskR contract."
            )
            for case in FILE_LIFECYCLE_CASES
        }

    if source.endswith("SongID/SongIdService.cs") and all(
        token in source_text
        for token in (
            "run.ArtifactDirectory",
            "File.WriteAllTextAsync(path, JsonSerializer.Serialize(entry), cancellationToken)",
            "Directory.CreateDirectory(workspace)",
            "private async Task RegisterCorpusEntryAsync(",
            "await PublishRunAsync(run)",
        )
    ):
        return {
            case: (
                "Frozen SongIdService's file calls belong to its optional external "
                "audio-analysis artifact/corpus pipeline. slskR materializes the "
                "bounded SongID run and persistence contract, reports analyzer "
                "artifacts as absent when those optional tools are unavailable, "
                "and keeps only transient normalization files; it does not expose "
                "the frozen artifact-directory or corpus-file layout as a local "
                "file contract."
            )
            for case in FILE_LIFECYCLE_CASES
        }

    reason: str | None = None
    if source == "Program.cs" and all(
        token in source_text
        for token in (
            "private static (string Filename, string Password) GenerateX509Certificate(",
            "IOFile.Copy(source, destination)",
            "private static void VerifyDirectory(",
        )
    ):
        reason = (
            "Frozen slskd Program owns startup command/bootstrap file operations: "
            "configuration seeding, optional certificate export, and directory "
            "writability probes. These are composition-root scaffolding; the "
            "runtime configuration, certificate, and durable transfer stores own "
            "the product file lifecycle."
        )
    elif source.endswith("PodCore/GoldStarClubService.cs") and all(
        token in source_text
        for token in (
            "private const string RevocationFileName = \"gold-star-club.revoked\"",
            "public Task RecordRevocationAsync(string peerId, CancellationToken ct = default)",
            "System.IO.File.WriteAllTextAsync(",
        )
    ):
        return {
            "partial-cancel-and-cleanup": (
                "Frozen GoldStarClubService writes only a small local membership-"
                "revocation marker; its cancellation token belongs to the marker "
                "operation, not to a caller-owned content transfer. The local "
                "revocation implementation publishes the marker atomically, so "
                "a cancelled write cannot become an externally visible partial "
                "download artifact."
            )
        }
    elif source.endswith("Mesh/Realm/Migration/RealmMigrationTool.cs") and all(
        token in source_text
        for token in (
            "public async Task<MigrationExportResult> ExportPodDataAsync(",
            "public async Task<MigrationImportResult> ImportPodDataAsync(",
            "public MigrationGuide GenerateMigrationGuide(",
        )
    ) and not any(
        candidate != source_path
        and re.search(
            r"(?:ExportPodDataAsync|ImportPodDataAsync|GenerateMigrationGuide)\s*\(",
            candidate.read_text(encoding="utf-8-sig", errors="ignore"),
        )
        for candidate in (frozen_root / "src/slskd").rglob("*.cs")
    ):
        return {
            case: (
                "Frozen RealmMigrationTool is registered as an unused migration "
                "utility but no production caller or route invokes its export, "
                "import, or guide methods; its files are not an observable product "
                "lifecycle."
            )
            for case in FILE_LIFECYCLE_CASES
        }
    elif source.endswith("Common/Validation/DirectoryExistsAttributes.cs") and (
        "ensureWriteable" in source_text
        and "File.WriteAllText" in source_text
        and "File.Delete" in source_text
    ):
        reason = (
            "Frozen source writes and immediately deletes a random writability "
            "probe; it does not persist product state or expose a file lifecycle contract."
        )
    elif source.endswith("Destinations/API/Controllers/DestinationsController.cs") and (
        "slskd-write-test-" in source_text
        and "File.WriteAllText" in source_text
        and "File.Delete" in source_text
    ):
        reason = (
            "Frozen destination validation writes and immediately deletes a "
            "temporary writability probe; it does not persist product state."
        )
    elif source.endswith("Common/CodeQuality/RegressionBuildTask.cs") and (
        "GenerateReports" in source_text
        and "regression-results-" in source_text
        and "benchmark-results-" in source_text
    ):
        reason = (
            "Frozen source generates build-time regression and benchmark reports; "
            "these diagnostic artifacts are not runtime product state."
        )
    elif source.endswith("Common/CodeQuality/RegressionHarness.cs") and (
        "GenerateCoverageReport" in source_text
        and "coverage-report-" in source_text
        and "coverage-summary-" in source_text
    ):
        reason = (
            "Frozen source generates regression-harness diagnostic reports; these "
            "build/test artifacts are not runtime product state."
        )
    elif source.endswith("Application.cs") and (
        "CacheBrowseResponse" in source_text
        and "browse.cache" in source_text
        and "File.Move" in source_text
    ):
        reason = (
            "Frozen source writes a derived browse-response cache and rebuilds it "
            "from shares; the cache path is not a public file contract and does "
            "not carry product state across restart."
        )
    elif source.endswith("Common/Dumper.cs") and (
        "Path.GetTempPath()" in source_text
        and ("DumpType.Full" in source_text or "collect --process-id" in source_text)
    ):
        reason = (
            "Frozen source stages a one-shot diagnostic memory dump in a temporary "
            "file for the HTTP response; the controller owns response cleanup and "
            "the dump is not durable application state."
        )
    elif source.endswith("Relay/API/Controllers/RelayController.cs") and (
        "share_" in source_text
        and "HandleShareUploadAsync" in source_text
        and "File.Delete(temp)" in source_text
    ):
        reason = (
            "Frozen relay-controller source stages an uploaded share database in a "
            "temporary file, consumes it into the relay projection, and deletes "
            "the staging file; it does not own a durable file lifecycle."
        )
    elif source.endswith("Streaming/MeshStreamService.cs") and (
        "slskdn-mesh-preview-" in source_text
        and "FileOptions.DeleteOnClose" in source_text
        and "FetchVerifiedThenCopyAsync" in source_text
    ):
        reason = (
            "Frozen mesh streaming stages verified preview bytes in a temporary "
            "DeleteOnClose file while copying the HTTP stream; it does not persist "
            "content or expose a restart/reload file contract."
        )
    elif source.endswith("Files/FileService.cs") and all(
        token in source_text
        for token in (
            "public virtual Stream CreateFile(",
            "public virtual string MoveFile(",
            "FileMode.Create",
        )
    ):
        return {
            "partial-cancel-and-cleanup": (
                "Frozen FileService creates or moves caller-selected files directly; "
                "those APIs have no cancellation-owned staging or partial-transfer "
                "contract, so cleanup belongs to the transfer caller."
            ),
            "restart-reload-retention-and-corruption": (
                "Frozen FileService is a stateless filesystem helper with no persisted "
                "service state or reload path; retention and corruption recovery belong "
                "to the caller-owned file or state store."
            ),
        }
    elif source.endswith("Transfers/MultiSource/Tracing/SwarmEventStore.cs") and all(
        token in source_text
        for token in (
            "public class SwarmEventStore",
            "File.AppendAllTextAsync(path, json",
            "RotateIfNeeded(path)",
            "public async Task<IReadOnlyList<SwarmEvent>> ReadAsync(",
        )
    ):
        return {
            case: (
                "Frozen SwarmEventStore is an internal JSONL implementation behind "
                "the trace-summary service; the log filename, append/rotation files, "
                "symlink behavior, and physical retention are not an independent "
                "public contract. slskR exposes the bounded swarm-job trace summary "
                "through its transfer state instead of promising this private log "
                "layout."
            )
            for case in FILE_LIFECYCLE_CASES
        }
    elif source.endswith("Swarm/SwarmDownloadOrchestrator.cs") and (
        "slskdn-swarm" in source_text
        and "ProcessJob" in source_text
        and not any(
            "SwarmDownloadOrchestrator" in candidate.read_text(
                encoding="utf-8-sig", errors="ignore"
            )
            and re.search(
                r"Add(?:HostedService|Singleton|Transient|Scoped)\s*<[^>]*SwarmDownloadOrchestrator",
                candidate.read_text(encoding="utf-8-sig", errors="ignore"),
            )
            for candidate in (frozen_root / "src/slskd").rglob("*.cs")
            if candidate != source_path
        )
    ):
        reason = (
            "Frozen source contains an unregistered experimental swarm background "
            "orchestrator whose chunk and output files are temporary staging; the "
            "registered multisource transfer service owns the observable contract."
        )
    elif source.endswith("VirtualSoulfind/DisasterMode/MeshTransferService.cs") and (
        "File.Exists(status.TargetPath)" in source_text
        and "File.OpenRead(status.TargetPath)" in source_text
        and not re.search(
            r"File\.(?:Create|WriteAll|Move|Copy|AppendAll)|FileStream\s*\(",
            source_text,
        )
    ):
        return {
            case: (
                "Frozen MeshTransferService only checks and reads an already-created "
                "target file; it does not own a file writer, replacement, cleanup, "
                "or restart/reload lifecycle."
            )
            for case in FILE_LIFECYCLE_CASES
        }

    if source.endswith("Core/API/Controllers/OptionsController.cs") and (
        (
            "IOFile.WriteAllText(tempFile, yaml)" in source_text
            and "IOFile.Move(tempFile, Program.ConfigurationFile" in source_text
        )
        or "IOFile.WriteAllText(Program.ConfigurationFile, yaml)" in source_text
    ) and "CancellationToken" not in source_text:
        return {
            "partial-cancel-and-cleanup": (
                "Frozen OptionsController performs a synchronous validated YAML write and "
                "has no cancellation or caller-visible partial-transfer staging contract; "
                "backup, replacement, and reload cases cover its durable file behavior."
            )
        }

    injected_storage_sources = {
        "Common/Moderation/PeerReputationStore.cs": (
            "public PeerReputationStore(",
            "_storagePath = storagePath",
        ),
        "Core/Security/JwtRevocationStore.cs": (
            "public JwtRevocationStore(string path)",
            "_path = path",
        ),
        "Integrations/MusicBrainz/Overlay/MusicBrainzOverlayService.cs": (
            "MusicBrainzOverlayService(ILogger<MusicBrainzOverlayService> logger, string storagePath)",
            "_storagePath = storagePath",
        ),
        "Integrations/MusicBrainz/Radar/ArtistReleaseRadarService.cs": (
            "ArtistReleaseRadarService(ILogger<ArtistReleaseRadarService> logger, string storagePath)",
            "_storagePath = storagePath",
        ),
        "Mesh/Realm/SubjectIndex/RealmSubjectIndexService.cs": (
            "string storagePath,",
            "_storagePath = storagePath",
        ),
        "Opinions/OpinionService.cs": (
            "OpinionService(ILogger<OpinionService> logger, string storagePath)",
            "this.storagePath = storagePath",
        ),
        "QuarantineJury/QuarantineJuryService.cs": (
            "public QuarantineJuryService(ILogger<QuarantineJuryService> logger, string storagePath)",
            "_storagePath = storagePath",
        ),
        "SourceFeeds/SourceFeedImportService.cs": (
            "string storagePath)",
            "_storagePath = storagePath",
        ),
    }
    injected_storage_tokens = injected_storage_sources.get(source)
    if (
        injected_storage_tokens is not None
        and "AtomicFileWriter." in source_text
        and all(token in source_text for token in injected_storage_tokens)
    ):
        return {
            "path-and-default-selection": (
                "Frozen source receives its storage path from the composition root "
                "and persists only to that injected path; default-path selection is "
                "owned by the caller/configuration layer, while the store's atomic "
                "write and reload behavior are covered separately."
            )
        }

    if source.endswith("VirtualSoulfind/v2/Resolution/SimpleResolver.cs") and all(
        token in source_text
        for token in (
            "private readonly ConcurrentDictionary<string, PlanExecutionState> _executions = new();",
            "Directory.CreateDirectory(downloadDir);",
            'var tmpPath = Path.Combine(downloadDir, $"vs2_',
            "File.WriteAllBytesAsync(tmpPath, reply.Payload, cancellationToken)",
            "await using (var fs = File.Create(tmpPath))",
        )
    ):
        return {
            "path-and-default-selection": (
                "Frozen SimpleResolver selects its configured DownloadDirectory "
                "or the system temporary directory and creates the staging root; "
                "it does not expose a caller-selected product destination."
            ),
            "nominal-bytes-and-metadata": (
                "Frozen SimpleResolver writes backend results only to a GUID-named "
                "temporary staging path and returns that path to the execution "
                "state; the resolver does not publish a durable product file or "
                "define an independent metadata contract."
            ),
            "existing-missing-and-overwrite": (
                "Frozen SimpleResolver gives every fetched staging artifact a new "
                "GUID-derived filename under the selected download directory; it "
                "does not select, replace, or overwrite an existing destination."
            ),
            "permissions-symlink-and-path-confinement": (
                "Frozen SimpleResolver accepts only the configured download-directory "
                "root and generates the leaf filename internally; it has no caller-"
                "selected destination or independent symlink/path-confinement contract."
            ),
            "partial-cancel-and-cleanup": (
                "Frozen SimpleResolver's fetched files are temporary backend "
                "staging artifacts, not caller-owned completed files; execution "
                "cancellation returns a cancelled in-memory state and has no "
                "separate durable partial-file contract."
            ),
            "restart-reload-retention-and-corruption": (
                "Frozen SimpleResolver stores execution state only in its in-memory "
                "ConcurrentDictionary and returns temporary fetched paths; there is "
                "no persisted state or reload path for this staging writer."
            ),
        }

    path_selected_by_caller = {
        "Bootstrap/StartupFileSystem.cs": (
            "GenerateX509Certificate(",
            "filename = Path.Combine(baseDirectory, filename)",
            "AtomicFileWriter.WriteAllBytes(",
        ),
        "DhtRendezvous/DhtRendezvousService.cs": (
            "Path.Combine(Program.AppDirectory, \"dht_nodes.bin\")",
            "AtomicFileWriter.WriteAllBytesAsync(",
            "File.ReadAllBytesAsync(",
        ),
        "Files/FileService.cs": (
            "Creates a new file with the specified fully qualified",
            "public virtual Stream CreateFile(string filename",
            "public virtual string MoveFile(string sourceFilename",
        ),
        "Identity/ProfileService.cs": (
            "var dataDir = Program.AppDirectory",
            "Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), \"slskd\")",
            "return Path.Combine(dataDir, \"peer-profile.json\")",
        ),
        "Common/Security/API/SecurityController.cs": (
            "var configFile = Program.ConfigurationFile",
            "IOFile.WriteAllText(tempFile",
            "IOFile.Move(tempFile, configFile",
        ),
        "Mesh/Overlay/KeyStore.cs": (
            "var path = options.KeyPath",
            "File.Move(tempPath, path",
        ),
        "Mesh/Realm/Migration/RealmMigrationTool.cs": (
            "ExportPodDataAsync(",
            "Directory.CreateDirectory(exportPath)",
            "Path.Combine(exportPath, \"migration-manifest.json\")",
        ),
        "Jobs/Manifests/JobManifestService.cs": (
            "jobsRoot = Path.Combine(AppPathResolver.GetWriteBaseDirectory(Program.AppDirectory, Program.DefaultAppDirectory), \"jobs\")",
            "Path.Combine(folder, $\"{manifest.JobId}.yaml\")",
            "AtomicFileWriter.WriteAllTextAsync(path, yaml",
        ),
        "Transfers/AutoReplace/AutoReplaceBackgroundService.cs": (
            "StateFilePath = Path.Combine(Program.AppDirectory, StateFileName)",
            "AtomicFileWriter.WriteAllText(StateFilePath, json)",
        ),
        "Transfers/MultiSource/ContentVerificationService.cs": (
            "Program.DefaultAppDirectory",
            "verification-probe-budget.json",
            "AtomicFileWriter.WriteAllText(path,",
        ),
        "Transfers/MultiSource/Tracing/SwarmEventStore.cs": (
            "AppPathResolver.GetWriteBaseDirectory(Program.AppDirectory, Program.DefaultAppDirectory)",
            '"logs", "sessions"',
            "File.AppendAllTextAsync(path, json",
        ),
        "Transfers/MultiSource/MultiSourceDownloadService.cs": (
            "request.OutputPath",
            "FileStream(",
            "FileMode.Create",
        ),
        "SourceFeeds/SpotifyConnectionService.cs": (
            "_storagePath = Path.Combine(",
            "global::slskd.Program.DefaultAppDirectory",
            '"source-feeds",',
            "AtomicFileWriter.WriteAllText(",
        ),
        "VirtualSoulfind/v2/Resolution/SimpleResolver.cs": (
            "_options.CurrentValue.DownloadDirectory",
            "ResolveOptionalAppRelativePath(",
            "Path.GetTempPath()",
        ),
    }
    path_selected_tokens = path_selected_by_caller.get(source)
    if path_selected_tokens is not None and all(
        token in source_text for token in path_selected_tokens
    ):
        return {
            "path-and-default-selection": (
                "Frozen source receives its export/configuration/key path from "
                "the caller or options layer and does not define the product's "
                "default path; the writer's byte, replacement, and cleanup "
                "behavior remains covered by its other lifecycle cases."
            )
        }

    if source.endswith("Transfers/Downloads/DownloadService.cs") and all(
        token in source_text
        for token in (
            "FileOptions.DeleteOnClose",
            "PlanIncompleteOutput(",
            "EnrichTransferMetadata(",
        )
    ):
        return {
            case: (
                "Frozen DownloadService's source-local file operations are a "
                "temporary directory writability probe and read-only metadata "
                "enrichment; the durable transfer stream, cancellation cleanup, "
                "and persisted transfer state are owned by the Soulseek transfer "
                "runtime and the separate transfer-state store."
            )
            for case in (
                "permissions-symlink-and-path-confinement",
                "partial-cancel-and-cleanup",
                "restart-reload-retention-and-corruption",
            )
        }

    if source.endswith("Common/Security/SecureFileWriter.cs") and all(
        token in source_text
        for token in (
            "public static FileStream Open(string path, string trustedRoot)",
            "OpenTruncate",
            "OpenNoFollow",
        )
    ):
        return {
            "partial-cancel-and-cleanup": (
                "Frozen SecureFileWriter only opens a confined, truncated output "
                "handle; the transfer caller owns cancellation and removal of any "
                "partial output."
            ),
            "restart-reload-retention-and-corruption": (
                "Frozen SecureFileWriter has no persisted state or reload path; it "
                "only creates a fresh output handle for its caller."
            ),
        }

    if source.endswith("DhtRendezvous/Security/CertificateManager.cs") and all(
        token in source_text
        for token in (
            "WriteCertificateAtomically",
            "TryDeleteTempFile",
            "File.Move(tempPath, path, overwrite: true)",
        )
    ):
        return {
            "partial-cancel-and-cleanup": (
                "Frozen CertificateManager writes certificates synchronously "
                "through a guarded temporary-file publish and deletes that "
                "temporary file on failure; it has no cancellation-owned "
                "partial-transfer contract."
            )
        }

    return {case: reason for case in FILE_LIFECYCLE_CASES} if reason else {}


def security_not_applicable_cases(
    frozen_root: Path | None,
    source: str,
) -> dict[str, str]:
    """Classify security cases that the frozen component does not own.

    The frozen API-key handler and SecurityService authenticate credentials,
    issue/revoke tokens, and enforce caller ranges.  They contain no request
    quota or lockout behavior; the frozen session controller owns that
    separate contract.  Keep this allowlist exact so unrelated security
    components remain open until their own behavior is proven.
    """
    if frozen_root is None:
        return {}
    path = frozen_root / "src/slskd" / source
    source_text = path.read_text(encoding="utf-8-sig")

    declaration_cases = (
        "activation-default-and-profile",
        "accepted-nominal-input",
        "rejected-malicious-and-boundary-input",
        "quota-time-lockout-and-concurrency",
        "secret-logging-and-privacy-output",
        "restart-rotation-and-recovery",
    )
    non_security_helpers = {
        "Common/RateLimiter.cs": (
            ("public class RateLimiter", "Ensures a minimum interval", "Staged"),
            "Frozen RateLimiter is a timer/debounce helper used by search and transfer state updates; it owns no authentication, authorization, input-rejection, secret, quota, or security-state contract.",
        ),
        "Common/TokenBucket.cs": (
            ("public interface ITokenBucket", "public class TokenBucket", "Task<int> GetAsync"),
            "Frozen TokenBucket is a byte-bandwidth governor used by UploadGovernor; it owns transfer-speed accounting, not authentication, authorization, attack rejection, secret handling, or security-state recovery.",
        ),
    }
    non_security_helper = non_security_helpers.get(source)
    if non_security_helper is not None:
        required_tokens, reason = non_security_helper
        if all(token in source_text for token in required_tokens):
            return {case: reason for case in declaration_cases}
        return {}

    composed_security_helpers = {
        "Common/Security/BindExposureAnalyzer.cs": (
            ("public static class BindExposureAnalyzer", "AnalyzeWebBinding", "IsRemoteReachable"),
            "Frozen BindExposureAnalyzer is a stateless bind-address projection "
            "used by startup hardening; startup configuration owns the externally "
            "observable exposure decision.",
        ),
        "Common/Security/BucketPadder.cs": (
            ("public class BucketPadder", "IMessagePadder", "byte[] Unpad"),
            "Frozen BucketPadder is a stateless message-padding dependency of "
            "PrivacyLayer; the composed privacy transport owns the observable "
            "wire and lifecycle contract.",
        ),
        "Common/Security/IdentitySeparationEnforcer.cs": (
            ("public static class IdentitySeparationEnforcer", "IsValidIdentityFormat", "SanitizePodPeerId"),
            "Frozen IdentitySeparationEnforcer is a stateless identity-format "
            "helper used by pod services and the identity validator; those callers "
            "own the externally observable identity contract.",
        ),
        "Common/Security/IpRangeClassifier.cs": (
            ("public static class IpRangeClassifier", "Classify", "IsSafeForTunneling"),
            "Frozen IpRangeClassifier is a stateless address-classification "
            "primitive composed into endpoint, DNS, and outbound-URI policies; "
            "those policies own the observable rejection contract.",
        ),
        "Common/Security/LoggingSanitizer.cs": (
            ("public static class LoggingSanitizer", "SanitizeSensitiveData", "SafeContext"),
            "Frozen LoggingSanitizer is a stateless formatting helper; each "
            "security, transport, and controller caller owns the emitted log or "
            "response contract rather than this helper owning independent state.",
        ),
        "Common/Security/Obfs4VersionChecker.cs": (
            ("public sealed class Obfs4VersionChecker", "RunVersionCheckAsync", "CancellationToken"),
            "Frozen Obfs4VersionChecker is an executable-availability dependency "
            "of Obfs4Transport; transport selection owns activation, rejection, "
            "secret, and recovery behavior.",
        ),
        "Common/Security/RandomJitterObfuscator.cs": (
            ("public class RandomJitterObfuscator", "ITimingObfuscator", "GetNextDelayAsync"),
            "Frozen RandomJitterObfuscator is a stateless timing dependency of "
            "PrivacyLayer; the composed privacy transport owns activation and "
            "wire behavior.",
        ),
        "Common/Security/SecurityUtils.cs": (
            ("public static class SecurityUtils", "ConstantTimeEquals", "GenerateSecureRandomBytes"),
            "Frozen Common SecurityUtils is a stateless cryptographic primitive "
            "composed into authentication, token, payload, and certificate "
            "callers; those concrete controls own the observable security "
            "contract.",
        ),
        "Common/Security/TimedBatcher.cs": (
            ("public class TimedBatcher", "IMessageBatcher", "GetNextBatchAsync"),
            "Frozen TimedBatcher is a timed message-batching dependency of "
            "PrivacyLayer; the composed privacy transport owns its security and "
            "wire lifecycle.",
        ),
        "DhtRendezvous/Security/PathGuard.cs": (
            ("public static partial class PathGuard", "CommonPathGuard", "ValidatePeerPath"),
            "Frozen DhtRendezvous PathGuard is a stateless wrapper over the common "
            "path policy; the DHT transfer and message callers own the observable "
            "path-rejection contract.",
        ),
        "Mesh/Transport/EndpointCertificatePinValidator.cs": (
            ("public static class EndpointCertificatePinValidator", "Validate", "trustedPins"),
            "Frozen EndpointCertificatePinValidator is a stateless adapter over "
            "the mesh certificate-pin policy; CertificatePinManager and transport "
            "callers own pin activation, rotation, and failure behavior.",
        ),
        "Transfers/ScheduledRateLimitService.cs": (
            ("public class ScheduledRateLimitService", "GetEffectiveUploadSpeedLimit", "IsNightTime"),
            "Frozen ScheduledRateLimitService schedules transfer bandwidth limits; "
            "it owns no authentication, authorization, attack rejection, secret, "
            "or security-state lifecycle.",
        ),
        "VirtualSoulfind/ShadowIndex/ShardEvictionPolicy.cs": (
            ("public static class ShardEvictionPolicy", "IsExpired", "TrimShard"),
            "Frozen ShardEvictionPolicy is a stateless cache-retention policy for "
            "the ShadowIndex; cache ownership, not a security-control lifecycle, "
            "owns its observable behavior.",
        ),
    }
    composed_security_helper = composed_security_helpers.get(source)
    if composed_security_helper is not None:
        required_tokens, reason = composed_security_helper
        if all(token in source_text for token in required_tokens):
            return {case: reason for case in declaration_cases}
        return {}

    route_authorization_filters = {
        "Common/Authentication/RequireScopeAttribute.cs": (
            ("IAuthorizationFilter", "OnAuthorization", "Scope"),
            "Frozen RequireScopeAttribute is an authorization-pipeline filter; its "
            "scope grant/deny behavior is exercised by the exhaustive route "
            "authorization matrix, not by an independent persisted security-control lifecycle.",
        ),
        "Common/Authentication/ScopedApiKeyDenyByDefaultFilter.cs": (
            ("IAuthorizationFilter", "IOrderedFilter", "scope_mapping_required"),
            "Frozen ScopedApiKeyDenyByDefaultFilter is an authorization-pipeline "
            "filter; its scoped-principal deny behavior is exercised by the "
            "exhaustive route authorization matrix, not by an independent "
            "persisted security-control lifecycle.",
        ),
        "PodCore/API/PodApiAuthorizer.cs": (
            ("public static class PodApiAuthorizer", "GetAuthenticatedPeerId", "GetAccessAsync"),
            "Frozen PodApiAuthorizer is a static access projection used by the pod "
            "controllers; authenticated identity, membership, ban, and moderator "
            "decisions are exercised through those controller routes rather than "
            "forming an independent persisted security-control lifecycle.",
        ),
    }
    route_filter = route_authorization_filters.get(source)
    if route_filter is not None:
        required_tokens, reason = route_filter
        if all(token in source_text for token in required_tokens):
            return {case: reason for case in declaration_cases}
        return {}

    composed_security_helpers = {
        "Core/Security/AntiforgeryCookieRecovery.cs": (
            ("public static class AntiforgeryCookieRecovery", "TryGetAndStoreTokens", "ClearKnownCookies"),
            "Frozen AntiforgeryCookieRecovery is a static cookie-recovery helper; "
            "the CSRF authorization filter owns the externally observable request "
            "validation, stale-cookie recovery, and response contract.",
        ),
    }
    composed_security_helper = composed_security_helpers.get(source)
    if composed_security_helper is not None:
        required_tokens, reason = composed_security_helper
        if all(token in source_text for token in required_tokens):
            return {case: reason for case in declaration_cases}
        return {}

    if source == "Core/Security/ValidateCsrfForCookiesOnlyAttribute.cs":
        required_tokens = (
            "IAsyncAuthorizationFilter",
            "SafeMethods",
            "OnAuthorizationAsync",
            "ValidateRequestAsync",
        )
        if all(token in source_text for token in required_tokens):
            return {
                "quota-time-lockout-and-concurrency": (
                    "Frozen CSRF authorization is a request-validation filter; it "
                    "owns no request quota, lockout, or concurrent-state budget."
                )
            }
        return {}

    stateless_security_cases = {
        "Common/Security/PathGuard.cs": (
            (
                "public static partial class PathGuard",
                "PathViolationType",
                "NormalizeAbsolutePathWithinRoots",
            ),
            "Frozen Common PathGuard is a stateless path-validation primitive; it "
            "owns no quota, lockout, or restart state. Its path-rejection contract "
            "is exercised by the confined file and transfer callers.",
        ),
        "Common/Security/SecureFileWriter.cs": (
            ("public static class SecureFileWriter", "OpenNoFollow", "OpenTruncate"),
            "Frozen SecureFileWriter is a stateless confined-file primitive; it "
            "owns no quota, lockout, or restart state. Its confinement contract is "
            "exercised by the transfer caller.",
        ),
        "Common/Security/OutboundUriGuard.cs": (
            (
                "public static class OutboundUriGuard",
                "CheckAsync",
                "CreateNoRedirectHandler",
            ),
            "Frozen OutboundUriGuard is a stateless SSRF and redirect-policy "
            "primitive; it owns no quota, lockout, or restart state. Its rejection "
            "contract is exercised by the guarded outbound clients.",
        ),
        "Identity/PeerEndpointPolicy.cs": (
            (
                "public static class PeerEndpointPolicy",
                "IsLeakyAddress",
                "IpRangeClassifier.IsBlocked",
            ),
            "Frozen PeerEndpointPolicy is a stateless publication filter; it owns "
            "no quota, lockout, or restart state. Its endpoint rejection contract "
            "is exercised by the peer-profile projection.",
        ),
        "Common/Security/HardeningValidator.cs": (
            (
                "public static class HardeningValidator",
                "RuleAuthDisabledNonLoopback",
                "RuleWeakMetricsPassword",
            ),
            "Frozen HardeningValidator is a startup configuration validator; it "
            "owns no request quota, lockout, or concurrent-state budget. Its "
            "startup rejection and revalidation behavior is exercised by the "
            "controller configuration differential.",
        ),
        "Common/Security/SecurityServices.cs": (
            (
                "public sealed class SecurityServices",
                "GetAggregateStats",
                "ReportSecurityEvent",
            ),
            "Frozen SecurityServices aggregates independently-owned security "
            "services and owns no request quota or lockout budget. Its aggregate "
            "projection, trust decision, and event forwarding behavior is "
            "exercised by the runtime security differential.",
        ),
        "DhtRendezvous/Security/MessageValidator.cs": (
            (
                "public static partial class MessageValidator",
                "ValidateMeshHello",
                "ValidatePing",
            ),
            "Frozen MessageValidator is a stateless overlay-input validator; it "
            "owns no request quota, lockout, or restart state. Its rejection "
            "contract is exercised by the typed overlay message boundary.",
        ),
        "DhtRendezvous/Security/SecureMessageFramer.cs": (
            (
                "public sealed class SecureMessageFramer",
                "ReadPayloadAsync",
                "MaxMessageSize",
            ),
            "Frozen SecureMessageFramer owns only per-connection framing state; it "
            "has no persisted restart state or independent request quota. Its "
            "bounded framing contract is exercised by the overlay framer.",
        ),
        "DhtRendezvous/Security/CertificateManager.cs": (
            ("public sealed class CertificateManager", "CertificatePinStore", "WriteCertificateAtomically"),
            "Frozen CertificateManager owns certificate identity and pin state but "
            "no request quota or lockout budget; those cases belong to the overlay "
            "connection controls.",
        ),
        "Solid/SolidFetchPolicy.cs": (
            ("public sealed class SolidFetchPolicy", "ValidateAsync", "AllowedHosts"),
            "Frozen SolidFetchPolicy is a request-scoped SSRF/host policy with only "
            "an expiring DNS cache; it owns no request quota or persisted restart "
            "state. Its allow, deny, and privacy contract is exercised by the "
            "Solid WebID resolver.",
        ),
    }
    stateless_security = stateless_security_cases.get(source)
    if stateless_security is not None:
        required_tokens, reason = stateless_security
        if all(token in source_text for token in required_tokens):
            cases = {"quota-time-lockout-and-concurrency": reason}
            if source in {
                "Common/Security/PathGuard.cs",
                "Common/Security/SecureFileWriter.cs",
                "Common/Security/OutboundUriGuard.cs",
                "Identity/PeerEndpointPolicy.cs",
                "DhtRendezvous/Security/MessageValidator.cs",
                "DhtRendezvous/Security/SecureMessageFramer.cs",
                "Solid/SolidFetchPolicy.cs",
            }:
                cases["restart-rotation-and-recovery"] = reason
            return cases
        return {}

    lifecycle_boundaries = {
        "Common/Security/AnonymityTransportSelector.cs": (
            ("public class AnonymityTransportSelector", "InitializeTransports", "SelectTransportAsync"),
            {"quota-time-lockout-and-concurrency", "restart-rotation-and-recovery"},
            "Frozen AnonymityTransportSelector owns transport selection and failover, but no request quota or persisted/reload state; those contracts belong to the selected transport and policy controls.",
        ),
        "Common/Security/ContentSafety.cs": (
            ("public static class ContentSafety", "VerifyFileAsync", "VerifyHeader"),
            {"quota-time-lockout-and-concurrency", "restart-rotation-and-recovery"},
            "Frozen ContentSafety is a stateless file-signature classifier; it owns no request budget or persisted state. Download ownership supplies the bounded input and lifecycle.",
        ),
        "DhtRendezvous/Security/ContentSafety.cs": (
            ("public static class ContentSafety", "VerifyHeader", "IsExecutable"),
            {"quota-time-lockout-and-concurrency", "restart-rotation-and-recovery"},
            "Frozen DHT ContentSafety is a stateless header classifier; it owns no request budget or persisted state. The overlay/content caller owns those boundaries.",
        ),
        "Common/Security/DirectTransport.cs": (
            ("public class DirectTransport", "ConnectAsync", "GetStatus"),
            {"quota-time-lockout-and-concurrency", "restart-rotation-and-recovery"},
            "Frozen DirectTransport opens direct sockets and reports availability; it owns no request quota or persisted/reload state.",
        ),
        "Common/Security/HttpTunnelTransport.cs": (
            ("public class HttpTunnelTransport", "ConnectAsync", "GetStatus"),
            {"quota-time-lockout-and-concurrency", "restart-rotation-and-recovery"},
            "Frozen HttpTunnelTransport opens an optional tunnel and reports availability; it owns no request quota or persisted/reload state.",
        ),
        "Common/Security/I2PTransport.cs": (
            ("public class I2PTransport", "ConnectAsync", "GetStatus"),
            {"quota-time-lockout-and-concurrency", "restart-rotation-and-recovery"},
            "Frozen I2PTransport opens an optional SAM tunnel and reports availability; it owns no request quota or persisted/reload state.",
        ),
        "Common/Security/MeekTransport.cs": (
            ("public class MeekTransport", "ConnectAsync", "GetStatus"),
            {"quota-time-lockout-and-concurrency", "restart-rotation-and-recovery"},
            "Frozen MeekTransport opens an optional obfuscated tunnel and reports availability; it owns no request quota or persisted/reload state.",
        ),
        "Common/Security/Obfs4Transport.cs": (
            ("public class Obfs4Transport", "ConnectAsync", "GetStatus", "StartObfs4ProxyAsync"),
            {"quota-time-lockout-and-concurrency", "restart-rotation-and-recovery"},
            "Frozen Obfs4Transport manages an optional proxy process and reports availability; request budgeting and durable restart state belong to its callers and process supervisor.",
        ),
        "Common/Security/RelayOnlyTransport.cs": (
            ("public class RelayOnlyTransport", "ConnectAsync", "GetStatus"),
            {"quota-time-lockout-and-concurrency", "restart-rotation-and-recovery"},
            "Frozen RelayOnlyTransport selects a relay stream and reports availability; it owns no request quota or persisted/reload state.",
        ),
        "Common/Security/TorSocksTransport.cs": (
            ("public class TorSocksTransport", "ConnectAsync", "GetStatus"),
            {"quota-time-lockout-and-concurrency", "restart-rotation-and-recovery"},
            "Frozen TorSocksTransport opens an optional SOCKS tunnel and reports availability; it owns no request quota or persisted/reload state.",
        ),
        "Common/Security/WebSocketTransport.cs": (
            ("public class WebSocketTransport", "ConnectAsync", "GetStatus"),
            {"quota-time-lockout-and-concurrency", "restart-rotation-and-recovery"},
            "Frozen WebSocketTransport opens an optional tunnel and reports availability; it owns no request quota or persisted/reload state.",
        ),
        "Common/Security/SecurityEventSink.cs": (
            ("public sealed class SecurityEventAggregator", "ConcurrentQueue<SecurityEvent>", "MaxEvents"),
            {"restart-rotation-and-recovery"},
            "Frozen SecurityEventAggregator retains a bounded in-memory event queue and counters only; it has no persisted/reload or rotation file contract.",
        ),
        "Common/Security/SecurityHealthCheck.cs": (
            ("public sealed class SecurityHealthCheck", "IHealthCheck", "CheckHealthAsync"),
            {"quota-time-lockout-and-concurrency", "restart-rotation-and-recovery"},
            "Frozen SecurityHealthCheck is an aggregate health projection over independently-owned controls; it owns no request budget or persisted/reload state.",
        ),
        "Common/Security/SecurityMiddleware.cs": (
            ("public sealed class SecurityMiddleware", "InvokeAsync", "PathGuard.ContainsTraversal"),
            {"quota-time-lockout-and-concurrency", "restart-rotation-and-recovery"},
            "Frozen SecurityMiddleware is a request-pipeline adapter; quota state belongs to NetworkGuard/ViolationTracker and the middleware owns no persisted/reload state.",
        ),
        "Security/CompositeSecurityPolicy.cs": (
            ("public class CompositeSecurityPolicy", "EvaluateAsync", "short-circuits on deny"),
            {"quota-time-lockout-and-concurrency", "restart-rotation-and-recovery"},
            "Frozen CompositeSecurityPolicy only composes independently-owned policy decisions; it owns no request budget or persisted/reload state.",
        ),
    }
    lifecycle_boundary = lifecycle_boundaries.get(source)
    if lifecycle_boundary is not None:
        required_tokens, cases, reason = lifecycle_boundary
        if all(token in source_text for token in required_tokens):
            return {case: reason for case in cases}
        return {}

    declaration_contracts = {
        "Common/Authentication/AuthPolicy.cs": (
            ("public static class AuthPolicy", "public const string"),
            "Frozen AuthPolicy is a constant-name declaration; executable authentication behavior is owned by its handlers and authorization policy.",
        ),
        "Common/Authentication/AuthRole.cs": (
            ("public static class AuthRole", "public const string"),
            "Frozen AuthRole is a constant-name declaration; executable authorization behavior is covered by the route authorization matrix.",
        ),
        "Common/Authentication/Role.cs": (
            ("public enum Role", "ReadOnly", "Administrator"),
            "Frozen Role is an enum declaration; executable authorization behavior is covered by the route authorization matrix.",
        ),
        "Common/Exceptions/UnauthorizedException.cs": (
            ("public class UnauthorizedException", ": SlskdException"),
            "Frozen UnauthorizedException only carries an exception type and constructors; executable security behavior is owned by the caller and middleware.",
        ),
        "Common/Security/API/SecurityRequests.cs": (
            ("Request/response models are defined in SecurityController.cs",),
            "Frozen SecurityRequests is an intentionally empty placeholder; request validation and security behavior are owned by SecurityController and its models.",
        ),
        "Common/Security/ObfuscatedTransportMode.cs": (
            ("public enum ObfuscatedTransportMode", "Direct", "Obfs4"),
            "Frozen ObfuscatedTransportMode is an enum declaration; executable transport behavior is owned by the concrete transport implementations.",
        ),
        "Common/Validation/X509CertificateAttribute.cs": (
            ("public class X509CertificateAttribute", "ValidationAttribute", "X509.TryValidate"),
            "Frozen X509CertificateAttribute is a data-annotation adapter around X509.TryValidate; configuration binding owns the externally observable certificate validation contract.",
        ),
    }
    contract = declaration_contracts.get(source)
    if contract is not None:
        required_tokens, reason = contract
        if all(token in source_text for token in required_tokens):
            return {case: reason for case in declaration_cases}
        return {}

    data_contract_interfaces = {
        "Common/Security/IAnonymityTransport.cs": (
            "IAnonymityTransport",
            "AnonymityTransportStatus",
        ),
        "Common/Security/ICoverTrafficGenerator.cs": (
            "ICoverTrafficGenerator",
            "CoverTrafficStats",
        ),
        "Common/Security/IMessageBatcher.cs": (
            "IMessageBatcher",
            "BatchedMessage",
        ),
        "Common/Security/IPrivacyLayer.cs": (
            "IPrivacyLayer",
            "PrivacyStatistics",
        ),
    }
    data_contract = data_contract_interfaces.get(source)
    if data_contract is not None:
        declaration_source = re.sub(r"//[^\n]*|/\*.*?\*/", "", source_text, flags=re.DOTALL)
        interface_name, data_name = data_contract
        classes = set(
            re.findall(
                r"\b(?:public|internal)\s+(?:sealed\s+)?class\s+(\w+)",
                declaration_source,
            )
        )
        if (
            re.search(rf"\binterface\s+{re.escape(interface_name)}\b", declaration_source)
            and data_name in classes
            and classes <= {data_name}
            and not re.search(r"\b(?:record|struct)\s+\w+", declaration_source)
        ):
            reason = (
                "Frozen source contains only a security interface and its data-contract "
                "types; concrete implementations own the executable security-control lifecycle."
            )
            return {case: reason for case in declaration_cases}
        return {}

    interface_only_sources = {
        "Common/Security/IAnonymityTransportSelector.cs",
        "Common/Security/IDnsSecurityService.cs",
        "Common/Security/IMessagePadder.cs",
        "Common/Security/IObfs4VersionChecker.cs",
        "Common/Security/ITimingObfuscator.cs",
        "Solid/ISolidFetchPolicy.cs",
    }
    if source in interface_only_sources:
        declaration_source = re.sub(r"//[^\n]*|/\*.*?\*/", "", source_text, flags=re.DOTALL)
        if (
            re.search(r"\binterface\s+\w+", declaration_source)
            and not re.search(r"\b(?:class|record|struct|enum)\s+\w+", declaration_source)
        ):
            reason = (
                "Frozen source defines an interface-only security abstraction; concrete "
                "implementations own the executable security-control lifecycle."
            )
            return {case: reason for case in declaration_cases}
        return {}

    pure_contracts = {
        "Common/Security/API/SecurityModels.cs": (
            ("public sealed class BanIpRequest", "public sealed class SecurityDashboard"),
            "Frozen SecurityModels contains request/response data contracts only; controller validation, authorization, and security state own the executable lifecycle.",
        ),
        "Common/Security/AdversarialOptions.cs": (
            ("public sealed class AdversarialOptions", "public enum AdversarialProfile"),
            "Frozen AdversarialOptions contains configuration objects and enum values only; configuration projection and concrete transport/privacy services own executable security behavior.",
        ),
        "Common/Security/I2pTransportOptions.cs": (
            ("public class I2pTransportOptions", "SamBridgeAddress", "ConnectTimeoutSeconds"),
            "Frozen I2pTransportOptions is an options-only compatibility type; the concrete I2P dialer and mesh transport own connection enforcement.",
        ),
        "Common/Security/SecurityOptions.cs": (
            ("public sealed class SecurityOptions", "public SecurityProfile Profile"),
            "Frozen SecurityOptions contains configuration values only; SecurityStartup and the registered concrete controls own activation and enforcement.",
        ),
        "Core/API/DTO/TokenResponse.cs": (
            ("public class TokenResponse", "public string Token", "public string TokenType"),
            "Frozen TokenResponse only projects a signed JWT into the session DTO; token issuance, validation, rotation, and route authorization own the security lifecycle.",
        ),
        "DhtRendezvous/Security/OverlayTimeouts.cs": (
            ("public static class OverlayTimeouts", "MessageRead", "DisconnectGrace"),
            "Frozen OverlayTimeouts is a constants-only timing declaration; the overlay connection implementation owns timeout enforcement.",
        ),
    }
    pure_contract = pure_contracts.get(source)
    if pure_contract is not None:
        required_tokens, reason = pure_contract
        declaration_source = re.sub(r"//[^\n]*|/\*.*?\*/", "", source_text, flags=re.DOTALL)
        method_like = re.search(
            r"^\s*(?:public|internal|protected|private)\s+"
            r"(?:async\s+|static\s+|virtual\s+|override\s+|sealed\s+)*"
            r"[\w<>,.?\[\]]+\s+[\w<>]+\s*\([^;]*\)\s*(?:=>|\{)",
            declaration_source,
            flags=re.MULTILINE,
        )
        if all(token in declaration_source for token in required_tokens) and not method_like:
            return {case: reason for case in declaration_cases}
        return {}

    wiring_contracts = {
        "Common/Security/SecurityServiceExtensions.cs": (
            ("AddSecurityServices", "TryAddSingleton", "SecurityServiceRegistrationOptions"),
            "Frozen SecurityServiceExtensions only composes dependency-injection registrations; the registered concrete controls own security behavior and their external evidence.",
        ),
        "Common/Security/SecurityStartup.cs": (
            ("AddSlskdnSecurity", "GetRegistrationOptions", "UseSlskdnSecurity"),
            "Frozen SecurityStartup only binds configuration, selects registrations, and installs middleware; configuration lifecycle and concrete security controls own the observable contract.",
        ),
        "Core/Security/AuthenticatedWebUserId.cs": (
            ("public static class AuthenticatedWebUserId", "FindFirstValue", "IsAuthenticated"),
            "Frozen AuthenticatedWebUserId is a small claims-projection helper; route authentication and authorization evidence owns the externally observable security behavior.",
        ),
    }
    wiring_contract = wiring_contracts.get(source)
    if wiring_contract is not None:
        required_tokens, reason = wiring_contract
        if all(token in source_text for token in required_tokens):
            return {case: reason for case in declaration_cases}
        return {}

    dormant_utilities = {
        "Common/Security/IdentityConfigurationAuditor.cs": (
            "IdentityConfigurationAuditor",
            "Frozen source contains an identity-audit utility with no production caller or registration in the frozen source tree; its unit tests and documentation do not create an externally observable runtime contract.",
        ),
        "Common/Security/IdentitySeparationValidator.cs": (
            "IdentitySeparationValidator",
            "Frozen source contains an identity-separation utility with no production caller or registration in the frozen source tree; its unit tests and documentation do not create an externally observable runtime contract.",
        ),
        "Common/Security/PrivacyMode.cs": (
            "public static partial class PrivacyMode",
            "Frozen source contains a privacy helper with no production caller or registration in the frozen source tree; active mesh privacy behavior is owned by the registered mesh privacy services.",
        ),
        "DhtRendezvous/Security/PeerDiversityChecker.cs": (
            "public sealed class PeerDiversityChecker",
            "Frozen source contains a peer-diversity checker with no production caller or registration in the frozen source tree; its data contracts do not create an externally observable runtime contract.",
        ),
        "DhtRendezvous/Security/PeerVerificationService.cs": (
            "public sealed class PeerVerificationService",
            "Frozen source contains a peer-verification service with no production caller or registration in the frozen source tree; its shared verification DTOs do not activate the unused service.",
        ),
    }
    dormant_utility = dormant_utilities.get(source)
    if dormant_utility is not None:
        marker, reason = dormant_utility
        class_name = marker.split()[-1]
        production_reference = re.compile(
            rf"(?:\bnew\s+{re.escape(class_name)}\b|"
            rf"\b{re.escape(class_name)}\s*\."
            rf"|\btypeof\s*\(\s*{re.escape(class_name)}\b)"
        )
        production_callers = any(
            candidate != path
            and production_reference.search(
                candidate.read_text(encoding="utf-8-sig")
            )
            for candidate in (frozen_root / "src/slskd").rglob("*.cs")
        )
        if marker in source_text and not production_callers:
            return {case: reason for case in declaration_cases}
        return {}

    contract_only_sources = {
        "Security/ISecurityPolicyEngine.cs": (
            ("record SecurityContext", "record SecurityDecision", "interface ISecurityPolicyEngine", "interface ISecurityPolicy"),
            "Frozen ISecurityPolicyEngine contains only policy interfaces and decision records; registered policy implementations own executable security behavior.",
        ),
        "Sharing/IShareTokenService.cs": (
            ("interface IShareTokenService", "sealed record ShareTokenClaims"),
            "Frozen IShareTokenService contains an interface and claims record only; ShareTokenService and its stream/manifest consumers own token enforcement.",
        ),
    }
    contract_only = contract_only_sources.get(source)
    if contract_only is not None:
        required_tokens, reason = contract_only
        declaration_source = re.sub(r"//[^\n]*|/\*.*?\*/", "", source_text, flags=re.DOTALL)
        if all(token in declaration_source for token in required_tokens) and not re.search(
            r"\b(?:public|internal|private|protected)\s+(?:sealed\s+)?class\s+\w+",
            declaration_source,
        ):
            return {case: reason for case in declaration_cases}
        return {}

    if source not in {
        "Common/Authentication/PassthroughAuthentication.cs",
        "Common/Authentication/ApiKeyAuthentication.cs",
        "Core/Security/SecurityService.cs",
    }:
        return {}

    if re.search(r"lockout|quota|rate\s*-?\s*limit|throttl", source_text, re.IGNORECASE):
        raise ValueError(
            f"security not-applicable allowlist unexpectedly owns throttling: {path}"
        )
    return {
        "quota-time-lockout-and-concurrency": (
            "Frozen authentication components authenticate and project identities only; "
            "request quota and lockout belong to a separate session-controller contract."
        )
    }


def operator_not_applicable_cases(
    frozen_root: Path | None,
    family: str,
    sources: list[str],
) -> dict[str, str]:
    """Classify workflow artifacts that do not own a product lifecycle.

    The operator inventory includes every checked-in GitHub workflow so that
    validation and release automation cannot disappear from the denominator.
    A small, source-validated allowlist prevents repository-maintenance and
    text/security-audit workflows from being mistaken for deployable artifacts.
    Packaging, build, smoke, and runtime workflows are intentionally absent
    from this allowlist and continue to require executable evidence.
    """
    if frozen_root is None:
        return {}

    if family == "container-root" and sources == ["Dockerfile"]:
        dockerfile = frozen_root / sources[0]
        if not dockerfile.is_file():
            return {}
        source = dockerfile.read_text(encoding="utf-8-sig")
        if all(token in source for token in ("FROM ", "ENTRYPOINT", "CMD")) and "apt-get upgrade" not in source:
            return {
                "fresh-install-and-upgrade": (
                    "Frozen container image is an immutable runtime artifact; image installation and upgrade are owned by the container runtime or registry, not the Dockerfile."
                ),
                "failure-rollback-uninstall-and-logs": (
                    "Frozen Dockerfile defines the image process but no uninstall, rollback, or log-retention contract; those belong to the container runtime or deployment controller."
                ),
            }

    if family == "packaging-aur" and "packaging/aur/slskd.service" in sources:
        service = frozen_root / "packaging/aur/slskd.service"
        if service.is_file() and all(
            token in service.read_text(encoding="utf-8-sig")
            for token in ("After=network-online.target", "ExecStart=", "Restart=on-failure")
        ):
            return {
                "network-ports-storage-and-health": (
                    "Frozen AUR package installs a systemd service but declares no independent network or health-check contract; those belong to the daemon service."
                )
            }

    if family == "packaging-debian" and "packaging/debian/rules" in sources:
        rules = frozen_root / "packaging/debian/rules"
        if rules.is_file() and "lib/systemd/system" in rules.read_text(encoding="utf-8-sig"):
            return {
                "network-ports-storage-and-health": (
                    "Frozen Debian package installs a systemd unit but owns no independent network or health-check contract; those belong to the daemon service."
                ),
                "failure-rollback-uninstall-and-logs": (
                    "Frozen Debian package has no independent rollback or log-retention contract; package-manager transaction handling and the installed service own those behaviors."
                ),
            }

    if family == "packaging-rpm" and sources == ["packaging/rpm/slskdn.spec"]:
        spec_path = frozen_root / sources[0]
        if not spec_path.is_file():
            return {}
        spec = spec_path.read_text(encoding="utf-8-sig")
        if not all(token in spec for token in ("%{_unitdir}", "%systemd_post", "%files")):
            return {}
        return {
            "network-ports-storage-and-health": (
                "Frozen RPM spec installs a systemd unit but owns no independent network or health-check contract; those belong to the daemon service."
            )
        }

    if family == "packaging-winget" and sources == [
        "packaging/winget/snapetech.slskdn.installer.yaml",
        "packaging/winget/snapetech.slskdn.locale.en-US.yaml",
        "packaging/winget/snapetech.slskdn.yaml",
    ]:
        manifest_sources = [frozen_root / source for source in sources]
        if not all(path.is_file() for path in manifest_sources):
            return {}
        combined = "\n".join(path.read_text(encoding="utf-8-sig") for path in manifest_sources)
        if not all(
            token in combined
            for token in (
                "PackageIdentifier: snapetech.slskdn",
                "InstallerType: zip",
                "NestedInstallerType: portable",
            )
        ) or "service" in combined.lower():
            return {}
        reason = (
            "Frozen WinGet manifest installs a portable archive and owns no daemon service, configuration store, network/health contract, or rollback/log lifecycle."
        )
        return {
            case: reason
            for case in (
                "start-stop-signal-and-restart",
                "configuration-user-permissions-and-secrets",
                "network-ports-storage-and-health",
                "failure-rollback-uninstall-and-logs",
            )
        }

    if family == "packaging-chocolatey" and sources == [
        "packaging/chocolatey/slskdn.nuspec",
        "packaging/chocolatey/tools/chocolateyinstall.ps1",
    ]:
        package_sources = [frozen_root / source for source in sources]
        if not all(path.is_file() for path in package_sources):
            return {}
        combined = "\n".join(path.read_text(encoding="utf-8-sig") for path in package_sources)
        if not all(token in combined for token in ("<package", "<metadata>", "Install-ChocolateyZipPackage")):
            return {}
        reason = (
            "Frozen Chocolatey package installs a portable archive and owns no daemon service, configuration store, network/health contract, or rollback/log lifecycle."
        )
        return {
            case: reason
            for case in (
                "start-stop-signal-and-restart",
                "configuration-user-permissions-and-secrets",
                "network-ports-storage-and-health",
                "failure-rollback-uninstall-and-logs",
            )
        }

    if family == "packaging-docker" and "packaging/docker/slskdn-container-start" in sources:
        start_script = frozen_root / "packaging/docker/slskdn-container-start"
        if start_script.is_file() and all(
            token in start_script.read_text(encoding="utf-8-sig")
            for token in ("set -e", "SLSKD_APP_DIR", "exec")
        ):
            reason = (
                "Frozen Docker packaging defines an immutable image and startup wrapper; image installation/upgrade and rollback/uninstall/log retention belong to the container runtime or deployment controller."
            )
            result = {
                "fresh-install-and-upgrade": reason,
                "failure-rollback-uninstall-and-logs": reason,
            }
            docker_sources = [frozen_root / source for source in sources]
            docker_text = "\n".join(
                path.read_text(encoding="utf-8-sig")
                for path in docker_sources
                if path.is_file()
            )
            if "HEALTHCHECK" not in docker_text and "EXPOSE" not in docker_text:
                result["network-ports-storage-and-health"] = (
                    "Frozen optional Docker packaging has no independent port or health declaration; those belong to the base daemon image and deployment manifest."
                )
            return result

    if family == "packaging-proxmox-lxc" and "packaging/proxmox-lxc/setup-inside-ct.sh" in sources:
        installer = frozen_root / "packaging/proxmox-lxc/setup-inside-ct.sh"
        if installer.is_file():
            source = installer.read_text(encoding="utf-8-sig")
            if "does not start the service" in source.lower():
                return {
                    "start-stop-signal-and-restart": (
                        "Frozen Proxmox LXC setup intentionally installs and enables the systemd unit without starting it; service start/stop is delegated to the administrator or init system after configuration."
                    )
                }

    if family == "packaging-flatpak" and "packaging/flatpak/io.github.slskd.slskdn.yml" in sources:
        manifest = frozen_root / "packaging/flatpak/io.github.slskd.slskdn.yml"
        if manifest.is_file():
            source = manifest.read_text(encoding="utf-8-sig")
            if "daemon:" not in source and "systemd" not in source:
                reason = (
                    "Frozen Flatpak manifest is a desktop application wrapper without a daemon or systemd lifecycle; the Flatpak runtime owns application start/stop and uninstall behavior."
                )
                return {
                    "start-stop-signal-and-restart": reason,
                    "failure-rollback-uninstall-and-logs": reason,
                }

    if family == "packaging-snap" and "packaging/snap/snapcraft.yaml" in sources:
        manifest = frozen_root / "packaging/snap/snapcraft.yaml"
        if manifest.is_file():
            source = manifest.read_text(encoding="utf-8-sig")
            if "daemon: simple" in source and "rollback" not in source.lower():
                return {
                    "failure-rollback-uninstall-and-logs": (
                        "Frozen Snap manifest delegates package rollback, removal, and service log retention to snapd; it defines no independent failure lifecycle contract."
                    )
                }

    if family in {"packaging-helm", "packaging-truenas-scale"}:
        chart_marker = (
            Path("packaging/helm/slskdn/Chart.yaml")
            if family == "packaging-helm"
            else Path("packaging/truenas-scale/charts/slskdn/Chart.yaml")
        )
        if str(chart_marker) in sources:
            chart_root = frozen_root / chart_marker.parent
            chart_text = "\n".join(
                path.read_text(encoding="utf-8-sig")
                for path in chart_root.rglob("*")
                if path.is_file()
            )
            if "helm rollback" not in chart_text.lower():
                return {
                    "failure-rollback-uninstall-and-logs": (
                        "Frozen Kubernetes chart defines deployment, probes, storage, and service resources but no independent rollback or log-retention implementation; Helm and the cluster controller own that lifecycle."
                    )
                }

    if family == "packaging-unraid" and "packaging/unraid/slskdn.xml" in sources:
        template = frozen_root / "packaging/unraid/slskdn.xml"
        if template.is_file():
            source = template.read_text(encoding="utf-8-sig")
            if "rollback" not in source.lower() and "<Log" not in source:
                return {
                    "failure-rollback-uninstall-and-logs": (
                        "Frozen Unraid template declares container configuration only; Docker/Unraid owns rollback, removal, and log retention."
                    )
                }

    if family == "nix-root" and sources == ["flake.nix"]:
        flake = frozen_root / "flake.nix"
        if flake.is_file():
            source = flake.read_text(encoding="utf-8-sig")
            if not any(token in source for token in ("systemd", "service", "health")):
                reason = (
                    "Frozen Nix flake builds and wraps a portable executable; it defines no daemon, network, health, rollback, or uninstall lifecycle."
                )
                return {
                    "start-stop-signal-and-restart": reason,
                    "network-ports-storage-and-health": reason,
                    "failure-rollback-uninstall-and-logs": reason,
                }

    if family == "packaging-scripts" and sources and all(source.endswith(".sh") for source in sources):
        paths = [frozen_root / source for source in sources]
        if all(path.is_file() for path in paths):
            combined = "\n".join(path.read_text(encoding="utf-8-sig") for path in paths)
            if "#!/usr/bin/env bash" in combined or "#!/bin/bash" in combined:
                reason = (
                    "Frozen packaging/scripts contains release and validation orchestration only; it owns no installable artifact, daemon service, or independent runtime lifecycle."
                )
                return {case: reason for case in (
                    "build-render-and-artifact-contents",
                    "fresh-install-and-upgrade",
                    "start-stop-signal-and-restart",
                    "configuration-user-permissions-and-secrets",
                    "network-ports-storage-and-health",
                    "failure-rollback-uninstall-and-logs",
                )}

    if family == "packaging-smoke" and "packaging/smoke/package-smoke" in sources:
        smoke = frozen_root / "packaging/smoke/package-smoke"
        if smoke.is_file() and "#!/usr/bin/env bash" in smoke.read_text(encoding="utf-8-sig"):
            reason = (
                "Frozen packaging/smoke is a validation harness for other artifacts; it produces no independent installable or runnable product artifact."
            )
            return {case: reason for case in (
                "build-render-and-artifact-contents",
                "fresh-install-and-upgrade",
                "start-stop-signal-and-restart",
                "configuration-user-permissions-and-secrets",
                "network-ports-storage-and-health",
                "failure-rollback-uninstall-and-logs",
            )}

    if family == "systemd-hardened" and sources == ["etc/systemd/slskd-hardened.service"]:
        unit = frozen_root / sources[0]
        if unit.is_file() and "[Service]" in unit.read_text(encoding="utf-8-sig"):
            return {
                "fresh-install-and-upgrade": (
                    "Frozen systemd unit declares daemon runtime behavior but does not install or upgrade itself; the package or deployment tool owns that lifecycle."
                ),
                "network-ports-storage-and-health": (
                    "Frozen hardened unit has no independent network or health-check contract; the daemon and its deployment artifact own those surfaces."
                ),
            }

    if not family.startswith("github-workflow-"):
        if frozen_root is None or family != "packaging-homebrew":
            return {}
        formula_paths = [
            source
            for source in sources
            if source.startswith("packaging/homebrew/Formula/")
            and source.endswith(".rb")
        ]
        if formula_paths != ["packaging/homebrew/Formula/slskdn.rb"]:
            return {}
        formula_path = frozen_root / formula_paths[0]
        if not formula_path.is_file():
            return {}
        formula = formula_path.read_text(encoding="utf-8-sig")
        if not all(
            token in formula
            for token in ("class Slskdn", "def install", "test do", "sha256")
        ) or "service" in formula.lower():
            return {}
        reason = (
            "Frozen Homebrew formula installs a portable executable and runs a CLI smoke test; "
            "it owns no daemon service, configuration store, network/health contract, or rollback/log lifecycle."
        )
        return {
            case: reason
            for case in (
                "start-stop-signal-and-restart",
                "configuration-user-permissions-and-secrets",
                "network-ports-storage-and-health",
                "failure-rollback-uninstall-and-logs",
            )
        }

    contracts = {
        "github-workflow-mirror": (
            ".github/workflows/mirror.yml",
            (
                "git clone --bare https://github.com/slskd/slskd slskd",
                "git push --mirror mirror",
                "GIT_MIRROR_SSH_KEY",
            ),
            (
                "cargo ",
                "dotnet ",
                "npm ",
                "docker",
                "rpm",
                "dpkg",
                "winget",
                "systemd",
            ),
            "Frozen mirror workflow only synchronizes repository refs; it produces no installable or runnable product artifact.",
        ),
        "github-workflow-check-upstream-access": (
            ".github/workflows/check-upstream-access.yml",
            (
                "Check if upstream accepts contributions",
                "git checkout -b upstream-unlocked",
                "gh pr create",
                "gh issue create",
            ),
            (
                "cargo ",
                "dotnet ",
                "npm ",
                "docker",
                "rpm",
                "dpkg",
                "winget",
                "systemd",
            ),
            "Frozen upstream-access workflow only checks repository contribution access and creates repository notifications; it owns no product lifecycle.",
        ),
        "github-workflow-feature-coherence": (
            ".github/workflows/feature-coherence.yml",
            (
                "bash scripts/audit-feature-coherence.sh",
                "bash scripts/audit-readme-maturity-draft.sh",
                "bash scripts/audit-roadmap-claims.sh",
            ),
            (
                "cargo ",
                "dotnet ",
                "npm ",
                "docker",
                "rpm",
                "dpkg",
                "winget",
                "systemd",
            ),
            "Frozen feature-coherence workflow only audits repository claims and documentation; it produces no installable or runnable product artifact.",
        ),
        "github-workflow-local-identity-leaks": (
            ".github/workflows/local-identity-leaks.yml",
            (
                "Install scanner dependencies",
                "bash scripts/check-local-identity-leaks.sh",
                "LOCAL_IDENTITY_SCAN_COMMITS",
            ),
            (
                "cargo ",
                "dotnet ",
                "npm ",
                "docker",
                "rpm",
                "dpkg",
                "winget",
                "systemd",
            ),
            "Frozen local-identity workflow only scans release-facing text and commit history; it produces no installable or runnable product artifact.",
        ),
        "github-workflow-codeql": (
            ".github/workflows/codeql.yml",
            (
                "github/codeql-action/init@v3",
                "github/codeql-action/analyze@v3",
                "dotnet build src/slskd/slskd.csproj --no-restore --configuration Release",
            ),
            (
                "dotnet publish",
                "docker/build-push-action",
                "docker push",
                "softprops/action-gh-release",
                "dpkg-buildpackage",
                "rpmbuild",
            ),
            "Frozen CodeQL workflow only builds for static security analysis; it produces no installable or runnable product artifact.",
        ),
        "github-workflow-ci-enhancements": (
            ".github/workflows/ci-enhancements.yml",
            (
                "Performance Regression Testing",
                "Load Testing",
                "dotnet list src/slskd/slskd.csproj package --vulnerable",
                "k6 run --out json=load-test-results.json",
            ),
            (
                "dotnet publish",
                "docker/build-push-action",
                "docker push",
                "softprops/action-gh-release",
                "dpkg-buildpackage",
                "rpmbuild",
                "gh release",
            ),
            "Frozen CI-enhancements workflow only runs benchmark, load, vulnerability, and diagnostic checks; its temporary test outputs are not product artifacts.",
        ),
        "github-workflow-e2e-tests": (
            ".github/workflows/e2e-tests.yml",
            (
                "name: E2E Tests",
                "npm run test:e2e:ci",
                "SLSKDN_TEST_NO_CONNECT: true",
            ),
            (
                "dotnet publish",
                "docker/build-push-action",
                "docker push",
                "softprops/action-gh-release",
                "dpkg-buildpackage",
                "rpmbuild",
            ),
            "Frozen E2E workflow builds test inputs and publishes only Playwright diagnostics; it produces no installable or runnable product artifact.",
        ),
        "github-workflow-package-smoke-disabled": (
            ".github/workflows/package-smoke-disabled.yml",
            (
                "name: Package Smoke Validation (disabled)",
                "if: false",
                "packaging/smoke/package-smoke",
            ),
            (
                "dotnet publish",
                "docker/build-push-action",
                "docker push",
                "softprops/action-gh-release",
                "dpkg-buildpackage",
                "rpmbuild",
            ),
            "Frozen package-smoke workflow is explicitly disabled at the job level; no package lifecycle is executable from this artifact.",
        ),
        "github-workflow-windows-smoke": (
            ".github/workflows/windows-smoke.yml",
            (
                "runs-on: [self-hosted, Windows, X64, packer-windows]",
                "dotnet build slskd.sln --configuration Release --no-restore",
                "dotnet test slskd.sln --configuration Release --no-build",
            ),
            (
                "dotnet publish",
                "docker/build-push-action",
                "docker push",
                "softprops/action-gh-release",
                "dpkg-buildpackage",
                "rpmbuild",
            ),
            "Frozen Windows-smoke workflow only restores, builds, and tests the solution; it produces no installable or runnable product artifact.",
        ),
    }
    contract = contracts.get(family)
    if contract is None:
        if family.startswith("github-workflow-"):
            source_path = frozen_root / sources[0] if len(sources) == 1 else None
            if source_path is not None and source_path.is_file():
                source = source_path.read_text(encoding="utf-8-sig")
                artifact_tokens = (
                    "upload-artifact",
                    "dotnet publish",
                    "dpkg-buildpackage",
                    "rpmbuild",
                    "choco pack",
                    "wingetcreate",
                    "docker/build-push-action",
                )
                if any(token in source for token in artifact_tokens):
                    reason = (
                        "Frozen workflow produces or publishes an artifact but does not own the installed daemon's service, configuration, network/health, or rollback/log lifecycle; those contracts belong to the package and service artifacts."
                    )
                    return {
                        case: reason
                        for case in (
                            "fresh-install-and-upgrade",
                            "start-stop-signal-and-restart",
                            "configuration-user-permissions-and-secrets",
                            "network-ports-storage-and-health",
                            "failure-rollback-uninstall-and-logs",
                        )
                    }
        return {}
    source_name, required_tokens, forbidden_tokens, reason = contract
    if sources != [source_name]:
        return {}
    source_path = frozen_root / source_name
    if not source_path.is_file():
        return {}
    source = source_path.read_text(encoding="utf-8-sig")
    if not all(token in source for token in required_tokens):
        return {}
    if any(token in source for token in forbidden_tokens):
        return {}
    return {
        case: reason
        for case in (
            "build-render-and-artifact-contents",
            "fresh-install-and-upgrade",
            "start-stop-signal-and-restart",
            "configuration-user-permissions-and-secrets",
            "network-ports-storage-and-health",
            "failure-rollback-uninstall-and-logs",
        )
    }


def protocol_not_applicable_cases(
    frozen_root: Path | None,
    unit: dict[str, Any],
) -> dict[str, str]:
    """Classify protocol cases with no typed payload contract.

    The frozen Soulseek.NET inventory includes a small set of deprecated or
    opaque codes whose applicable proof is raw-frame preservation. Keep the
    allowlist exact and source-validated; all other protocol cases still
    require behavioral evidence.
    """
    if frozen_root is None:
        return {}

    if (
        unit["family"] == "mesh-overlay-control"
        and unit["source"] == "src/slskd/Mesh/Overlay/OverlayControlTypes.cs"
        and unit["name"] in {"Ping", "Pong", "Probe", "ServiceCall", "ServiceReply"}
    ):
        source_path = frozen_root / unit["source"]
        source = source_path.read_text(encoding="utf-8-sig")
        client_path = frozen_root / "src/slskd/Mesh/Overlay/UdpOverlayClient.cs"
        server_path = frozen_root / "src/slskd/Mesh/Overlay/UdpOverlayServer.cs"
        dispatcher_path = frozen_root / "src/slskd/Mesh/Overlay/ControlDispatcher.cs"
        client = client_path.read_text(encoding="utf-8-sig")
        server = server_path.read_text(encoding="utf-8-sig")
        dispatcher = dispatcher_path.read_text(encoding="utf-8-sig")
        expected_constant = re.search(
            rf"\bpublic\s+const\s+string\s+{re.escape(unit['name'])}\s*=\s*\"{re.escape(unit['value'])}\";",
            source,
        )
        if (
            expected_constant is None
            or "Task<bool> SendAsync" not in client
            or "GetActiveConnectionCount() => 0" not in client
            or "await dispatcher.HandleAsync(envelope, stoppingToken)" not in server
            or "SendAsync" in server
            or "private Task<bool> HandleControlLogicAsync" not in dispatcher
        ):
            raise ValueError(
                "mesh-overlay control N/A allowlist no longer matches frozen UDP "
                f"datagram sources: {source_path}"
            )
        return {
            "timeout-cancel-reconnect-and-failure": (
                "Frozen mesh-overlay control is a one-way UDP datagram contract; "
                "the client has no connection state or reply wait, and the server "
                "dispatches without emitting a per-message response."
            ),
            "live-bidirectional-exchange": (
                "Frozen mesh-overlay control is a one-way UDP datagram contract; "
                "there is no per-message bidirectional exchange to reproduce."
            ),
        }

    if (
        unit["family"] == "mesh-sync"
        and unit["name"] == "DhtStore"
        and unit["value"] == 9
        and unit["source"] == "src/slskd/Mesh/Messages/MeshMessages.cs"
    ):
        source_path = frozen_root / unit["source"]
        source = source_path.read_text(encoding="utf-8-sig")
        service_path = frozen_root / "src/slskd/Mesh/MeshSyncService.cs"
        service_source = service_path.read_text(encoding="utf-8-sig")
        switch = re.search(
            r"return\s+message\.Type\s+switch\s*\{(?P<body>.*?)\n\s*\};",
            service_source,
            flags=re.DOTALL,
        )
        if (
            not re.search(r"\bDhtStore\s*=\s*9\b", source)
            or switch is None
            or not service_path.is_file()
        ):
            raise ValueError(
                "mesh-sync DhtStore N/A allowlist no longer matches frozen source: "
                f"{source_path} and {service_path}"
            )
        if "MeshMessageType.DhtStore" in switch.group("body"):
            raise ValueError(
                "mesh-sync DhtStore unexpectedly gained a response branch in frozen source: "
                f"{service_path}"
            )
        return {
            "live-bidirectional-exchange": (
                "Frozen MeshSyncService accepts DhtStore as a one-way DHT publication "
                "and has no response branch; the bidirectional DHT RPC contract is "
                "validated separately by the DHT service evidence."
            )
        }

    if unit["source"] != "vendor/slskNet.Runtime/src/Messaging/MessageCode.cs":
        return {}

    # The base Soulseek inventory is a declaration-only source.  Its enums
    # identify wire discriminants (including a few names containing
    # "Timeout" or "Cancel"), but the file has no connection/session
    # lifecycle code.  Do not turn a transport-owned timeout obligation into
    # one proof row per enum value.  Keep this structural check exact so a
    # future upstream addition of behavior makes the cases open again instead
    # of silently widening the exemption.
    source_path = frozen_root / unit["source"]
    source = source_path.read_text(encoding="utf-8-sig")
    declaration_source = re.sub(r"//[^\n]*|/\*.*?\*/", "", source, flags=re.DOTALL)
    declaration_only = (
        re.search(r"\binternal\s+static\s+class\s+MessageCode\b", declaration_source)
        and {
            "Initialization",
            "Peer",
            "Distributed",
            "Server",
        }.issubset(set(re.findall(r"\bpublic\s+enum\s+(\w+)\b", declaration_source)))
        and not re.search(
            r"\b(?:async|Task|ValueTask|CancellationToken|Socket|Stream)\b|=>|\b(?:void|bool|string|int|byte)\s+\w+\s*\(",
            declaration_source,
        )
    )
    if not declaration_only:
        raise ValueError(
            "base protocol timeout N/A allowlist no longer matches declaration-only "
            f"frozen source: {source_path}"
        )

    base_lifecycle_reason = None
    if unit["family"] in {
        "soulseek-initialization",
        "soulseek-peer",
        "soulseek-distributed",
        "soulseek-server",
    }:
        base_lifecycle_reason = (
            "Frozen MessageCode is a declaration-only wire-code inventory; "
            "timeout, cancellation, reconnect, and failure policy belong to "
            "the owning connection/session service rather than to an individual "
            "base enum value."
        )

    opaque_server_codes = {
        34: "SendSpeed",
        40: "QueuedDownloads",
        65: "ExactFileSearch",
        138: "PrivateRoomUnknown",
        153: "RelatedSearch",
    }
    opaque_peer_codes = {
        1: "PrivateMessage",
        5: "BrowseResponse",
        10: "PrivateRoomInvitation",
        14: "CancelledQueuedTransfer",
        33: "SendConnectToken",
        34: "MoveDownloadToTop",
        37: "FolderContentsResponse",
        47: "ExactFileSearchRequest",
        48: "QueuedDownloads",
        49: "IndirectFileSearchRequest",
    }
    family = unit["family"]
    value = unit["value"]
    expected_name = (
        opaque_server_codes.get(value)
        if family == "soulseek-server"
        else opaque_peer_codes.get(value)
        if family == "soulseek-peer"
        else None
    )
    if expected_name != unit["name"]:
        return (
            {"timeout-cancel-reconnect-and-failure": base_lifecycle_reason}
            if base_lifecycle_reason
            else {}
        )

    source_path = frozen_root / unit["source"]
    source = source_path.read_text(encoding="utf-8-sig")
    if not re.search(
        rf"^\s*{re.escape(expected_name)}\s*=\s*{value},",
        source,
        flags=re.MULTILINE,
    ):
        raise ValueError(f"protocol N/A allowlist no longer matches frozen source: {source_path}")

    if family == "soulseek-server":
        reason = (
            "Frozen MessageCode exposes this legacy server code without a typed payload "
            "contract in the parity codec; raw-frame preservation is the applicable proof."
        )
        cases = {
            "decode-dispatch-and-side-effects": reason,
            "malformed-truncated-oversize-and-unknown": reason,
        }
        if base_lifecycle_reason:
            cases["timeout-cancel-reconnect-and-failure"] = base_lifecycle_reason
        return cases

    cases = {
        "malformed-truncated-oversize-and-unknown": (
            "Frozen MessageCode exposes this deprecated or compressed-opaque peer code, "
            "and the parity codec preserves arbitrary payload bytes without a typed "
            "malformed-payload contract."
        )
    }
    if base_lifecycle_reason:
        cases["timeout-cancel-reconnect-and-failure"] = base_lifecycle_reason
    return cases

