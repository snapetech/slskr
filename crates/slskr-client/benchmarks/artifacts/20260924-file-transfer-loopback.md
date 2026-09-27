# File Transfer Flush Benchmark — 2026-09-24

Status: internal diagnostic evidence only. This is a local loopback
microbenchmark, not a remote-peer transfer benchmark or a CI performance
threshold.

## Workload

- Harness: `cargo bench --locked -p slskr-client --bench file_transfer_payload`
- Profile: optimized Cargo bench profile
- Payload: 64 MiB per trial, sent in 80 KiB chunks
- Samples: five measured trials and one warmup per variant, repeated twice
- Variants: previous per-chunk `write_all` plus `flush`, and current writer
- Transport: real loopback TCP sockets with `TCP_NODELAY` enabled
- Receiver: validates every plain payload byte and decodes/validates every
  obfuscated frame
- Toolchain: Rust 1.94.0; Tokio 1.53.1
- Host: Linux 7.1.7-arch1-1, x86_64, AMD Ryzen 9 9950X3D

## Results

| Run | Plain median delta | Obfuscated median delta | Sample ranges overlap |
| --- | ---: | ---: | --- |
| First | +0.27% | +0.43% | Yes, both variants |
| Repeat | -0.12% | +0.06% | Yes, both variants |

The medians vary by less than the sample spread, and the direction is not
consistent across repeated runs. No reproducible throughput gain was measured.
The pinned Tokio `TcpStream::flush` implementation is a no-op, so this change
does not remove a socket syscall on the production TCP path. The measurement
does not establish behavior across a remote network or another `AsyncWrite`
implementation. Confidence in the local conclusion is moderate.

Raw output:

- [`20260924-file-transfer-loopback.json`](20260924-file-transfer-loopback.json)
- [`20260924-file-transfer-loopback-repeat.json`](20260924-file-transfer-loopback-repeat.json)
