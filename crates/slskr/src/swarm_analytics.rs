use super::{multisource, unix_timestamp};
use std::collections::{BTreeMap, HashSet};

pub(super) fn swarm_analytics_dashboard(
    store: &multisource::SwarmStore,
    time_window_hours: u64,
    ranking_limit: usize,
) -> serde_json::Value {
    let cutoff = unix_timestamp().saturating_sub(time_window_hours.saturating_mul(3_600));
    let jobs = store
        .list()
        .into_iter()
        .filter(|job| job.created_at >= cutoff)
        .collect::<Vec<_>>();
    let total_downloads = jobs.len() as u64;
    let successful_downloads = jobs.iter().filter(|job| job.status == "completed").count() as u64;
    let failed_downloads = jobs.iter().filter(|job| job.status == "failed").count() as u64;
    let completed = jobs
        .iter()
        .filter_map(|job| job.result.as_ref().filter(|result| result.success))
        .collect::<Vec<_>>();
    let total_bytes = completed
        .iter()
        .map(|result| result.bytes_downloaded)
        .sum::<u64>();
    let total_chunks = completed
        .iter()
        .map(|result| result.chunks.len() as u64)
        .sum::<u64>();
    let attempted_chunks = jobs.iter().map(|job| job.total_chunks).sum::<u64>();
    let average_duration =
        average_u64(completed.iter().map(|result| result.total_time_ms)) / 1_000.0;
    let average_sources = average_u64(completed.iter().map(|result| result.sources_used as u64));
    let average_speed = if completed.is_empty() {
        0.0
    } else {
        completed
            .iter()
            .map(|result| {
                result.bytes_downloaded as f64 / (result.total_time_ms.max(1) as f64 / 1_000.0)
            })
            .sum::<f64>()
            / completed.len() as f64
    };
    let success_rate = ratio(successful_downloads, total_downloads);
    let chunk_success_rate = ratio(total_chunks, attempted_chunks);

    let mut peers = BTreeMap::<String, (String, u64, u64, u64)>::new();
    let mut available_peers = HashSet::new();
    for job in &jobs {
        available_peers.extend(job.sources.iter().map(|source| source.to_ascii_lowercase()));
        if let Some(result) = job.result.as_ref() {
            for chunk in &result.chunks {
                let entry = peers
                    .entry(chunk.username.to_ascii_lowercase())
                    .or_insert_with(|| (chunk.username.clone(), 0, 0, 0));
                entry.1 = entry.1.saturating_add(1);
                entry.2 = entry.2.saturating_add(chunk.bytes_downloaded);
                entry.3 = entry.3.saturating_add(chunk.time_ms);
            }
        }
    }
    let mut peer_rows = peers
        .into_values()
        .map(|(peer_id, chunks, bytes, time_ms)| {
            let average_rtt = if chunks == 0 {
                0.0
            } else {
                time_ms as f64 / chunks as f64
            };
            let throughput = bytes as f64 / (time_ms.max(1) as f64 / 1_000.0);
            (peer_id, chunks, bytes, average_rtt, throughput)
        })
        .collect::<Vec<_>>();
    peer_rows.sort_by(|left, right| {
        right.2.cmp(&left.2).then_with(|| {
            left.0
                .to_ascii_lowercase()
                .cmp(&right.0.to_ascii_lowercase())
        })
    });
    let active_peer_count = peer_rows.len() as u64;
    let peer_rankings = peer_rows
        .into_iter()
        .take(ranking_limit)
        .enumerate()
        .map(
            |(index, (peer_id, chunks, bytes, average_rtt, throughput))| {
                serde_json::json!({
                    "peerId": peer_id,
                    "source": "overlay",
                    "reputationScore": 1.0,
                    "averageRttMs": average_rtt,
                    "averageThroughputBytesPerSecond": throughput,
                    "chunksCompleted": chunks,
                    "chunksFailed": 0,
                    "chunkSuccessRate": 1.0,
                    "totalBytesTransferred": bytes,
                    "rank": index + 1,
                })
            },
        )
        .collect::<Vec<_>>();
    let peer_utilization = ratio(active_peer_count, available_peers.len() as u64);
    let rescue_count = jobs
        .iter()
        .filter(|job| job.output_path.starts_with("rescue/"))
        .count() as u64;
    let first_byte_samples = completed
        .iter()
        .filter_map(|result| result.chunks.iter().map(|chunk| chunk.time_ms).min())
        .collect::<Vec<_>>();
    let efficiency = serde_json::json!({
        "chunkUtilization": ratio(total_chunks, attempted_chunks),
        "peerUtilization": peer_utilization,
        "redundancyFactor": if total_chunks == 0 { 0.0 } else { 1.0 },
        "averageTimeToFirstByteMs": average_u64(first_byte_samples.into_iter()),
        "averageReassignmentRate": 0.0,
        "averageRescueRate": ratio(rescue_count, total_downloads),
    });
    let performance = serde_json::json!({
        "totalDownloads": total_downloads,
        "successfulDownloads": successful_downloads,
        "failedDownloads": failed_downloads,
        "successRate": success_rate,
        "averageDurationSeconds": average_duration,
        "averageSpeedBytesPerSecond": average_speed,
        "averageSourcesUsed": average_sources,
        "totalBytesDownloaded": total_bytes,
        "totalChunksCompleted": total_chunks,
        "chunkSuccessRate": chunk_success_rate,
        "timeWindow": format_dotnet_hours(time_window_hours),
    });
    let mut recommendations = Vec::new();
    if success_rate < 0.8 {
        let description = format!(
            "Current success rate is {}. Consider improving peer selection criteria.",
            format_percent_one(success_rate)
        );
        recommendations.push(swarm_recommendation(
            "PeerSelection",
            "High",
            "Low Success Rate",
            &description,
            "Review peer reputation thresholds and increase minimum reputation score for peer selection.",
            0.3,
        ));
    }
    if chunk_success_rate < 0.9 {
        let description = format!(
            "Chunk success rate is {}. Consider adjusting chunk size.",
            format_percent_one(chunk_success_rate)
        );
        recommendations.push(swarm_recommendation(
            "ChunkSize",
            "Medium",
            "High Chunk Failure Rate",
            &description,
            "Try reducing chunk size to improve reliability, or increase timeout values.",
            0.2,
        ));
    }
    if peer_utilization < 0.5 {
        let description = format!(
            "Only {} of available peers are being utilized.",
            format_percent_one(peer_utilization)
        );
        recommendations.push(swarm_recommendation(
            "SourceCount",
            "Low",
            "Low Peer Utilization",
            &description,
            "Consider increasing the number of sources per download to improve redundancy.",
            0.15,
        ));
    }
    if average_speed / (1024.0 * 1024.0) < 0.5 {
        let description = format!(
            "Average download speed is {:.2} MB/s. This may indicate network or peer issues.",
            average_speed / (1024.0 * 1024.0)
        );
        recommendations.push(swarm_recommendation(
            "NetworkConfig",
            "High",
            "Low Download Speed",
            &description,
            "Check network connectivity, firewall settings, and consider using more sources per download.",
            0.4,
        ));
    }
    recommendations.sort_by(|left, right| {
        recommendation_priority(right)
            .cmp(&recommendation_priority(left))
            .then_with(|| {
                right["estimatedImpact"]
                    .as_f64()
                    .partial_cmp(&left["estimatedImpact"].as_f64())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    serde_json::json!({
        "performanceMetrics": performance,
        "peerRankings": peer_rankings,
        "efficiencyMetrics": efficiency,
        "recommendations": recommendations,
    })
}

fn ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn average_u64(values: impl Iterator<Item = u64>) -> f64 {
    let (total, count) = values.fold((0_u128, 0_u64), |(total, count), value| {
        (total.saturating_add(u128::from(value)), count + 1)
    });
    if count == 0 {
        0.0
    } else {
        total as f64 / count as f64
    }
}

fn format_dotnet_hours(hours: u64) -> String {
    if hours < 24 {
        format!("{hours:02}:00:00")
    } else {
        format!("{}.{:02}:00:00", hours / 24, hours % 24)
    }
}

fn format_percent_one(value: f64) -> String {
    format!("{:.1}%", value * 100.0)
}

fn swarm_recommendation(
    kind: &str,
    priority: &str,
    title: &str,
    description: &str,
    action: &str,
    estimated_impact: f64,
) -> serde_json::Value {
    serde_json::json!({
        "type": kind,
        "priority": priority,
        "title": title,
        "description": description,
        "action": action,
        "estimatedImpact": estimated_impact,
    })
}

fn recommendation_priority(value: &serde_json::Value) -> u8 {
    match value["priority"].as_str() {
        Some("Critical") => 4,
        Some("High") => 3,
        Some("Medium") => 2,
        Some("Low") => 1,
        _ => 0,
    }
}
