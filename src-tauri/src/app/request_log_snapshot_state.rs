//! Usage: Short-lived in-memory memberships for stable request-log pagination.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

const SNAPSHOT_TTL: Duration = Duration::from_secs(10 * 60);
const MAX_SNAPSHOTS: usize = 32;
const MAX_TOTAL_MEMBERSHIPS: usize = 2_000_000;
pub(crate) const MAX_SNAPSHOT_MEMBERSHIPS: usize = 1_000_000;

#[derive(Debug, Clone)]
pub(crate) struct RequestLogSnapshotSlice {
    pub(crate) snapshot_id: String,
    pub(crate) ids: Vec<i64>,
    pub(crate) total_count: usize,
    pub(crate) total_pages: usize,
    pub(crate) page: usize,
    pub(crate) page_size: usize,
    pub(crate) expires_at_ms: i64,
}

#[derive(Debug)]
struct RequestLogSnapshot {
    filter_fingerprint: String,
    page_size: usize,
    ids: Vec<i64>,
    expires_at: Instant,
    last_access: Instant,
}

#[derive(Debug, Default)]
struct RequestLogSnapshotStore {
    snapshots: HashMap<String, RequestLogSnapshot>,
}

#[derive(Default)]
pub(crate) struct RequestLogSnapshotState {
    inner: Mutex<RequestLogSnapshotStore>,
}

impl RequestLogSnapshotState {
    pub(crate) fn create(
        &self,
        filter_fingerprint: String,
        page_size: usize,
        ids: Vec<i64>,
        page: usize,
    ) -> Result<RequestLogSnapshotSlice, String> {
        let now = Instant::now();
        let expires_at_ms = expiry_unix_ms();
        let mut store = lock_or_recover(&self.inner);
        store.create(filter_fingerprint, page_size, ids, page, now, expires_at_ms)
    }

    pub(crate) fn page(
        &self,
        snapshot_id: &str,
        filter_fingerprint: &str,
        page_size: usize,
        page: usize,
    ) -> Result<RequestLogSnapshotSlice, String> {
        let now = Instant::now();
        let expires_at_ms = expiry_unix_ms();
        let mut store = lock_or_recover(&self.inner);
        store.page(
            snapshot_id,
            filter_fingerprint,
            page_size,
            page,
            now,
            expires_at_ms,
        )
    }

    pub(crate) fn invalidate(&self, snapshot_id: &str) {
        let mut store = lock_or_recover(&self.inner);
        store.snapshots.remove(snapshot_id);
    }
}

impl RequestLogSnapshotStore {
    fn create(
        &mut self,
        filter_fingerprint: String,
        page_size: usize,
        ids: Vec<i64>,
        page: usize,
        now: Instant,
        expires_at_ms: i64,
    ) -> Result<RequestLogSnapshotSlice, String> {
        if ids.len() > MAX_SNAPSHOT_MEMBERSHIPS {
            return Err(
                "REQUEST_LOG_SNAPSHOT_TOO_LARGE: narrow the request-log filters or time range"
                    .to_string(),
            );
        }
        self.prune_expired(now);
        self.evict_for(ids.len());
        let snapshot_id = crate::shared::uuid::new_uuid_v4();
        self.snapshots.insert(
            snapshot_id.clone(),
            RequestLogSnapshot {
                filter_fingerprint,
                page_size,
                ids,
                expires_at: now + SNAPSHOT_TTL,
                last_access: now,
            },
        );
        self.slice(&snapshot_id, page, now, expires_at_ms)
    }

    fn page(
        &mut self,
        snapshot_id: &str,
        filter_fingerprint: &str,
        page_size: usize,
        page: usize,
        now: Instant,
        expires_at_ms: i64,
    ) -> Result<RequestLogSnapshotSlice, String> {
        if !crate::shared::uuid::is_canonical_uuid_v4(snapshot_id) {
            return Err(snapshot_expired_error());
        }
        self.prune_expired(now);
        let Some(snapshot) = self.snapshots.get(snapshot_id) else {
            return Err(snapshot_expired_error());
        };
        if snapshot.filter_fingerprint != filter_fingerprint || snapshot.page_size != page_size {
            return Err(
                "SEC_INVALID_INPUT: request logs snapshot does not match this query".to_string(),
            );
        }
        self.slice(snapshot_id, page, now, expires_at_ms)
    }

    fn slice(
        &mut self,
        snapshot_id: &str,
        page: usize,
        now: Instant,
        expires_at_ms: i64,
    ) -> Result<RequestLogSnapshotSlice, String> {
        let Some(snapshot) = self.snapshots.get_mut(snapshot_id) else {
            return Err(snapshot_expired_error());
        };
        let total_count = snapshot.ids.len();
        let total_pages = total_pages(total_count, snapshot.page_size);
        if page == 0 || page > total_pages {
            return Err("SEC_INVALID_INPUT: request logs page is out of range".to_string());
        }
        let start = (page - 1)
            .checked_mul(snapshot.page_size)
            .ok_or_else(|| "SEC_INVALID_INPUT: request logs page is out of range".to_string())?;
        let end = start.saturating_add(snapshot.page_size).min(total_count);
        snapshot.last_access = now;
        snapshot.expires_at = now + SNAPSHOT_TTL;
        Ok(RequestLogSnapshotSlice {
            snapshot_id: snapshot_id.to_string(),
            ids: snapshot.ids[start..end].to_vec(),
            total_count,
            total_pages,
            page,
            page_size: snapshot.page_size,
            expires_at_ms,
        })
    }

    fn prune_expired(&mut self, now: Instant) {
        self.snapshots
            .retain(|_, snapshot| snapshot.expires_at > now);
    }

    fn evict_for(&mut self, incoming_count: usize) {
        while !self.snapshots.is_empty()
            && (self.snapshots.len() >= MAX_SNAPSHOTS
                || self.total_memberships().saturating_add(incoming_count) > MAX_TOTAL_MEMBERSHIPS)
        {
            let Some(oldest_id) = self
                .snapshots
                .iter()
                .min_by_key(|(_, snapshot)| snapshot.last_access)
                .map(|(snapshot_id, _)| snapshot_id.clone())
            else {
                break;
            };
            self.snapshots.remove(&oldest_id);
        }
    }

    fn total_memberships(&self) -> usize {
        self.snapshots
            .values()
            .map(|snapshot| snapshot.ids.len())
            .sum()
    }
}

fn total_pages(total_count: usize, page_size: usize) -> usize {
    total_count
        .saturating_add(page_size.saturating_sub(1))
        .checked_div(page_size)
        .unwrap_or(0)
        .max(1)
}

fn expiry_unix_ms() -> i64 {
    crate::shared::time::now_unix_millis().saturating_add(SNAPSHOT_TTL.as_millis() as i64)
}

fn snapshot_expired_error() -> String {
    "REQUEST_LOG_SNAPSHOT_EXPIRED: refresh the request-log page".to_string()
}

fn lock_or_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_keeps_membership_page_boundaries_and_total() {
        let now = Instant::now();
        let mut store = RequestLogSnapshotStore::default();
        let first = store
            .create("filters".to_string(), 2, vec![7, 6, 5, 4, 3], 1, now, 1)
            .expect("create snapshot");
        assert_eq!(first.ids, vec![7, 6]);
        assert_eq!(first.total_count, 5);
        assert_eq!(first.total_pages, 3);

        let second = store
            .page(&first.snapshot_id, "filters", 2, 2, now, 2)
            .expect("read second page");
        assert_eq!(second.ids, vec![5, 4]);
        assert_eq!(second.total_pages, 3);
        assert_eq!(second.expires_at_ms, 2);
    }

    #[test]
    fn snapshot_rejects_expired_or_mismatched_queries() {
        let now = Instant::now();
        let mut store = RequestLogSnapshotStore::default();
        let first = store
            .create("filters".to_string(), 2, vec![2, 1], 1, now, 1)
            .expect("create snapshot");
        assert!(store
            .page(&first.snapshot_id, "other", 2, 1, now, 2)
            .unwrap_err()
            .starts_with("SEC_INVALID_INPUT:"));
        assert!(store
            .page(&first.snapshot_id, "filters", 2, 1, now + SNAPSHOT_TTL, 2)
            .unwrap_err()
            .starts_with("REQUEST_LOG_SNAPSHOT_EXPIRED:"));
    }
}
