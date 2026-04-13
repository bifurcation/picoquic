//! SACK (Selective Acknowledgment) list management.
//!
//! Translated from picoquic/sacks.c.
//!
//! This module provides SACK list functionality for tracking received packet
//! numbers and generating ACK frames. The SACK list maintains ranges of
//! contiguous packet numbers that have been received, merging adjacent ranges
//! and supporting acknowledgment of acknowledgments.
//!
//! Key features:
//! - Range tracking with automatic merging of adjacent ranges
//! - Send count tracking for each range (regular vs opportunistic ACKs)
//! - ACK horizon support to limit memory usage for acknowledged ranges
//! - Efficient range lookup using BTreeMap

use std::collections::BTreeMap;

// =============================================================================
// Constants
// =============================================================================

/// Maximum number of times a range is tracked before being considered "fully sent".
pub const MAX_ACK_RANGE_REPEAT: usize = 4;

/// Minimum number of times to repeat an ACK range before considering it stable.
pub const MIN_ACK_RANGE_REPEAT: usize = 2;

// =============================================================================
// SACK Item
// =============================================================================

/// A single SACK range representing contiguous received packet numbers.
#[derive(Debug, Clone)]
pub struct SackItem {
    /// Start of the range (inclusive).
    pub start: u64,
    /// End of the range (inclusive).
    pub end: u64,
    /// Time this range was created or last modified.
    pub time_created: u64,
    /// Number of times this range has been sent in ACKs.
    /// Index 0 is for regular ACKs, index 1 is for opportunistic ACKs.
    pub nb_times_sent: [usize; 2],
}

impl SackItem {
    /// Create a new SACK item for the given range.
    pub fn new(start: u64, end: u64, time_created: u64) -> Self {
        Self {
            start,
            end,
            time_created,
            nb_times_sent: [0, 0],
        }
    }

    /// Get the number of times this range has been sent.
    pub fn times_sent(&self, is_opportunistic: bool) -> usize {
        self.nb_times_sent[is_opportunistic as usize]
    }

    /// Record that this range was sent in an ACK.
    pub fn record_sent(&mut self, is_opportunistic: bool) {
        let idx = is_opportunistic as usize;
        if self.nb_times_sent[idx] < MAX_ACK_RANGE_REPEAT {
            self.nb_times_sent[idx] += 1;
        }
    }
}

// =============================================================================
// Range Count Tracking
// =============================================================================

/// Tracks the count of ranges by number of times sent.
///
/// This helps efficiently select which ranges to include in an ACK frame
/// based on how many times they've already been acknowledged.
#[derive(Debug, Clone, Default)]
pub struct RangeCount {
    /// Count of ranges for each send count (0 to MAX_ACK_RANGE_REPEAT-1).
    pub counts: [usize; MAX_ACK_RANGE_REPEAT],
}

impl RangeCount {
    /// Create a new empty range count tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Decrement count for the given send count.
    pub fn decrement(&mut self, times_sent: usize) {
        if times_sent < MAX_ACK_RANGE_REPEAT && self.counts[times_sent] > 0 {
            self.counts[times_sent] -= 1;
        }
    }

    /// Increment count for the given send count.
    pub fn increment(&mut self, times_sent: usize) {
        if times_sent < MAX_ACK_RANGE_REPEAT {
            self.counts[times_sent] += 1;
        }
    }

    /// Reset all counts to zero.
    pub fn reset(&mut self) {
        self.counts = [0; MAX_ACK_RANGE_REPEAT];
    }
}

// =============================================================================
// SACK List
// =============================================================================

/// A list of SACK ranges for tracking received packet numbers.
///
/// The list automatically merges adjacent ranges and supports:
/// - Efficient range insertion and lookup
/// - Send count tracking for ACK generation
/// - ACK horizon to prune old acknowledged ranges
#[derive(Debug)]
pub struct SackList {
    /// Map of SACK ranges, keyed by start of range.
    ranges: BTreeMap<u64, SackItem>,
    /// ACK horizon - packets at or below this are considered received.
    pub ack_horizon: u64,
    /// Delay before moving a range past the horizon (0 = disabled).
    pub horizon_delay: i64,
    /// Range counts for regular (0) and opportunistic (1) ACKs.
    range_counts: [RangeCount; 2],
}

impl Default for SackList {
    fn default() -> Self {
        Self::new()
    }
}

impl SackList {
    /// Create a new empty SACK list.
    pub fn new() -> Self {
        Self {
            ranges: BTreeMap::new(),
            ack_horizon: 0,
            horizon_delay: 0,
            range_counts: [RangeCount::new(), RangeCount::new()],
        }
    }

    /// Check if the list is empty.
    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    /// Get the number of ranges in the list.
    pub fn len(&self) -> usize {
        self.ranges.len()
    }

    /// Get the first (lowest) packet number in the list.
    pub fn first(&self) -> Option<u64> {
        self.ranges.values().next().map(|item| item.start)
    }

    /// Get the last (highest) packet number in the list.
    pub fn last(&self) -> Option<u64> {
        self.ranges.values().next_back().map(|item| item.end)
    }

    /// Get the first SACK item.
    pub fn first_item(&self) -> Option<&SackItem> {
        self.ranges.values().next()
    }

    /// Get the last SACK item.
    pub fn last_item(&self) -> Option<&SackItem> {
        self.ranges.values().next_back()
    }

    /// Iterate over all SACK items in ascending order by start.
    pub fn iter(&self) -> impl Iterator<Item = &SackItem> {
        self.ranges.values()
    }

    /// Iterate over all SACK items in descending order by start.
    pub fn iter_rev(&self) -> impl Iterator<Item = &SackItem> {
        self.ranges.values().rev()
    }

    /// Clear all ranges from the list.
    pub fn clear(&mut self) {
        self.ranges.clear();
        self.range_counts[0].reset();
        self.range_counts[1].reset();
    }

    /// Reset the list to contain a single range.
    pub fn reset(&mut self, range_min: u64, range_max: u64, current_time: u64) -> bool {
        self.clear();
        self.insert(range_min, range_max, current_time)
    }

    /// Insert a new range into the list.
    ///
    /// Returns true on success, false on failure.
    pub fn insert(&mut self, range_min: u64, range_max: u64, current_time: u64) -> bool {
        let item = SackItem::new(range_min, range_max, current_time);
        self.ranges.insert(range_min, item);
        self.range_counts[0].increment(0);
        self.range_counts[1].increment(0);
        true
    }

    /// Find the range that contains or is just below the given packet number.
    pub fn find_range_below(&self, pn: u64) -> Option<&SackItem> {
        // Find the range with the largest start <= pn
        self.ranges.range(..=pn).next_back().map(|(_, item)| item)
    }

    /// Check if a packet number has already been received.
    pub fn is_received(&self, pn: u64) -> bool {
        if self.horizon_delay > 0 && pn < self.ack_horizon {
            return true;
        }
        if let Some(item) = self.find_range_below(pn) {
            pn <= item.end
        } else {
            false
        }
    }

    /// Update the list with a new received range.
    ///
    /// Returns 0 if the range was new, 1 if it was a duplicate.
    pub fn update(&mut self, pn_min: u64, pn_max: u64, current_time: u64) -> i32 {
        let mut is_duplicate = true;

        // Find the range that might contain or be adjacent to pn_min
        let prev_start = self.ranges.range(..=pn_min).next_back().map(|(k, _)| *k);

        if let Some(start) = prev_start {
            let prev = self.ranges.get(&start).unwrap();
            if prev.end + 1 >= pn_min {
                // Overlap or adjacent - extend the existing range
                let mut extended = prev.clone();

                if pn_max > extended.end {
                    extended.end = pn_max;
                    extended.time_created = current_time;
                    is_duplicate = false;

                    // Reset send counts since range was modified
                    self.reset_item_counts(&extended);

                    // Check for merges with subsequent ranges
                    let to_merge: Vec<u64> = self
                        .ranges
                        .range((std::ops::Bound::Excluded(start), std::ops::Bound::Unbounded))
                        .take_while(|(_, item)| item.start <= extended.end + 1)
                        .map(|(k, _)| *k)
                        .collect();

                    for merge_start in to_merge {
                        if let Some(merge_item) = self.ranges.remove(&merge_start) {
                            self.decrement_counts(&merge_item);
                            if merge_item.end > extended.end {
                                extended.end = merge_item.end;
                            }
                            if merge_item.time_created > extended.time_created {
                                extended.time_created = merge_item.time_created;
                            }
                        }
                    }

                    self.ranges.insert(start, extended);
                }

                if self.horizon_delay > 0 {
                    self.update_horizon(current_time);
                }

                return if is_duplicate { 1 } else { 0 };
            }
        }

        // Check if we can extend a subsequent range
        if let Some((next_start, next_item)) = self
            .ranges
            .range(pn_min..)
            .next()
            .map(|(k, v)| (*k, v.clone()))
        {
            if next_item.start <= pn_max + 1 {
                // Extend the next range backwards
                let mut extended = next_item;
                extended.start = pn_min;
                extended.time_created = current_time;
                self.reset_item_counts(&extended);

                self.ranges.remove(&next_start);
                self.decrement_counts(&extended); // Was counted at next_start

                // Check for merges
                let to_merge: Vec<u64> = self
                    .ranges
                    .range((
                        std::ops::Bound::Excluded(pn_min),
                        std::ops::Bound::Unbounded,
                    ))
                    .take_while(|(_, item)| item.start <= extended.end + 1)
                    .map(|(k, _)| *k)
                    .collect();

                for merge_start in to_merge {
                    if let Some(merge_item) = self.ranges.remove(&merge_start) {
                        self.decrement_counts(&merge_item);
                        if merge_item.end > extended.end {
                            extended.end = merge_item.end;
                        }
                        if merge_item.time_created > extended.time_created {
                            extended.time_created = merge_item.time_created;
                        }
                    }
                }

                self.ranges.insert(pn_min, extended);
                self.range_counts[0].increment(0);
                self.range_counts[1].increment(0);

                if self.horizon_delay > 0 {
                    self.update_horizon(current_time);
                }

                return 0;
            }
        }

        // No overlap - insert new range
        self.insert(pn_min, pn_max, current_time);

        if self.horizon_delay > 0 {
            self.update_horizon(current_time);
        }

        0
    }

    /// Check whether a range fills a hole (returns 0) or not (returns -1).
    pub fn check_fills_hole(&self, pn_min: u64, pn_max: u64) -> i32 {
        if let Some(item) = self.find_range_below(pn_min) {
            if pn_max <= item.end {
                return -1; // Already covered
            }
        }
        0
    }

    /// Process acknowledgment of an acknowledgment.
    ///
    /// Marks the corresponding range as fully acknowledged so it can be pruned.
    pub fn process_ack_of_ack(&mut self, start_of_range: u64, end_of_range: u64) {
        if let Some(item) = self.ranges.get(&start_of_range).cloned() {
            // Check if this is the highest range (should not be deleted)
            let is_last = self
                .ranges
                .range((
                    std::ops::Bound::Excluded(start_of_range),
                    std::ops::Bound::Unbounded,
                ))
                .next()
                .is_none();

            if is_last {
                // Shrink the range instead of deleting
                if end_of_range < item.end {
                    let mut updated = item.clone();
                    updated.start = end_of_range + 1;
                    self.ranges.remove(&start_of_range);
                    self.ranges.insert(updated.start, updated);
                } else {
                    // Range is fully covered - keep the last packet
                    let mut updated = item.clone();
                    updated.start = item.end;
                    self.ranges.remove(&start_of_range);
                    self.ranges.insert(updated.start, updated);
                }
            } else if item.end == end_of_range {
                // Exact match - delete or mark as fully sent
                if self.horizon_delay > 0 {
                    // Mark as fully sent but keep for horizon
                    if let Some(existing) = self.ranges.get_mut(&start_of_range) {
                        for r in 0..2 {
                            if existing.nb_times_sent[r] < MAX_ACK_RANGE_REPEAT {
                                self.range_counts[r].decrement(existing.nb_times_sent[r]);
                            }
                            existing.nb_times_sent[r] = MAX_ACK_RANGE_REPEAT;
                        }
                    }
                } else {
                    // Delete the range
                    self.decrement_counts(&item);
                    self.ranges.remove(&start_of_range);
                }
            }
        }
    }

    /// Update the ACK horizon based on time.
    pub fn update_horizon(&mut self, current_time: u64) {
        while let Some((&start, item)) = self.ranges.iter().next() {
            if item.nb_times_sent[0] < MAX_ACK_RANGE_REPEAT {
                break;
            }

            let delay = current_time as i64 - item.time_created as i64;
            if delay <= self.horizon_delay {
                break;
            }

            // Check if there's a next range (always keep the last one)
            let has_next = self
                .ranges
                .range((std::ops::Bound::Excluded(start), std::ops::Bound::Unbounded))
                .next()
                .is_some();

            if !has_next {
                break;
            }

            self.ack_horizon = item.end + 1;
            let item_clone = item.clone();
            self.decrement_counts(&item_clone);
            self.ranges.remove(&start);
        }
    }

    /// Select ACK ranges for transmission.
    ///
    /// Returns (max_send_count, skip_count) indicating which ranges to include.
    pub fn select_ack_ranges(
        &self,
        first_sack_count: Option<usize>,
        max_ranges: usize,
        is_opportunistic: bool,
    ) -> (usize, usize) {
        let idx = is_opportunistic as usize;
        let mut cumul_sent = 0;
        let first_count = first_sack_count.unwrap_or(MAX_ACK_RANGE_REPEAT);

        for i in 0..MAX_ACK_RANGE_REPEAT {
            cumul_sent += self.range_counts[idx].counts[i];
            if i == first_count {
                cumul_sent -= 1;
            }
            if cumul_sent >= max_ranges {
                return (i, cumul_sent - max_ranges);
            }
        }

        (MAX_ACK_RANGE_REPEAT, 0)
    }

    /// Record that a range was sent in an ACK frame.
    pub fn record_sent(&mut self, start: u64, is_opportunistic: bool) {
        let idx = is_opportunistic as usize;
        if let Some(item) = self.ranges.get_mut(&start) {
            if item.nb_times_sent[idx] < MAX_ACK_RANGE_REPEAT {
                self.range_counts[idx].decrement(item.nb_times_sent[idx]);
                item.nb_times_sent[idx] += 1;
                if item.nb_times_sent[idx] < MAX_ACK_RANGE_REPEAT {
                    self.range_counts[idx].increment(item.nb_times_sent[idx]);
                }
            }
        }
    }

    // Helper to decrement counts when removing an item
    fn decrement_counts(&mut self, item: &SackItem) {
        for r in 0..2 {
            if item.nb_times_sent[r] < MAX_ACK_RANGE_REPEAT {
                self.range_counts[r].decrement(item.nb_times_sent[r]);
            }
        }
    }

    // Helper to reset item counts (called when item is modified)
    fn reset_item_counts(&mut self, item: &SackItem) {
        for r in 0..2 {
            if item.nb_times_sent[r] < MAX_ACK_RANGE_REPEAT {
                self.range_counts[r].decrement(item.nb_times_sent[r]);
            }
            self.range_counts[r].increment(0);
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sack_list_new() {
        let list = SackList::new();
        assert!(list.is_empty());
        assert_eq!(list.len(), 0);
        assert!(list.first().is_none());
        assert!(list.last().is_none());
    }

    #[test]
    fn test_sack_list_insert() {
        let mut list = SackList::new();

        list.insert(10, 20, 1000);
        assert!(!list.is_empty());
        assert_eq!(list.len(), 1);
        assert_eq!(list.first(), Some(10));
        assert_eq!(list.last(), Some(20));
    }

    #[test]
    fn test_sack_list_update_no_overlap() {
        let mut list = SackList::new();

        list.update(10, 20, 1000);
        list.update(30, 40, 2000);

        assert_eq!(list.len(), 2);
        assert_eq!(list.first(), Some(10));
        assert_eq!(list.last(), Some(40));
    }

    #[test]
    fn test_sack_list_update_merge_adjacent() {
        let mut list = SackList::new();

        list.update(10, 20, 1000);
        list.update(21, 30, 2000); // Adjacent to first range

        assert_eq!(list.len(), 1);
        assert_eq!(list.first(), Some(10));
        assert_eq!(list.last(), Some(30));
    }

    #[test]
    fn test_sack_list_update_merge_overlap() {
        let mut list = SackList::new();

        list.update(10, 20, 1000);
        list.update(15, 30, 2000); // Overlaps with first range

        assert_eq!(list.len(), 1);
        assert_eq!(list.first(), Some(10));
        assert_eq!(list.last(), Some(30));
    }

    #[test]
    fn test_sack_list_update_duplicate() {
        let mut list = SackList::new();

        let ret1 = list.update(10, 20, 1000);
        assert_eq!(ret1, 0); // New range

        let ret2 = list.update(12, 18, 2000);
        assert_eq!(ret2, 1); // Duplicate (subset of existing)
    }

    #[test]
    fn test_sack_list_is_received() {
        let mut list = SackList::new();

        list.update(10, 20, 1000);
        list.update(30, 40, 2000);

        assert!(list.is_received(10));
        assert!(list.is_received(15));
        assert!(list.is_received(20));
        assert!(!list.is_received(25)); // Gap
        assert!(list.is_received(30));
        assert!(list.is_received(40));
        assert!(!list.is_received(50));
    }

    #[test]
    fn test_sack_list_check_fills_hole() {
        let mut list = SackList::new();

        list.update(10, 20, 1000);

        // Range within existing
        assert_eq!(list.check_fills_hole(12, 18), -1);

        // Range that would fill a gap
        assert_eq!(list.check_fills_hole(25, 30), 0);
    }

    #[test]
    fn test_sack_list_merge_multiple() {
        let mut list = SackList::new();

        list.update(10, 15, 1000);
        list.update(20, 25, 2000);
        list.update(30, 35, 3000);

        assert_eq!(list.len(), 3);

        // Now bridge them all
        list.update(14, 32, 4000);

        assert_eq!(list.len(), 1);
        assert_eq!(list.first(), Some(10));
        assert_eq!(list.last(), Some(35));
    }

    #[test]
    fn test_sack_list_iteration() {
        let mut list = SackList::new();

        list.update(30, 35, 3000);
        list.update(10, 15, 1000);
        list.update(20, 25, 2000);

        let starts: Vec<u64> = list.iter().map(|item| item.start).collect();
        assert_eq!(starts, vec![10, 20, 30]);

        let starts_rev: Vec<u64> = list.iter_rev().map(|item| item.start).collect();
        assert_eq!(starts_rev, vec![30, 20, 10]);
    }

    #[test]
    fn test_sack_list_clear() {
        let mut list = SackList::new();

        list.update(10, 20, 1000);
        list.update(30, 40, 2000);

        list.clear();

        assert!(list.is_empty());
        assert_eq!(list.len(), 0);
    }

    #[test]
    fn test_sack_list_reset() {
        let mut list = SackList::new();

        list.update(10, 20, 1000);
        list.update(30, 40, 2000);

        list.reset(50, 60, 3000);

        assert_eq!(list.len(), 1);
        assert_eq!(list.first(), Some(50));
        assert_eq!(list.last(), Some(60));
    }

    #[test]
    fn test_sack_item_send_tracking() {
        let mut item = SackItem::new(10, 20, 1000);

        assert_eq!(item.times_sent(false), 0);
        assert_eq!(item.times_sent(true), 0);

        item.record_sent(false);
        assert_eq!(item.times_sent(false), 1);
        assert_eq!(item.times_sent(true), 0);

        item.record_sent(true);
        assert_eq!(item.times_sent(false), 1);
        assert_eq!(item.times_sent(true), 1);

        // Should cap at MAX_ACK_RANGE_REPEAT
        for _ in 0..10 {
            item.record_sent(false);
        }
        assert_eq!(item.times_sent(false), MAX_ACK_RANGE_REPEAT);
    }

    #[test]
    fn test_sack_list_record_sent() {
        let mut list = SackList::new();

        list.update(10, 20, 1000);

        // Initially send count is 0
        let item = list.first_item().unwrap();
        assert_eq!(item.times_sent(false), 0);

        // Record a send
        list.record_sent(10, false);

        let item = list.first_item().unwrap();
        assert_eq!(item.times_sent(false), 1);
    }

    #[test]
    fn test_sack_list_select_ack_ranges() {
        let mut list = SackList::new();

        // Insert several ranges
        for i in 0..5 {
            list.update(i * 10, i * 10 + 5, 1000);
        }

        // All ranges have send count 0
        let (max_sent, skip) = list.select_ack_ranges(None, 3, false);
        // With 5 ranges at count 0, and max_ranges=3, we should get (0, 2)
        assert_eq!(max_sent, 0);
        assert_eq!(skip, 2);
    }

    #[test]
    fn test_sack_list_horizon() {
        let mut list = SackList::new();
        list.horizon_delay = 1000;

        // Insert and mark as fully sent
        list.update(10, 20, 0);
        if let Some(item) = list.ranges.get_mut(&10) {
            item.nb_times_sent[0] = MAX_ACK_RANGE_REPEAT;
        }

        // Insert another range
        list.update(30, 40, 500);

        // Update horizon at time 2000 (1000ms after first range created)
        list.update_horizon(2000);

        // First range should be removed, horizon should be 21
        assert_eq!(list.ack_horizon, 21);
        assert!(!list.ranges.contains_key(&10));
    }

    #[test]
    fn test_sack_list_horizon_keeps_last() {
        let mut list = SackList::new();
        list.horizon_delay = 1000;

        // Insert single range and mark as fully sent
        list.update(10, 20, 0);
        if let Some(item) = list.ranges.get_mut(&10) {
            item.nb_times_sent[0] = MAX_ACK_RANGE_REPEAT;
        }

        // Update horizon - should keep last range
        list.update_horizon(2000);

        // Single range should NOT be removed
        assert!(list.ranges.contains_key(&10));
        assert_eq!(list.ack_horizon, 0);
    }

    #[test]
    fn test_sack_list_process_ack_of_ack() {
        let mut list = SackList::new();

        list.update(10, 20, 1000);
        list.update(30, 40, 2000);

        // Process ack of ack for first range
        list.process_ack_of_ack(10, 20);

        // First range should be removed (not the last one)
        assert!(!list.ranges.contains_key(&10));
        assert!(list.ranges.contains_key(&30));
    }

    #[test]
    fn test_sack_list_process_ack_of_ack_last_range() {
        let mut list = SackList::new();

        list.update(10, 20, 1000);

        // Process ack of ack for the only range
        list.process_ack_of_ack(10, 20);

        // Should shrink to just the last packet
        let item = list.first_item().unwrap();
        assert_eq!(item.start, 20);
        assert_eq!(item.end, 20);
    }

    #[test]
    fn test_range_count() {
        let mut rc = RangeCount::new();

        rc.increment(0);
        rc.increment(0);
        rc.increment(1);

        assert_eq!(rc.counts[0], 2);
        assert_eq!(rc.counts[1], 1);
        assert_eq!(rc.counts[2], 0);

        rc.decrement(0);
        assert_eq!(rc.counts[0], 1);

        rc.reset();
        assert_eq!(rc.counts[0], 0);
        assert_eq!(rc.counts[1], 0);
    }

    #[test]
    fn test_sack_list_extend_backwards() {
        let mut list = SackList::new();

        list.update(20, 30, 1000);
        list.update(10, 19, 2000); // Should extend backwards

        assert_eq!(list.len(), 1);
        assert_eq!(list.first(), Some(10));
        assert_eq!(list.last(), Some(30));
    }

    #[test]
    fn test_sack_list_find_range_below() {
        let mut list = SackList::new();

        list.update(10, 20, 1000);
        list.update(30, 40, 2000);
        list.update(50, 60, 3000);

        // Find exact start
        let item = list.find_range_below(30).unwrap();
        assert_eq!(item.start, 30);

        // Find within range
        let item = list.find_range_below(35).unwrap();
        assert_eq!(item.start, 30);

        // Find between ranges
        let item = list.find_range_below(45).unwrap();
        assert_eq!(item.start, 30);

        // Find below all ranges
        let item = list.find_range_below(5);
        assert!(item.is_none());
    }
}
