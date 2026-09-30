use super::*;

pub(super) fn songid_fallback_query(source: &str, source_type: &str) -> String {
    if source_type == "local_file" {
        return Path::new(source.trim())
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .trim()
            .to_owned();
    }
    source.trim().to_owned()
}

pub(super) fn songid_spotify_metadata_from_html(source: &str, html: &str) -> serde_json::Value {
    let title = [metadata_value(html, "og:title"), metadata_title(html)]
        .into_iter()
        .find(|value| !value.trim().is_empty())
        .unwrap_or_default()
        .trim()
        .to_owned();
    let description = metadata_value(html, "og:description");
    let description_parts = description
        .split('·')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    let artist = description_parts.first().copied().unwrap_or_default();
    let album = description_parts.get(1).copied().unwrap_or_default();
    let query = match (artist, title.as_str()) {
        (artist, title) if !artist.is_empty() && !title.is_empty() => {
            format!("{artist} - {title}")
        }
        (_, title) if !title.is_empty() => title.to_owned(),
        (artist, _) => artist.to_owned(),
    };
    let source_id = source
        .split_once("/track/")
        .map(|(_, id)| id.split(['?', '#']).next().unwrap_or_default().trim())
        .filter(|id| !id.is_empty())
        .unwrap_or_default();
    serde_json::json!({
        "query": query,
        "metadata": {
            "title": title,
            "artist": artist,
            "album": album,
            "spotifyTrackId": source_id,
            "previewUrl": metadata_value(html, "og:audio"),
            "extra": {"analysisAudioSource": "spotify_page"},
        },
        "evidence": [format!("Spotify page metadata extracted query: {query}")],
    })
}

async fn songid_fetch_spotify_page_metadata(source: &str) -> Result<serde_json::Value, String> {
    let url =
        reqwest::Url::parse(source).map_err(|error| format!("Spotify URL is invalid: {error}"))?;
    if !matches!(url.scheme(), "http" | "https")
        || !url
            .host_str()
            .is_some_and(|host| host.eq_ignore_ascii_case("open.spotify.com"))
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err("Spotify URL is not a public open.spotify.com page".to_owned());
    }
    let host = url
        .host_str()
        .ok_or_else(|| "Spotify URL has no host".to_owned())?;
    let port = url
        .port_or_known_default()
        .ok_or_else(|| "Spotify URL port is unknown".to_owned())?;
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|error| format!("Spotify URL resolution failed: {error}"))?
        .collect::<Vec<_>>();
    if addresses.is_empty()
        || addresses
            .iter()
            .any(|address| is_blocked_integration_ip(address.ip()))
    {
        return Err("Spotify URL resolves to a private address".to_owned());
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .resolve_to_addrs(host, &addresses)
        .build()
        .map_err(|error| format!("failed to build Spotify metadata client: {error}"))?;
    let response = client
        .get(url)
        .header("User-Agent", "slskR-songid/1.0")
        .header(reqwest::header::ACCEPT, "text/html")
        .send()
        .await
        .map_err(|error| format!("Spotify metadata request failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Spotify metadata returned HTTP {}",
            response.status()
        ));
    }
    let body = read_bounded_source_provider_bytes(response, "Spotify metadata").await?;
    let html = String::from_utf8_lossy(&body);
    Ok(songid_spotify_metadata_from_html(source, &html))
}

async fn songid_run_tool(
    command: &mut tokio::process::Command,
    label: &str,
) -> Result<std::process::Output, String> {
    let output = tokio::time::timeout(Duration::from_secs(30), command.output())
        .await
        .map_err(|_| format!("{label} timed out"))?
        .map_err(|error| format!("{label} failed to start: {error}"))?;
    if output.stdout.len() > MAX_SONGID_TOOL_OUTPUT_BYTES
        || output.stderr.len() > MAX_SONGID_TOOL_OUTPUT_BYTES
    {
        return Err(format!(
            "{label} output exceeds {MAX_SONGID_TOOL_OUTPUT_BYTES} bytes"
        ));
    }
    Ok(output)
}

fn songid_parse_fpcalc_output(output: &str) -> Option<String> {
    output.lines().map(str::trim).find_map(|line| {
        if let Some(fingerprint) = line.strip_prefix("FINGERPRINT=") {
            return (!fingerprint.trim().is_empty()).then(|| fingerprint.trim().to_owned());
        }
        if line.is_empty() || line.starts_with("DURATION=") {
            return None;
        }
        Some(line.to_owned())
    })
}

pub(super) async fn songid_extract_chromaprint(
    source: &str,
    settings: &ChromaprintIntegrationSettings,
) -> Result<String, String> {
    if !settings.enabled {
        return Err("Chromaprint is disabled".to_owned());
    }
    let source_path = Path::new(source);
    if !source_path.is_file() {
        return Err("local SongID source is not a file".to_owned());
    }

    // The target feeds ffmpeg's bounded, normalized PCM output into the
    // native Chromaprint service. This host exposes the same library through
    // fpcalc, so normalize through the configured ffmpeg first and invoke
    // fpcalc on that bounded WAV. This also keeps the configured decoder path
    // meaningful instead of silently using an unrelated decoder.
    let normalized_path = std::env::temp_dir().join(format!(
        "slskr-songid-{}.wav",
        uuid::Uuid::new_v4().simple()
    ));
    let result = async {
        let mut ffmpeg = tokio::process::Command::new(&settings.ffmpeg_path);
        ffmpeg
            .arg("-hide_banner")
            .arg("-nostdin")
            .arg("-loglevel")
            .arg("error")
            .arg("-y")
            .arg("-t")
            .arg(settings.duration_seconds.to_string())
            .arg("-i")
            .arg(source_path)
            .arg("-f")
            .arg("wav")
            .arg("-acodec")
            .arg("pcm_s16le")
            .arg("-ar")
            .arg(settings.sample_rate.to_string())
            .arg("-ac")
            .arg(settings.channels.to_string())
            .arg(&normalized_path)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let ffmpeg_output = songid_run_tool(&mut ffmpeg, "ffmpeg SongID normalization").await?;
        if !ffmpeg_output.status.success() {
            let stderr = String::from_utf8_lossy(&ffmpeg_output.stderr);
            return Err(format!(
                "ffmpeg exited with {}: {}",
                ffmpeg_output.status,
                stderr.trim().chars().take(512).collect::<String>()
            ));
        }

        let metadata = fs::metadata(&normalized_path)
            .map_err(|error| format!("normalized SongID audio is unavailable: {error}"))?;
        let max_pcm_bytes = u64::from(settings.sample_rate)
            .checked_mul(u64::from(settings.channels))
            .and_then(|value| value.checked_mul(u64::from(settings.duration_seconds)))
            .and_then(|value| value.checked_mul(2))
            .ok_or_else(|| "Chromaprint PCM size overflow".to_owned())?;
        if metadata.len() == 0 || metadata.len() > max_pcm_bytes.saturating_add(4096) {
            return Err("normalized SongID audio exceeded its bounded duration".to_owned());
        }

        let mut fpcalc = tokio::process::Command::new("fpcalc");
        fpcalc
            .arg("-plain")
            .arg("-algorithm")
            .arg(settings.algorithm.to_string())
            .arg("-length")
            .arg(settings.duration_seconds.to_string())
            .arg("-rate")
            .arg(settings.sample_rate.to_string())
            .arg("-channels")
            .arg(settings.channels.to_string())
            .arg(&normalized_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let fpcalc_output = songid_run_tool(&mut fpcalc, "fpcalc").await?;
        if !fpcalc_output.status.success() {
            let stderr = String::from_utf8_lossy(&fpcalc_output.stderr);
            return Err(format!(
                "fpcalc exited with {}: {}",
                fpcalc_output.status,
                stderr.trim().chars().take(512).collect::<String>()
            ));
        }
        let stdout = String::from_utf8_lossy(&fpcalc_output.stdout);
        songid_parse_fpcalc_output(&stdout)
            .ok_or_else(|| "fpcalc returned no Chromaprint fingerprint".to_owned())
    }
    .await;
    let _ = fs::remove_file(&normalized_path);
    result
}

pub(super) async fn songid_acoustid_lookup(
    fingerprint: &str,
    settings: &AcoustIdIntegrationSettings,
    sample_rate: u32,
    duration_seconds: u32,
) -> Result<Option<serde_json::Value>, String> {
    if !settings.enabled {
        return Ok(None);
    }
    let client_id = settings
        .client_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "AcoustID is enabled without a client id".to_owned())?;
    let base_url = reqwest::Url::parse(&settings.base_url)
        .map_err(|error| format!("AcoustID URL is invalid: {error}"))?;
    let resolved = validate_lidarr_base_url(base_url.as_str())
        .map_err(|error| format!("AcoustID URL is invalid: {error}"))?;
    let endpoint = format!("{}/lookup", base_url.as_str().trim_end_matches('/'));
    let url = reqwest::Url::parse(&endpoint)
        .map_err(|error| format!("AcoustID URL is invalid: {error}"))?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .resolve_to_addrs(&resolved.host, &resolved.addrs)
        .build()
        .map_err(|error| format!("failed to build AcoustID client: {error}"))?;
    let duration = duration_seconds.to_string();
    let sample_rate = sample_rate.to_string();
    let response = client
        .post(url)
        .form(&[
            ("client", client_id),
            ("format", "json"),
            ("fingerprint", fingerprint),
            ("duration", duration.as_str()),
            ("sample_rate", sample_rate.as_str()),
            ("meta", "recordings"),
        ])
        .send()
        .await
        .map_err(|error| format!("AcoustID lookup failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("AcoustID returned HTTP {}", response.status()));
    }
    let root = read_bounded_source_provider_json(response, "AcoustID lookup").await?;
    Ok(root
        .get("results")
        .and_then(serde_json::Value::as_array)
        .and_then(|results| results.first())
        .cloned())
}

fn songid_acoustid_finding(result: &serde_json::Value) -> serde_json::Value {
    let recording = result
        .get("recordings")
        .and_then(serde_json::Value::as_array)
        .and_then(|recordings| recordings.first());
    let recording_id = recording
        .and_then(|recording| recording.get("id"))
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let title = recording
        .and_then(|recording| recording.get("title"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let artist = recording
        .and_then(|recording| recording.get("artists"))
        .and_then(serde_json::Value::as_array)
        .and_then(|artists| artists.first())
        .and_then(|artist| artist.get("name"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let score = result
        .get("score")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    serde_json::json!({
        "recordingId": recording_id,
        "externalId": result.get("id").cloned().unwrap_or(serde_json::Value::Null),
        "title": title,
        "artist": artist,
        "score": score,
        "summary": format!("AcoustID score {score:.2}"),
    })
}

pub(super) async fn songid_source_analysis(
    source: &str,
    source_type: &str,
    integrations: &IntegrationSettings,
) -> (
    String,
    serde_json::Value,
    Vec<String>,
    Option<serde_json::Value>,
    Option<serde_json::Value>,
) {
    if source_type == "spotify_url" {
        if let Some(metadata) = tokio::time::timeout(
            Duration::from_secs(10),
            songid_fetch_spotify_page_metadata(source),
        )
        .await
        .ok()
        .and_then(Result::ok)
        {
            let query = metadata
                .get("query")
                .and_then(serde_json::Value::as_str)
                .filter(|query| !query.trim().is_empty())
                .unwrap_or(source)
                .to_owned();
            let run_metadata = metadata
                .get("metadata")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));
            let evidence = metadata
                .get("evidence")
                .and_then(serde_json::Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .map(str::to_owned)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            return (query, run_metadata, evidence, None, None);
        }
    }

    let query = songid_fallback_query(source, source_type);
    let mut evidence = match source_type {
        "local_file" => vec![format!("Local file path detected: {source}")],
        "youtube_url" => vec!["YouTube URL detected; using source query fallback.".to_owned()],
        "spotify_url" => {
            vec!["Spotify metadata fetch failed; using source query fallback.".to_owned()]
        }
        "url" => vec!["URL detected; using source query fallback.".to_owned()],
        _ => vec!["Treating input as a direct SongID text query.".to_owned()],
    };
    let metadata = serde_json::json!({
        "title": if source_type == "local_file" { query.clone() } else { String::new() },
        "artist": "",
        "album": "",
        "extra": {"analysisAudioSource": source_type},
    });
    let mut full_source_fingerprint = None;
    let mut acoustid_finding = None;
    if source_type == "local_file" && integrations.chromaprint.enabled {
        match songid_extract_chromaprint(source, &integrations.chromaprint).await {
            Ok(fingerprint) => {
                evidence.push(format!(
                    "Chromaprint fingerprint extracted ({} characters).",
                    fingerprint.len()
                ));
                if integrations.acoustid.enabled {
                    match songid_acoustid_lookup(
                        &fingerprint,
                        &integrations.acoustid,
                        integrations.chromaprint.sample_rate,
                        integrations.chromaprint.duration_seconds,
                    )
                    .await
                    {
                        Ok(Some(result)) => {
                            let finding = songid_acoustid_finding(&result);
                            evidence.push("AcoustID returned a recording match.".to_owned());
                            acoustid_finding = Some(finding);
                        }
                        Ok(None) => {
                            evidence.push("AcoustID returned no recording match.".to_owned());
                        }
                        Err(error) => {
                            evidence.push(format!("AcoustID lookup failed: {error}"));
                        }
                    }
                }
                full_source_fingerprint = Some(serde_json::json!({
                    "path": "",
                    "durationSeconds": integrations.chromaprint.duration_seconds,
                    "fingerprintLength": fingerprint.len(),
                    "fingerprint": fingerprint,
                }));
            }
            Err(error) => evidence.push(format!("Chromaprint extraction failed: {error}")),
        }
    }
    (
        query,
        metadata,
        evidence,
        full_source_fingerprint,
        acoustid_finding,
    )
}

pub(super) async fn enqueue_songid_job(
    state: &AppState,
    source: String,
    source_type: &str,
    requested_query: Option<String>,
    match_query: bool,
) -> Result<Option<routing::HttpResponse>, String> {
    let Some(sender) = state.songid_jobs.as_ref().cloned() else {
        return Ok(None);
    };

    let library = state.library.read().await;
    let shares = state.shares.read().await;
    let library_items = library.records.len();
    let shared_files = shares.entries.len();
    drop(shares);
    drop(library);

    let run = match mutate_runtime_compat_state(state, |runtime, _| {
        runtime.queue_songid_run(&source, source_type, library_items, shared_files)
    })
    .await
    {
        Ok(Some(run)) => run,
        Ok(None) => {
            return Ok(Some(routing::service_unavailable_response(
                "song id run space exhausted",
            )));
        }
        Err(error) => return Err(error),
    };
    let Some(run_id) = run.get("id").and_then(serde_json::Value::as_str) else {
        return Ok(Some(routing::service_unavailable_response(
            "song id run has no id",
        )));
    };
    publish_songid_hub_event(state, "create", &run);

    let job = SongIdJob {
        run_id: run_id.to_owned(),
        source,
        source_type: source_type.to_owned(),
        requested_query,
        match_query,
    };
    if sender.try_send(job).is_err() {
        if let Ok(Some(failed)) =
            mutate_runtime_compat_state(state, |runtime, _| runtime.fail_songid_run(run_id)).await
        {
            publish_songid_hub_event(state, "update", &failed);
        }
        return Ok(Some(routing::service_unavailable_response(
            "SongID run queue is unavailable",
        )));
    }
    Ok(Some(routing::accepted_response(run.to_string())))
}

fn songid_path_is_within_root(path: &Path, root: &Path) -> bool {
    let Some(path) = normalize_absolute_path(path) else {
        return false;
    };
    let Some(root) = normalize_absolute_path(root) else {
        return false;
    };
    if !path.starts_with(&root) {
        return false;
    }
    canonicalize_with_missing_tail(&path)
        .zip(canonicalize_with_missing_tail(&root))
        .is_some_and(|(path, root)| path.starts_with(root))
}

/// Local SongID analysis is restricted to downloads, incomplete files, and
/// configured shares, matching the oracle's safe-root check. This is only
/// applied to an existing file; text and URL sources are not filesystem paths.
pub(super) fn songid_local_file_is_allowed(config: &AppConfig, source: &str) -> bool {
    let path = Path::new(source.trim());
    if !path.is_file() {
        return false;
    }
    std::iter::once(config.downloads_dir.as_path())
        .chain(std::iter::once(config.incomplete_dir.as_path()))
        .chain(
            config
                .share_settings
                .directories
                .iter()
                .map(|directory| directory.local_path.as_path()),
        )
        .any(|root| songid_path_is_within_root(path, root))
}

pub(super) fn spawn_songid_workers(state: Arc<AppState>, receiver: mpsc::Receiver<SongIdJob>) {
    let receiver = Arc::new(tokio::sync::Mutex::new(receiver));
    let worker_count = state
        .config
        .media_services
        .song_id_max_concurrent_runs
        .max(1);
    for _ in 0..worker_count {
        let task_state = Arc::clone(&state);
        let receiver = Arc::clone(&receiver);
        state.spawn_managed_task(async move {
            loop {
                let job = receiver.lock().await.recv().await;
                let Some(job) = job else {
                    break;
                };
                let _permit = match Arc::clone(&task_state.songid_run_slots)
                    .acquire_owned()
                    .await
                {
                    Ok(permit) => permit,
                    Err(_) => {
                        fail_songid_run_with_reason(
                            &task_state,
                            &job.run_id,
                            "SongID worker pool is unavailable.",
                        )
                        .await;
                        break;
                    }
                };
                process_songid_job(&task_state, job).await;
            }
        });
    }
}

async fn process_songid_job(state: &AppState, job: SongIdJob) {
    match mutate_runtime_compat_state(state, |runtime, _| {
        runtime.update_songid_run(&job.run_id, |run| {
            run["status"] = serde_json::json!("running");
            run["summary"] = serde_json::json!("Starting SongID source analysis.");
            run["currentStage"] = serde_json::json!("source_analysis");
            run["percentComplete"] = serde_json::json!(0.12);
        })
    })
    .await
    {
        Ok(Some(run)) => publish_songid_hub_event(state, "update", &run),
        Ok(None) => {
            record_daemon_log(
                state,
                logging::LogLevel::Warn,
                "songid",
                format!("SongID run {} disappeared before processing", job.run_id),
            )
            .await;
            return;
        }
        Err(error) => {
            record_daemon_log(
                state,
                logging::LogLevel::Warn,
                "songid",
                format!("SongID run {} could not start: {error}", job.run_id),
            )
            .await;
            fail_songid_run_with_reason(
                state,
                &job.run_id,
                "SongID analysis could not be started.",
            )
            .await;
            return;
        }
    }

    let integrations = state.integration_settings.read().await.clone();
    let (fallback_query, metadata, evidence, full_source_fingerprint, acoustid_finding) =
        songid_source_analysis(&job.source, &job.source_type, &integrations).await;
    let query = job
        .requested_query
        .filter(|query| !query.trim().is_empty())
        .unwrap_or(fallback_query);

    let library = state.library.read().await;
    let shares = state.shares.read().await;
    let runs = songid_runs_value(&library, &shares, job.match_query.then_some(query.as_str()));
    let matches = runs
        .iter()
        .flat_map(|run| {
            run.get("matches")
                .and_then(serde_json::Value::as_array)
                .cloned()
                .unwrap_or_default()
        })
        .collect::<Vec<_>>();
    let library_items = library.records.len();
    let shared_files = shares.entries.len();
    drop(shares);
    drop(library);

    let spotify_metadata_loaded = metadata
        .pointer("/extra/analysisAudioSource")
        .and_then(serde_json::Value::as_str)
        == Some("spotify_page");
    let summary = match job.source_type.as_str() {
        "local_file" if full_source_fingerprint.is_some() && acoustid_finding.is_some() => {
            "Analyzed local file with Chromaprint and AcoustID metadata."
        }
        "local_file" if full_source_fingerprint.is_some() => {
            "Analyzed local file with Chromaprint and filename metadata."
        }
        "local_file" => "Analyzed local file with filename fallback metadata.",
        "youtube_url" => "Classified YouTube URL; optional metadata tools may enrich the run.",
        "spotify_url" if spotify_metadata_loaded => {
            "Analyzed Spotify page metadata for SongID query generation."
        }
        "spotify_url" => "Spotify metadata fetch failed; using source query fallback.",
        "url" => "Classified URL; optional source metadata may enrich the run.",
        _ => "Using free-text SongID query.",
    };
    match mutate_runtime_compat_state(state, |runtime, _| {
        runtime.complete_songid_run(
            &job.run_id,
            &job.source,
            &job.source_type,
            &query,
            matches,
            library_items,
            shared_files,
            summary,
            evidence,
            metadata,
            full_source_fingerprint,
            acoustid_finding,
        )
    })
    .await
    {
        Ok(Some(run)) => publish_songid_hub_event(state, "update", &run),
        Ok(None) => {}
        Err(error) => {
            record_daemon_log(
                state,
                logging::LogLevel::Warn,
                "songid",
                format!(
                    "SongID run {} completion persistence failed: {error}",
                    job.run_id
                ),
            )
            .await;
            fail_songid_run_with_reason(
                state,
                &job.run_id,
                "SongID analysis could not be completed.",
            )
            .await;
        }
    }
}

async fn fail_songid_run_with_reason(state: &AppState, run_id: &str, reason: &str) {
    match mutate_runtime_compat_state(state, |runtime, _| {
        runtime.fail_songid_run_with_reason(run_id, reason)
    })
    .await
    {
        Ok(Some(run)) => publish_songid_hub_event(state, "update", &run),
        Ok(None) => {
            record_daemon_log(
                state,
                logging::LogLevel::Warn,
                "songid",
                format!("SongID run {run_id} disappeared before failure was recorded"),
            )
            .await;
        }
        Err(error) => {
            record_daemon_log(
                state,
                logging::LogLevel::Warn,
                "songid",
                format!("SongID run {run_id} failure state could not be persisted: {error}"),
            )
            .await;
        }
    }
}

pub(super) async fn requeue_persisted_songid_runs(state: &Arc<AppState>) {
    let Some(sender) = state.songid_jobs.as_ref().cloned() else {
        return;
    };
    let pending = {
        let runtime = state.runtime.read().await;
        runtime
            .songid_run_records
            .iter()
            .filter(|run| {
                matches!(
                    run.get("status").and_then(serde_json::Value::as_str),
                    Some("queued" | "running")
                )
            })
            .cloned()
            .collect::<Vec<_>>()
    };
    for run in pending {
        let Some(run_id) = run.get("id").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let Some(source) = run.get("source").and_then(serde_json::Value::as_str) else {
            continue;
        };
        if source.trim().is_empty() {
            continue;
        }
        if run.get("status").and_then(serde_json::Value::as_str) == Some("running") {
            match mutate_runtime_compat_state(state, |runtime, _| {
                runtime.requeue_songid_run(run_id)
            })
            .await
            {
                Ok(Some(_)) => {}
                Ok(None) => {
                    record_daemon_log(
                        state,
                        logging::LogLevel::Warn,
                        "songid",
                        format!("persisted SongID run {run_id} disappeared during recovery"),
                    )
                    .await;
                    continue;
                }
                Err(error) => {
                    record_daemon_log(
                        state,
                        logging::LogLevel::Warn,
                        "songid",
                        format!("SongID run {run_id} recovery persistence failed: {error}"),
                    )
                    .await;
                    continue;
                }
            }
        }
        let job = SongIdJob {
            run_id: run_id.to_owned(),
            source: source.to_owned(),
            source_type: run
                .get("sourceType")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("text_query")
                .to_owned(),
            requested_query: run
                .get("query")
                .and_then(serde_json::Value::as_str)
                .filter(|query| !query.trim().is_empty())
                .map(str::to_owned),
            match_query: state.config.controller_profile != ControllerProfile::Native,
        };
        if sender.try_send(job).is_err() {
            match mutate_runtime_compat_state(state, |runtime, _| runtime.fail_songid_run(run_id))
                .await
            {
                Ok(Some(failed)) => publish_songid_hub_event(state, "update", &failed),
                Ok(None) => {
                    record_daemon_log(
                        state,
                        logging::LogLevel::Warn,
                        "songid",
                        format!("persisted SongID run {run_id} disappeared before queueing"),
                    )
                    .await;
                }
                Err(error) => {
                    record_daemon_log(
                        state,
                        logging::LogLevel::Warn,
                        "songid",
                        format!("SongID run {run_id} could not be marked failed: {error}"),
                    )
                    .await;
                }
            }
        }
    }
}
