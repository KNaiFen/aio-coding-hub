//! Local continuity cache for bridged OpenAI Responses requests.

use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::io::{self, Write};
#[cfg(test)]
use std::sync::{Mutex, MutexGuard};
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};

const CACHE_TTL: Duration = Duration::from_secs(10 * 60);
const CACHE_MAX_ENTRIES: usize = 2_000;
const CACHE_MAX_ITEMS_PER_ENTRY: usize = 200;
const CACHE_MAX_BYTES_PER_ENTRY: usize = 1024 * 1024;
const CACHE_MAX_TOTAL_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub(crate) struct ResponsesCacheKey {
    namespace: String,
    response_id: String,
}

impl ResponsesCacheKey {
    pub(crate) fn new(
        namespace: impl Into<String>,
        response_id: impl Into<String>,
    ) -> Option<Self> {
        let namespace = namespace.into();
        let response_id = response_id.into();
        if namespace.trim().is_empty() || response_id.trim().is_empty() {
            return None;
        }
        Some(Self {
            namespace,
            response_id,
        })
    }
}

#[derive(Debug, Clone)]
struct CacheEntry {
    created_at: Instant,
    items_json: Box<[u8]>,
}

struct BoundedCountingWriter {
    bytes_written: usize,
    limit: usize,
}

impl Write for BoundedCountingWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let next_size = self
            .bytes_written
            .checked_add(buffer.len())
            .ok_or_else(|| io::Error::other("cache entry size overflow"))?;
        if next_size > self.limit {
            return Err(io::Error::other("cache entry byte limit exceeded"));
        }
        self.bytes_written = next_size;
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct BoundedVecWriter {
    bytes: Vec<u8>,
    limit: usize,
}

impl Write for BoundedVecWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let next_size = self
            .bytes
            .len()
            .checked_add(buffer.len())
            .ok_or_else(|| io::Error::other("cache entry size overflow"))?;
        if next_size > self.limit {
            return Err(io::Error::other("cache entry byte limit exceeded"));
        }
        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn serialized_size_with_limit<T: serde::Serialize + ?Sized>(
    value: &T,
    limit: usize,
) -> Option<usize> {
    let mut writer = BoundedCountingWriter {
        bytes_written: 0,
        limit,
    };
    serde_json::to_writer(&mut writer, value).ok()?;
    Some(writer.bytes_written)
}

fn serialize_with_limit<T: serde::Serialize + ?Sized>(value: &T, limit: usize) -> Option<Vec<u8>> {
    let mut writer = BoundedVecWriter {
        bytes: Vec::new(),
        limit,
    };
    serde_json::to_writer(&mut writer, value).ok()?;
    Some(writer.bytes)
}

fn cache() -> &'static RwLock<HashMap<ResponsesCacheKey, CacheEntry>> {
    static CACHE: OnceLock<RwLock<HashMap<ResponsesCacheKey, CacheEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

#[cfg(test)]
pub(crate) fn test_guard() -> MutexGuard<'static, ()> {
    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub(crate) fn get(key: &ResponsesCacheKey) -> Option<Vec<Value>> {
    let now = Instant::now();
    {
        let guard = cache().read().ok()?;
        let entry = guard.get(key)?;
        if now.duration_since(entry.created_at) <= CACHE_TTL {
            if let Ok(items) = serde_json::from_slice(&entry.items_json) {
                return Some(items);
            }
        }
    }
    if let Ok(mut guard) = cache().write() {
        guard.remove(key);
    }
    None
}

pub(crate) fn set(key: ResponsesCacheKey, mut items: Vec<Value>) {
    if items.is_empty() {
        return;
    }
    if items.len() > CACHE_MAX_ITEMS_PER_ENTRY {
        items = items.split_off(items.len() - CACHE_MAX_ITEMS_PER_ENTRY);
    }
    let Some(items_json) = serialize_with_limit(&items, CACHE_MAX_BYTES_PER_ENTRY) else {
        return;
    };
    let items_json = items_json.into_boxed_slice();
    let serialized_bytes = items_json.len();
    let Ok(mut guard) = cache().write() else {
        return;
    };
    let now = Instant::now();
    prune_expired_locked(&mut guard, now);

    let mut total_bytes = total_serialized_bytes_locked(&guard);
    if let Some(previous) = guard.remove(&key) {
        total_bytes = total_bytes.saturating_sub(previous.items_json.len());
    }
    while guard.len() >= CACHE_MAX_ENTRIES
        || total_bytes.saturating_add(serialized_bytes) > CACHE_MAX_TOTAL_BYTES
    {
        let Some(oldest_key) = guard
            .iter()
            .min_by_key(|(_, entry)| entry.created_at)
            .map(|(key, _)| key.clone())
        else {
            return;
        };
        if let Some(removed) = guard.remove(&oldest_key) {
            total_bytes = total_bytes.saturating_sub(removed.items_json.len());
        }
    }
    guard.insert(
        key,
        CacheEntry {
            created_at: now,
            items_json,
        },
    );
}

fn prune_expired_locked(cache: &mut HashMap<ResponsesCacheKey, CacheEntry>, now: Instant) {
    cache.retain(|_, entry| now.duration_since(entry.created_at) <= CACHE_TTL);
}

fn total_serialized_bytes_locked(cache: &HashMap<ResponsesCacheKey, CacheEntry>) -> usize {
    cache.values().fold(0, |total, entry| {
        total.saturating_add(entry.items_json.len())
    })
}

pub(crate) fn namespace(
    bridge_type: &str,
    source_provider_id: i64,
    session_id: Option<&str>,
    trace_id: &str,
) -> String {
    let boundary = session_id
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(trace_id);
    format!("{bridge_type}:source={source_provider_id}:session={boundary}")
}

pub(crate) fn cache_completed_response(
    namespace: &str,
    expanded_input: &[Value],
    response: &Value,
) {
    let Some(response_id) = response.get("id").and_then(Value::as_str) else {
        return;
    };
    let Some(output) = response.get("output").and_then(Value::as_array) else {
        return;
    };
    if !output.iter().any(is_tool_call_context_item) {
        return;
    }

    let Some(replay_items) = collect_replayable_items(expanded_input, output) else {
        return;
    };
    if let Some(key) = ResponsesCacheKey::new(namespace, response_id) {
        set(key, replay_items);
    }
}

fn collect_replayable_items(expanded_input: &[Value], output: &[Value]) -> Option<Vec<Value>> {
    let mut replayable_refs = VecDeque::with_capacity(CACHE_MAX_ITEMS_PER_ENTRY);
    for item in expanded_input {
        if is_replayable_input_item(item) {
            if replayable_refs.len() == CACHE_MAX_ITEMS_PER_ENTRY {
                replayable_refs.pop_front();
            }
            replayable_refs.push_back(item);
        }
    }
    for item in output {
        if is_tool_call_context_item(item) {
            if replayable_refs.len() == CACHE_MAX_ITEMS_PER_ENTRY {
                replayable_refs.pop_front();
            }
            replayable_refs.push_back(item);
        }
    }

    let replayable_refs = replayable_refs.into_iter().collect::<Vec<_>>();
    serialized_size_with_limit(&replayable_refs, CACHE_MAX_BYTES_PER_ENTRY)?;
    Some(
        replayable_refs
            .into_iter()
            .filter_map(|item| strip_item_id(item))
            .collect(),
    )
}

fn is_replayable_input_item(item: &Value) -> bool {
    !(item.get("type").and_then(Value::as_str) == Some("reasoning")
        && item.get("encrypted_content").is_some())
}

pub(crate) fn replayable_input_item(item: &Value) -> Option<Value> {
    if !is_replayable_input_item(item) {
        return None;
    }
    strip_item_id(item)
}

pub(crate) fn replayable_output_item(item: &Value) -> Option<Value> {
    if !is_tool_call_context_item(item) {
        return None;
    }
    strip_item_id(item)
}

pub(crate) fn strip_item_id(item: &Value) -> Option<Value> {
    let mut item = item.clone();
    if let Some(obj) = item.as_object_mut() {
        obj.remove("id");
    }
    Some(item)
}

pub(crate) fn is_tool_call_context_item(item: &Value) -> bool {
    matches!(
        item.get("type").and_then(Value::as_str),
        Some(
            "function_call"
                | "tool_call"
                | "local_shell_call"
                | "tool_search_call"
                | "custom_tool_call"
                | "mcp_tool_call"
        )
    )
}

pub(crate) fn is_tool_output_item(item: &Value) -> bool {
    matches!(
        item.get("type").and_then(Value::as_str),
        Some(
            "function_call_output"
                | "tool_call_output"
                | "local_shell_call_output"
                | "tool_search_call_output"
                | "tool_search_output"
                | "custom_tool_call_output"
                | "mcp_tool_call_output"
        )
    )
}

#[cfg(test)]
pub(crate) fn clear_for_tests() {
    if let Ok(mut guard) = cache().write() {
        guard.clear();
    }
}

#[cfg(test)]
pub(crate) fn force_insert_for_tests(key: ResponsesCacheKey, items: Vec<Value>, age: Duration) {
    let items_json = serde_json::to_vec(&items)
        .expect("test cache items should serialize")
        .into_boxed_slice();
    if let Ok(mut guard) = cache().write() {
        guard.insert(
            key,
            CacheEntry {
                created_at: Instant::now()
                    .checked_sub(age)
                    .expect("test cache age should be within Instant range"),
                items_json,
            },
        );
    }
}

#[cfg(test)]
fn set_age_for_tests(key: &ResponsesCacheKey, age: Duration) {
    if let Ok(mut guard) = cache().write() {
        if let Some(entry) = guard.get_mut(key) {
            entry.created_at = Instant::now()
                .checked_sub(age)
                .expect("test cache age should be within Instant range");
        }
    }
}

#[cfg(test)]
fn entry_json_for_tests(key: &ResponsesCacheKey) -> Option<Vec<u8>> {
    cache()
        .read()
        .ok()?
        .get(key)
        .map(|entry| entry.items_json.to_vec())
}

#[cfg(test)]
pub(crate) fn len_for_tests() -> usize {
    cache().read().map(|guard| guard.len()).unwrap_or(0)
}

#[cfg(test)]
fn total_serialized_bytes_for_tests() -> usize {
    cache()
        .read()
        .map(|guard| total_serialized_bytes_locked(&guard))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn items_with_serialized_size(target_size: usize) -> Vec<Value> {
        let mut items = vec![json!({"type": "function_call", "call_id": ""})];
        let base_size = serde_json::to_vec(&items)
            .expect("test cache items should serialize")
            .len();
        assert!(target_size >= base_size);
        items[0]["call_id"] = Value::String("x".repeat(target_size - base_size));
        assert_eq!(
            serde_json::to_vec(&items)
                .expect("test cache items should serialize")
                .len(),
            target_size
        );
        items
    }

    fn fill_cache_to_byte_budget() -> Vec<ResponsesCacheKey> {
        assert_eq!(CACHE_MAX_TOTAL_BYTES % CACHE_MAX_BYTES_PER_ENTRY, 0);
        let entry_count = CACHE_MAX_TOTAL_BYTES / CACHE_MAX_BYTES_PER_ENTRY;
        (0..entry_count)
            .map(|index| {
                let key = ResponsesCacheKey::new(
                    "bridge:source=1:session=a",
                    format!("resp_budget_{index}"),
                )
                .unwrap();
                set(
                    key.clone(),
                    items_with_serialized_size(CACHE_MAX_BYTES_PER_ENTRY),
                );
                set_age_for_tests(&key, Duration::from_secs((entry_count - index) as u64));
                key
            })
            .collect()
    }

    #[test]
    fn cache_completed_response_stores_replayable_tool_context_by_namespace() {
        let _guard = test_guard();
        clear_for_tests();
        let input = vec![json!({
            "id": "msg_1",
            "type": "message",
            "role": "user",
            "content": [{"type": "input_text", "text": "use a tool"}]
        })];
        let response = json!({
            "id": "resp_1",
            "output": [{
                "id": "fc_1",
                "type": "function_call",
                "call_id": "call_1",
                "name": "lookup",
                "arguments": "{}"
            }]
        });

        cache_completed_response("bridge:source=1:session=a", &input, &response);

        let key = ResponsesCacheKey::new("bridge:source=1:session=a", "resp_1").unwrap();
        let cached = get(&key).expect("cached replay items");
        assert_eq!(cached.len(), 2);
        assert!(cached[0].get("id").is_none());
        assert!(cached[1].get("id").is_none());
        assert_eq!(cached[1]["type"], "function_call");

        let other_namespace =
            ResponsesCacheKey::new("bridge:source=2:session=a", "resp_1").unwrap();
        assert!(get(&other_namespace).is_none());
    }

    #[test]
    fn set_stores_the_final_serialized_json_bytes() {
        let _guard = test_guard();
        clear_for_tests();
        let key = ResponsesCacheKey::new("bridge:source=1:session=a", "resp_bytes").unwrap();
        let items = vec![json!({
            "type": "function_call",
            "call_id": "call_1",
            "arguments": {"query": "weather"}
        })];
        let expected_json = serde_json::to_vec(&items).expect("test cache items should serialize");

        set(key.clone(), items.clone());

        assert_eq!(entry_json_for_tests(&key), Some(expected_json));
        assert_eq!(get(&key), Some(items));
    }

    #[test]
    fn cache_completed_response_ignores_plain_text_responses() {
        let _guard = test_guard();
        clear_for_tests();
        cache_completed_response(
            "bridge:source=1:session=a",
            &[json!({"type": "message", "role": "user", "content": []})],
            &json!({
                "id": "resp_text",
                "output": [{
                    "type": "message",
                    "role": "assistant",
                    "content": [{"type": "output_text", "text": "hello"}]
                }]
            }),
        );

        assert_eq!(len_for_tests(), 0);
    }

    #[test]
    fn cache_completed_response_rejects_oversized_replay_context() {
        let _guard = test_guard();
        clear_for_tests();
        let input = vec![json!({
            "type": "message",
            "role": "user",
            "content": "x".repeat(CACHE_MAX_BYTES_PER_ENTRY)
        })];
        let response = json!({
            "id": "resp_large_context",
            "output": [{
                "type": "function_call",
                "call_id": "call_1",
                "name": "lookup",
                "arguments": "{}"
            }]
        });

        cache_completed_response("bridge:source=1:session=a", &input, &response);

        assert_eq!(len_for_tests(), 0);
    }

    #[test]
    fn cache_completed_response_keeps_the_latest_items_before_cloning() {
        let _guard = test_guard();
        clear_for_tests();
        let input = (0..CACHE_MAX_ITEMS_PER_ENTRY + 5)
            .map(|index| json!({"type": "message", "index": index}))
            .collect::<Vec<_>>();
        let response = json!({
            "id": "resp_latest_items",
            "output": [{
                "type": "function_call",
                "index": CACHE_MAX_ITEMS_PER_ENTRY + 5
            }]
        });

        cache_completed_response("bridge:source=1:session=a", &input, &response);

        let key = ResponsesCacheKey::new("bridge:source=1:session=a", "resp_latest_items").unwrap();
        let cached = get(&key).expect("bounded cached items");
        assert_eq!(cached.len(), CACHE_MAX_ITEMS_PER_ENTRY);
        assert_eq!(
            cached.first().and_then(|item| item["index"].as_u64()),
            Some(6)
        );
        assert_eq!(
            cached.last().and_then(|item| item["index"].as_u64()),
            Some((CACHE_MAX_ITEMS_PER_ENTRY + 5) as u64)
        );
    }

    #[test]
    fn expired_entries_are_removed_on_read() {
        let _guard = test_guard();
        clear_for_tests();
        let key = ResponsesCacheKey::new("bridge:source=1:session=a", "resp_old").unwrap();
        force_insert_for_tests(
            key.clone(),
            vec![json!({"type": "function_call", "call_id": "call_1"})],
            CACHE_TTL + Duration::from_secs(1),
        );

        assert!(get(&key).is_none());
        assert_eq!(len_for_tests(), 0);
    }

    #[test]
    fn oversized_entries_are_not_cached() {
        let _guard = test_guard();
        clear_for_tests();
        let key = ResponsesCacheKey::new("bridge:source=1:session=a", "resp_large").unwrap();
        let items = vec![json!({
            "type": "function_call_output",
            "output": "x".repeat(CACHE_MAX_BYTES_PER_ENTRY)
        })];

        set(key.clone(), items);

        assert!(get(&key).is_none());
        assert_eq!(total_serialized_bytes_for_tests(), 0);
    }

    #[test]
    fn total_budget_evicts_oldest_entry_before_inserting() {
        let _guard = test_guard();
        clear_for_tests();
        let existing_keys = fill_cache_to_byte_budget();
        let oldest = existing_keys.first().expect("oldest cache key").clone();
        let survivor = existing_keys.get(1).expect("surviving cache key").clone();
        let incoming =
            ResponsesCacheKey::new("bridge:source=1:session=a", "resp_incoming").unwrap();
        let item = json!({"type": "function_call", "call_id": "call_1"});

        set(incoming.clone(), vec![item]);

        assert!(get(&oldest).is_none());
        assert!(get(&survivor).is_some());
        assert!(get(&incoming).is_some());
        assert_eq!(len_for_tests(), existing_keys.len());
        assert!(total_serialized_bytes_for_tests() <= CACHE_MAX_TOTAL_BYTES);
    }

    #[test]
    fn replacing_an_entry_reuses_its_byte_budget() {
        let _guard = test_guard();
        clear_for_tests();
        let existing_keys = fill_cache_to_byte_budget();
        let neighbor = existing_keys.first().expect("neighbor cache key").clone();
        let replacement = existing_keys.last().expect("replacement cache key").clone();
        let item = json!({"type": "function_call", "call_id": "call_1"});

        set(replacement.clone(), vec![item]);

        assert!(get(&neighbor).is_some());
        assert!(get(&replacement).is_some());
        assert_eq!(len_for_tests(), existing_keys.len());
        assert!(total_serialized_bytes_for_tests() <= CACHE_MAX_TOTAL_BYTES);
    }

    #[test]
    fn set_keeps_only_the_latest_items_within_the_existing_item_limit() {
        let _guard = test_guard();
        clear_for_tests();
        let key = ResponsesCacheKey::new("bridge:source=1:session=a", "resp_items").unwrap();
        let items = (0..CACHE_MAX_ITEMS_PER_ENTRY + 5)
            .map(|index| json!({"type": "function_call", "index": index}))
            .collect::<Vec<_>>();

        set(key.clone(), items);

        let cached = get(&key).expect("bounded cached items");
        assert_eq!(cached.len(), CACHE_MAX_ITEMS_PER_ENTRY);
        assert_eq!(
            cached.first().and_then(|item| item["index"].as_u64()),
            Some(5)
        );
        assert_eq!(
            cached.last().and_then(|item| item["index"].as_u64()),
            Some((CACHE_MAX_ITEMS_PER_ENTRY + 4) as u64)
        );
    }
}
