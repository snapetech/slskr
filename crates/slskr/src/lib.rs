#![allow(
    dead_code,
    unused_variables,
    reason = "the proof-only library includes the daemon binary's full compatibility surface"
)]
#![allow(
    clippy::all,
    reason = "the proof-only library reuses binary code; the slskr binary remains linted normally"
)]

// The daemon is primarily a binary, but bounded differential runners link the
// implementation as a library so the proof code is not compiled as a second
// executable test crate.
include!("main.rs");

#[cfg(any(
    feature = "bounded-protocol-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1",
    feature = "bounded-controller-api-tests-2",
    feature = "bounded-controller-api-tests-3",
    feature = "bounded-controller-api-tests-4",
    feature = "bounded-persistence-tests",
    feature = "bounded-file-lifecycle-tests",
    feature = "bounded-security-control-tests",
    feature = "bounded-security-authorization-tests"
))]
pub fn run_bounded_differential() {
    tests::run_bounded_differential_tests();
}

#[cfg(not(any(
    feature = "bounded-protocol-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1",
    feature = "bounded-controller-api-tests-2",
    feature = "bounded-controller-api-tests-3",
    feature = "bounded-controller-api-tests-4",
    feature = "bounded-persistence-tests",
    feature = "bounded-file-lifecycle-tests",
    feature = "bounded-security-control-tests",
    feature = "bounded-security-authorization-tests"
)))]
pub fn run_bounded_differential() {
    panic!("no bounded differential feature selected");
}
