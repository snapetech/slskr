use super::*;

#[derive(Debug, Default)]
pub(super) struct ListeningPartyStreamLimits {
    parties: BTreeMap<String, Arc<Semaphore>>,
    per_ip: BTreeMap<String, Arc<Semaphore>>,
}

#[derive(Debug, Eq, PartialEq)]
pub(super) enum ListeningPartyStreamLimitRejection {
    Party,
    Ip,
    Capacity,
}

impl ListeningPartyStreamLimits {
    pub(super) fn try_acquire(
        &mut self,
        party_id: &str,
        remote_ip: &str,
    ) -> Result<(OwnedSemaphorePermit, OwnedSemaphorePermit), ListeningPartyStreamLimitRejection>
    {
        self.parties.retain(|_, semaphore| {
            semaphore.available_permits() < LISTED_PARTY_MAX_CONCURRENT_STREAMS
        });
        self.per_ip.retain(|_, semaphore| {
            semaphore.available_permits() < LISTED_PARTY_MAX_CONCURRENT_STREAMS_PER_IP
        });

        let party = if let Some(party) = self.parties.get(party_id) {
            Arc::clone(party)
        } else {
            if self.parties.len() >= MAX_LISTED_PARTY_STREAM_LIMITERS {
                return Err(ListeningPartyStreamLimitRejection::Capacity);
            }
            let party = Arc::new(Semaphore::new(LISTED_PARTY_MAX_CONCURRENT_STREAMS));
            self.parties.insert(party_id.to_owned(), Arc::clone(&party));
            party
        };
        let party_permit = party
            .try_acquire_owned()
            .map_err(|_| ListeningPartyStreamLimitRejection::Party)?;

        let ip_key = format!("{party_id}:{remote_ip}");
        let ip = if let Some(ip) = self.per_ip.get(&ip_key) {
            Arc::clone(ip)
        } else {
            if self.per_ip.len() >= MAX_LISTED_PARTY_STREAM_LIMITERS {
                return Err(ListeningPartyStreamLimitRejection::Capacity);
            }
            let ip = Arc::new(Semaphore::new(LISTED_PARTY_MAX_CONCURRENT_STREAMS_PER_IP));
            self.per_ip.insert(ip_key, Arc::clone(&ip));
            ip
        };
        let ip_permit = match ip.try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                drop(party_permit);
                return Err(ListeningPartyStreamLimitRejection::Ip);
            }
        };
        Ok((party_permit, ip_permit))
    }
}
