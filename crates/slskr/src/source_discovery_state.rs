use super::*;

pub(super) const SOURCE_DISCOVERY_CYCLE_SECONDS: u64 = 270;
const MAX_SOURCE_DISCOVERY_SEARCHES: usize = 32;

#[derive(Clone, Debug, Default)]
pub(super) struct SourceDiscoveryState {
    pub(super) running: bool,
    pub(super) generation: u64,
    pub(super) starting_generation: Option<u64>,
    pub(super) search_term: String,
    pub(super) hash_verification_enabled: bool,
    pub(super) search_tokens: VecDeque<u32>,
    pub(super) search_cycles: u64,
    pub(super) last_cycle_new_files: usize,
    pub(super) last_dispatch_at: Option<u64>,
}

impl SourceDiscoveryState {
    pub(super) fn begin_start(
        &mut self,
        search_term: String,
        hash_verification_enabled: bool,
    ) -> Option<u64> {
        if self.is_running() {
            return None;
        }
        self.generation = self.generation.wrapping_add(1).max(1);
        self.starting_generation = Some(self.generation);
        self.search_term = search_term;
        self.hash_verification_enabled = hash_verification_enabled;
        self.search_tokens.clear();
        Some(self.generation)
    }

    pub(super) fn finish_start(&mut self, generation: u64, token: u32) -> bool {
        if self.starting_generation != Some(generation) {
            return false;
        }
        self.starting_generation = None;
        self.running = true;
        self.record_dispatch(token);
        true
    }

    pub(super) fn fail_start(&mut self, generation: u64) {
        if self.starting_generation != Some(generation) {
            return;
        }
        self.starting_generation = None;
        self.running = false;
        self.search_term.clear();
        self.hash_verification_enabled = false;
        self.search_tokens.clear();
    }

    pub(super) fn is_running(&self) -> bool {
        self.running || self.starting_generation.is_some()
    }

    pub(super) fn stop(&mut self) -> bool {
        if !self.is_running() {
            return false;
        }
        self.running = false;
        self.starting_generation = None;
        true
    }

    pub(super) fn record_dispatch_if_current(&mut self, generation: u64, token: u32) -> bool {
        if self.running && self.generation == generation {
            self.record_dispatch(token);
            true
        } else {
            false
        }
    }

    pub(super) fn record_dispatch(&mut self, token: u32) {
        if self.search_tokens.len() == MAX_SOURCE_DISCOVERY_SEARCHES {
            self.search_tokens.pop_front();
        }
        self.search_tokens.push_back(token);
        self.search_cycles = self.search_cycles.saturating_add(1);
        self.last_dispatch_at = Some(unix_timestamp());
    }
}
