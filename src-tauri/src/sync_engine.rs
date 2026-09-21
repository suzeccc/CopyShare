use std::collections::{HashSet, VecDeque};

use chrono::Utc;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::models::{ClipboardContentType, ClipboardEventVersion, ClipboardMessage};

const MAX_TRACKED_CLIPBOARD_MESSAGES: usize = 1_024;

pub fn content_hash(content_type: &ClipboardContentType, content: &str) -> String {
    let mut hasher = Sha256::new();
    let format = match content_type {
        ClipboardContentType::Text => "text",
        ClipboardContentType::Image => "image",
        ClipboardContentType::FileList => "fileList",
    };
    hasher.update(format.as_bytes());
    hasher.update([0]);
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[derive(Debug, Clone)]
pub struct SyncEngine {
    device_id: String,
    device_name: String,
    seen_message_ids: HashSet<String>,
    seen_message_order: VecDeque<String>,
    last_local_hash: Option<String>,
    last_remote_hash: Option<String>,
    pending_remote_echo_hashes: HashSet<String>,
    pending_remote_echo_order: VecDeque<String>,
    synchronized_content_hashes: HashSet<String>,
    synchronized_content_order: VecDeque<String>,
    next_origin_sequence: u64,
    hlc_physical_ms: i64,
    hlc_logical: u32,
    last_event_order: Option<ClipboardEventOrder>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ClipboardEventOrder {
    version: ClipboardEventVersion,
    message_id: String,
}

impl SyncEngine {
    pub fn new(device_id: impl Into<String>, device_name: impl Into<String>) -> Self {
        Self {
            device_id: device_id.into(),
            device_name: device_name.into(),
            seen_message_ids: HashSet::new(),
            seen_message_order: VecDeque::new(),
            last_local_hash: None,
            last_remote_hash: None,
            pending_remote_echo_hashes: HashSet::new(),
            pending_remote_echo_order: VecDeque::new(),
            synchronized_content_hashes: HashSet::new(),
            synchronized_content_order: VecDeque::new(),
            next_origin_sequence: 0,
            hlc_physical_ms: 0,
            hlc_logical: 0,
            last_event_order: None,
        }
    }

    pub fn set_device(&mut self, device_id: impl Into<String>, device_name: impl Into<String>) {
        self.device_id = device_id.into();
        self.device_name = device_name.into();
    }

    pub fn observe_local_text(&mut self, text: impl Into<String>) -> Option<ClipboardMessage> {
        self.observe_local_content(ClipboardContentType::Text, text)
    }

    pub fn observe_local_image(
        &mut self,
        image_base64: impl Into<String>,
    ) -> Option<ClipboardMessage> {
        self.observe_local_content(ClipboardContentType::Image, image_base64)
    }

    pub fn observe_local_file_list(
        &mut self,
        file_list: impl Into<String>,
    ) -> Option<ClipboardMessage> {
        self.observe_local_content(ClipboardContentType::FileList, file_list)
    }

    fn observe_local_content(
        &mut self,
        content_type: ClipboardContentType,
        content: impl Into<String>,
    ) -> Option<ClipboardMessage> {
        self.observe_local_content_at(content_type, content, Utc::now().timestamp_millis())
    }

    fn observe_local_content_at(
        &mut self,
        content_type: ClipboardContentType,
        content: impl Into<String>,
        now_ms: i64,
    ) -> Option<ClipboardMessage> {
        let content = content.into();
        let local_hash = content_hash(&content_type, &content);

        if self.pending_remote_echo_hashes.remove(&local_hash) {
            self.pending_remote_echo_order
                .retain(|hash| hash != &local_hash);
            self.last_local_hash = Some(local_hash);
            return None;
        }

        if self.last_local_hash.as_deref() == Some(&local_hash) {
            return None;
        }

        self.next_origin_sequence = self.next_origin_sequence.saturating_add(1);
        let event_version = self.next_local_event_version(now_ms);
        let message = ClipboardMessage {
            message_id: Uuid::new_v4().to_string(),
            source_device_id: self.device_id.clone(),
            source_device_name: self.device_name.clone(),
            content_type,
            content,
            content_hash: local_hash.clone(),
            timestamp: now_ms.div_euclid(1000),
            origin_sequence: Some(self.next_origin_sequence),
            event_version: Some(event_version),
        };
        self.last_local_hash = Some(local_hash);
        self.last_event_order = Some(event_order(&message));
        insert_bounded(
            &mut self.seen_message_ids,
            &mut self.seen_message_order,
            message.message_id.clone(),
        );
        Some(message)
    }

    fn next_local_event_version(&mut self, now_ms: i64) -> ClipboardEventVersion {
        if now_ms > self.hlc_physical_ms {
            self.hlc_physical_ms = now_ms;
            self.hlc_logical = 0;
        } else {
            self.hlc_logical = self.hlc_logical.saturating_add(1);
        }

        ClipboardEventVersion {
            physical_ms: self.hlc_physical_ms,
            logical: self.hlc_logical,
            origin_device_id: self.device_id.clone(),
        }
    }

    fn observe_remote_event_version(&mut self, version: &ClipboardEventVersion) {
        let now_ms = Utc::now().timestamp_millis();
        let next_physical_ms = now_ms.max(self.hlc_physical_ms).max(version.physical_ms);
        self.hlc_logical = if next_physical_ms == self.hlc_physical_ms
            && next_physical_ms == version.physical_ms
        {
            self.hlc_logical.max(version.logical).saturating_add(1)
        } else if next_physical_ms == self.hlc_physical_ms {
            self.hlc_logical.saturating_add(1)
        } else if next_physical_ms == version.physical_ms {
            version.logical.saturating_add(1)
        } else {
            0
        };
        self.hlc_physical_ms = next_physical_ms;
    }

    #[cfg(test)]
    pub(crate) fn observe_local_text_at(
        &mut self,
        text: impl Into<String>,
        now_ms: i64,
    ) -> Option<ClipboardMessage> {
        self.observe_local_content_at(ClipboardContentType::Text, text, now_ms)
    }

    pub fn reset_local_observation(&mut self) {
        self.last_local_hash = None;
    }

    pub fn should_skip_synchronized_content(&self, enabled: bool, hash: &str) -> bool {
        enabled && self.synchronized_content_hashes.contains(hash)
    }

    pub fn mark_content_synchronized(&mut self, hash: String) {
        insert_bounded(
            &mut self.synchronized_content_hashes,
            &mut self.synchronized_content_order,
            hash,
        );
    }

    #[cfg(test)]
    pub fn should_apply_remote_message(&mut self, message: &ClipboardMessage) -> bool {
        self.should_apply_remote_message_with_deduplication(message, true)
    }

    pub fn should_apply_remote_message_with_deduplication(
        &mut self,
        message: &ClipboardMessage,
        deduplicate_sync_content: bool,
    ) -> bool {
        if message.source_device_id == self.device_id {
            return false;
        }

        if self.seen_message_ids.contains(&message.message_id) {
            return false;
        }

        let incoming_order = event_order(message);
        if self
            .last_event_order
            .as_ref()
            .is_some_and(|current| incoming_order <= *current)
        {
            return false;
        }

        let duplicate_content = deduplicate_sync_content
            && (self.last_remote_hash.as_deref() == Some(&message.content_hash)
                || self.last_local_hash.as_deref() == Some(&message.content_hash));
        if duplicate_content {
            self.mark_remote_message_observed(message, false);
            return false;
        }

        true
    }

    pub fn mark_remote_message_applied(&mut self, message: &ClipboardMessage) {
        self.mark_remote_message_observed(message, true);
    }

    fn mark_remote_message_observed(&mut self, message: &ClipboardMessage, track_echo: bool) {
        insert_bounded(
            &mut self.seen_message_ids,
            &mut self.seen_message_order,
            message.message_id.clone(),
        );
        let order = event_order(message);
        self.observe_remote_event_version(&order.version);
        self.last_event_order = Some(order);
        self.last_remote_hash = Some(message.content_hash.clone());
        self.last_local_hash = Some(message.content_hash.clone());
        if track_echo {
            insert_bounded(
                &mut self.pending_remote_echo_hashes,
                &mut self.pending_remote_echo_order,
                message.content_hash.clone(),
            );
        }
    }

    #[cfg(test)]
    pub fn apply_remote_message(&mut self, message: &ClipboardMessage) -> bool {
        self.apply_remote_message_with_deduplication(message, true)
    }

    pub fn apply_remote_message_with_deduplication(
        &mut self,
        message: &ClipboardMessage,
        deduplicate_sync_content: bool,
    ) -> bool {
        if !self.should_apply_remote_message_with_deduplication(message, deduplicate_sync_content) {
            return false;
        }

        self.mark_remote_message_applied(message);
        true
    }

    #[cfg(test)]
    pub fn should_suppress_watcher_echo(&self, content: &str) -> bool {
        let hash = content_hash(&ClipboardContentType::Text, content);
        self.pending_remote_echo_hashes.contains(&hash)
    }

    #[cfg(test)]
    pub(crate) fn last_event_order_key(&self) -> Option<(ClipboardEventVersion, String)> {
        self.last_event_order
            .as_ref()
            .map(|order| (order.version.clone(), order.message_id.clone()))
    }

    #[cfg(test)]
    pub(crate) const fn tracking_capacity() -> usize {
        MAX_TRACKED_CLIPBOARD_MESSAGES
    }

    #[cfg(test)]
    pub(crate) fn tracked_message_count(&self) -> usize {
        self.seen_message_ids.len()
    }

    #[cfg(test)]
    pub(crate) fn pending_echo_count(&self) -> usize {
        self.pending_remote_echo_hashes.len()
    }
}

fn event_order(message: &ClipboardMessage) -> ClipboardEventOrder {
    ClipboardEventOrder {
        version: message.effective_event_version(),
        message_id: message.message_id.clone(),
    }
}

fn insert_bounded(set: &mut HashSet<String>, order: &mut VecDeque<String>, value: String) {
    if set.insert(value.clone()) {
        order.push_back(value);
    }

    while let Some(front) = order.front() {
        if set.contains(front) {
            break;
        }
        order.pop_front();
    }

    while set.len() > MAX_TRACKED_CLIPBOARD_MESSAGES {
        let Some(oldest) = order.pop_front() else {
            break;
        };
        set.remove(&oldest);
    }
}
