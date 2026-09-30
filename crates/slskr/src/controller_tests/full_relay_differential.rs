//! Controller full relay differential ownership.

use super::*;

#[cfg_attr(test, test)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-4"
))]
pub(super) fn controller_api_differential_relay_open_cases() {
    run_controller_future_on_large_stack("relay-open-cases", || {
        controller_api_differential_relay_open_cases_impl()
    });
}
