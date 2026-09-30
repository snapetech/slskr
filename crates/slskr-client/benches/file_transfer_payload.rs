//! Compare the former per-chunk flush path with the current `write_chunk`
//! implementation over real loopback TCP sockets. This is diagnostic-only;
//! the benchmark makes no throughput guarantee for remote peers.

use std::{
    env, fs, io,
    path::PathBuf,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use serde_json::json;
use slskr_client::file_transfer::{FileTransferConnection, MAX_OBFUSCATED_TRANSFER_FRAME_LEN};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
};

const DEFAULT_MIB: usize = 64;
const DEFAULT_CHUNK_BYTES: usize = 80 * 1024;
const DEFAULT_TRIALS: usize = 5;
const PAYLOAD_BYTE: u8 = 0xa5;

#[derive(Clone, Copy, Debug)]
enum Variant {
    PlainBefore,
    PlainAfter,
    ObfuscatedBefore,
    ObfuscatedAfter,
}

impl Variant {
    const ALL: [Self; 4] = [
        Self::PlainBefore,
        Self::PlainAfter,
        Self::ObfuscatedBefore,
        Self::ObfuscatedAfter,
    ];

    const fn index(self) -> usize {
        match self {
            Self::PlainBefore => 0,
            Self::PlainAfter => 1,
            Self::ObfuscatedBefore => 2,
            Self::ObfuscatedAfter => 3,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::PlainBefore => "plain-before",
            Self::PlainAfter => "plain-after",
            Self::ObfuscatedBefore => "obfuscated-before",
            Self::ObfuscatedAfter => "obfuscated-after",
        }
    }

    const fn is_obfuscated(self) -> bool {
        matches!(self, Self::ObfuscatedBefore | Self::ObfuscatedAfter)
    }

    const fn uses_current_writer(self) -> bool {
        matches!(self, Self::PlainAfter | Self::ObfuscatedAfter)
    }
}

struct Config {
    payload_bytes: usize,
    chunk_bytes: usize,
    trials: usize,
    output: Option<PathBuf>,
}

fn invalid_argument(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

fn parse_config() -> Result<Config, Box<dyn std::error::Error>> {
    let mut mib = DEFAULT_MIB;
    let mut chunk_bytes = DEFAULT_CHUNK_BYTES;
    let mut trials = DEFAULT_TRIALS;
    let mut output = None;
    let mut args = env::args().skip(1);
    while let Some(flag) = args.next() {
        if flag == "--bench" {
            continue;
        }
        let value = args
            .next()
            .ok_or_else(|| invalid_argument(format!("{flag} requires a value")))?;
        match flag.as_str() {
            "--mib" => mib = value.parse()?,
            "--chunk-bytes" => chunk_bytes = value.parse()?,
            "--trials" => trials = value.parse()?,
            "--output" => output = Some(PathBuf::from(value)),
            _ => return Err(invalid_argument(format!("unknown argument: {flag}")).into()),
        }
    }
    if mib == 0 || chunk_bytes == 0 || trials == 0 {
        return Err(invalid_argument("mib, chunk-bytes, and trials must be positive").into());
    }
    let payload_bytes = mib
        .checked_mul(1024 * 1024)
        .ok_or_else(|| invalid_argument("payload size overflows usize"))?;
    let max_chunk_bytes = MAX_OBFUSCATED_TRANSFER_FRAME_LEN - 4;
    if chunk_bytes > max_chunk_bytes {
        return Err(invalid_argument(format!(
            "chunk-bytes must be no larger than {max_chunk_bytes}"
        ))
        .into());
    }
    Ok(Config {
        payload_bytes,
        chunk_bytes,
        trials,
        output,
    })
}

async fn consume_payload(
    stream: TcpStream,
    payload_bytes: usize,
    chunk_bytes: usize,
    obfuscated: bool,
) -> Result<usize, String> {
    let mut received = 0;
    let mut offset = 0;
    let mut connection = if obfuscated {
        FileTransferConnection::new_obfuscated(stream)
    } else {
        FileTransferConnection::new(stream)
    };
    while offset < payload_bytes {
        let length = chunk_bytes.min(payload_bytes - offset);
        let chunk = connection
            .read_chunk(length)
            .await
            .map_err(|error| format!("payload receive failed: {error}"))?;
        if chunk.len() != length || chunk.iter().any(|byte| *byte != PAYLOAD_BYTE) {
            return Err("received payload did not match the sent bytes".to_owned());
        }
        received += chunk.len();
        offset += length;
    }
    Ok(received)
}

async fn run_trial(
    variant: Variant,
    payload: &[u8],
    chunk_bytes: usize,
) -> Result<f64, Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let payload_bytes = payload.len();
    let obfuscated = variant.is_obfuscated();
    let receiver = tokio::spawn(async move {
        let (stream, _) = listener
            .accept()
            .await
            .map_err(|error| format!("loopback accept failed: {error}"))?;
        consume_payload(stream, payload_bytes, chunk_bytes, obfuscated).await
    });
    let mut stream = TcpStream::connect(address).await?;
    stream.set_nodelay(true)?;
    let started = Instant::now();
    let mut offset = 0;

    if variant.uses_current_writer() {
        let mut connection = if obfuscated {
            FileTransferConnection::new_obfuscated(stream)
        } else {
            FileTransferConnection::new(stream)
        };
        while offset < payload.len() {
            let end = (offset + chunk_bytes).min(payload.len());
            connection
                .write_chunk(&payload[offset..end])
                .await
                .map_err(|error| invalid_argument(format!("payload write failed: {error}")))?;
            offset = end;
        }
        stream = connection.into_inner();
    } else {
        while offset < payload.len() {
            let end = (offset + chunk_bytes).min(payload.len());
            let chunk = &payload[offset..end];
            if obfuscated {
                let mut frame = Vec::with_capacity(4 + chunk.len());
                frame.extend_from_slice(&(chunk.len() as u32).to_le_bytes());
                frame.extend_from_slice(chunk);
                let encoded = slskr_protocol::encode_rotated(&frame, rand::random());
                stream.write_all(&encoded).await?;
            } else {
                stream.write_all(chunk).await?;
            }
            // This is the previous payload path. Tokio's TCP stream flush
            // implementation is currently a no-op, so this measures the
            // actual method-call overhead without inventing a buffered peer.
            stream.flush().await?;
            offset = end;
        }
    }

    stream.shutdown().await?;
    let duration = started.elapsed();
    let received = receiver.await??;
    if received != payload.len() {
        return Err(invalid_argument(format!(
            "receiver got {received} payload bytes, expected {}",
            payload.len()
        ))
        .into());
    }
    Ok(payload.len() as f64 / (1024.0 * 1024.0) / duration.as_secs_f64())
}

fn median(values: &[f64]) -> f64 {
    let mut ordered = values.to_vec();
    ordered.sort_by(f64::total_cmp);
    let middle = ordered.len() / 2;
    if ordered.len().is_multiple_of(2) {
        (ordered[middle - 1] + ordered[middle]) / 2.0
    } else {
        ordered[middle]
    }
}

fn summarize(values: &[f64]) -> serde_json::Value {
    let median = median(values);
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    json!({
        "samplesMiBPerSecond": values,
        "medianMiBPerSecond": median,
        "minMiBPerSecond": min,
        "maxMiBPerSecond": max,
    })
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = parse_config()?;
    let payload = vec![PAYLOAD_BYTE; config.payload_bytes];

    // Warm each variant once, then rotate trial order to limit order bias.
    for variant in Variant::ALL {
        let _ = run_trial(variant, &payload, config.chunk_bytes).await?;
    }
    let mut samples: [Vec<f64>; 4] = std::array::from_fn(|_| Vec::with_capacity(config.trials));
    for trial in 0..config.trials {
        for step in 0..Variant::ALL.len() {
            let variant = Variant::ALL[(step + trial) % Variant::ALL.len()];
            samples[variant.index()].push(run_trial(variant, &payload, config.chunk_bytes).await?);
        }
    }

    let plain_before = median(&samples[Variant::PlainBefore.index()]);
    let plain_after = median(&samples[Variant::PlainAfter.index()]);
    let obfuscated_before = median(&samples[Variant::ObfuscatedBefore.index()]);
    let obfuscated_after = median(&samples[Variant::ObfuscatedAfter.index()]);
    let timestamp_ms = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let result = json!({
        "schemaVersion": 1,
        "benchmark": "slskr-file-transfer-payload-loopback",
        "evidenceMode": "local-loopback",
        "timestampUnixMs": timestamp_ms,
        "os": env::consts::OS,
        "architecture": env::consts::ARCH,
        "tcpNodeDelay": true,
        "payloadBytesPerTrial": config.payload_bytes,
        "chunkBytes": config.chunk_bytes,
        "warmupTrialsPerVariant": 1,
        "measuredTrialsPerVariant": config.trials,
        "tokioTcpFlushIsNoOp": true,
        "variants": {
            Variant::PlainBefore.label(): summarize(&samples[Variant::PlainBefore.index()]),
            Variant::PlainAfter.label(): summarize(&samples[Variant::PlainAfter.index()]),
            Variant::ObfuscatedBefore.label(): summarize(&samples[Variant::ObfuscatedBefore.index()]),
            Variant::ObfuscatedAfter.label(): summarize(&samples[Variant::ObfuscatedAfter.index()]),
        },
        "medianDeltaPercent": {
            "plain": (plain_after / plain_before - 1.0) * 100.0,
            "obfuscated": (obfuscated_after / obfuscated_before - 1.0) * 100.0,
        }
    });
    let rendered = serde_json::to_string_pretty(&result)?;
    if let Some(output) = config.output {
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&output, format!("{rendered}\n"))?;
    }
    println!("{rendered}");
    Ok(())
}
