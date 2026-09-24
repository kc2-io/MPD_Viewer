//! Pure assignment-time accounting. No player telemetry or wall clock enters this policy.
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

#[derive(Clone, Debug)]
pub struct Turn {
    pub broadcast: String,
    pub budget: Option<Duration>,
    pub elapsed: Duration,
}
impl Turn {
    pub fn remaining(&self) -> Option<Duration> {
        self.budget.map(|b| b.saturating_sub(self.elapsed))
    }
    pub fn overdue(&self) -> bool {
        self.remaining() == Some(Duration::ZERO)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Rotation {
    pub turns: HashMap<String, Turn>,
    deferred: HashSet<String>,
    admitted: HashSet<String>,
    reset_on_open: HashSet<String>,
    pub pending: Option<(String, String)>,
    pending_bypassed: HashSet<String>,
    eligible: Vec<(String, String)>,
}
impl Rotation {
    pub fn clear_round(&mut self) {
        self.deferred.clear();
        self.admitted.clear();
        self.pending = None;
        self.pending_bypassed.clear();
    }
    pub fn remove(&mut self, login: &str) {
        self.turns.remove(login);
        self.deferred.remove(login);
        self.admitted.remove(login);
        self.reset_on_open.remove(login);
        if self
            .pending
            .as_ref()
            .is_some_and(|(a, b)| a == login || b == login)
        {
            self.clear_round();
        }
    }
    pub fn configure(&mut self, login: &str, minutes: Option<u32>) {
        if let Some(turn) = self.turns.get_mut(login) {
            turn.budget = minutes.map(|m| Duration::from_secs(u64::from(m) * 60));
            turn.elapsed = Duration::ZERO;
        }
        self.clear_round();
    }
    /// Eligibility/broadcast changes release old round deferrals; stale poll metadata does not.
    pub fn observe(&mut self, eligible: Vec<(String, String)>) {
        if self.eligible != eligible {
            self.clear_round();
            self.eligible = eligible;
        }
    }
    /// Begins only after the native host successfully opens; retries preserve the same turn.
    pub fn opened(&mut self, login: &str, broadcast: &str, minutes: Option<u32>) {
        let reset = self.reset_on_open.remove(login);
        let turn = self.turns.entry(login.into()).or_insert_with(|| Turn {
            broadcast: broadcast.into(),
            budget: minutes.map(|m| Duration::from_secs(u64::from(m) * 60)),
            elapsed: Duration::ZERO,
        });
        if reset || turn.broadcast != broadcast {
            *turn = Turn {
                broadcast: broadcast.into(),
                budget: minutes.map(|m| Duration::from_secs(u64::from(m) * 60)),
                elapsed: Duration::ZERO,
            };
        }
        if let Some((source, target)) = &self.pending {
            if target == login {
                self.reset_on_open.insert(source.clone());
                self.admitted.insert(login.into());
                // A transiently unavailable earlier target must not preempt the
                // newly admitted turn as soon as its next poll succeeds.
                self.deferred
                    .extend(self.pending_bypassed.drain().filter(|id| id != login));
                self.pending = None;
            }
        }
    }
    /// A long interval is treated as suspension, not active assignment. Counting is
    /// independent of remote playback states, and paused automation consumes nothing.
    pub fn advance(&mut self, elapsed: Duration, running: bool, assigned: &HashSet<String>) {
        if !running || elapsed > Duration::from_secs(5) {
            return;
        }
        for login in assigned {
            if let Some(turn) = self.turns.get_mut(login) {
                turn.elapsed = turn.elapsed.saturating_add(elapsed);
            }
        }
    }
    pub fn overdue(&self, login: &str) -> bool {
        self.turns.get(login).is_some_and(Turn::overdue)
    }
    pub fn deferred(&self, login: &str) -> bool {
        self.deferred.contains(login)
    }
    /// The admitted member must receive its own timed turn before wrapping a wave.
    pub fn ready_to_wrap(&self) -> bool {
        self.admitted.iter().any(|id| self.overdue(id))
    }
    pub fn wrap(&mut self) {
        self.clear_round();
    }
    pub fn begin(&mut self, source: String, target: String) {
        self.deferred.insert(source.clone());
        self.pending = Some((source, target));
    }
    /// Pure selection: retain top-K behavior until a timed assignment must yield.
    /// Call with rotation disabled while a native close still reserves capacity.
    pub fn desired(
        &mut self,
        ranked: &[crate::Favorite],
        presence: &HashMap<String, crate::Presence>,
        existing: &HashSet<String>,
        skipped: &HashMap<String, String>,
        failed: &HashSet<String>,
        limit: usize,
        allow_rotation: bool,
    ) -> Vec<String> {
        // Closing can span a poll. A retained broadcast ID alone is not enough
        // to open the reserved target after its first missing/stale observation.
        if let Some((source, target)) = self.pending.clone() {
            let fresh: HashSet<_> =
                crate::select(ranked, presence, &HashSet::new(), skipped, usize::MAX)
                    .into_iter()
                    .filter(|id| {
                        !existing.contains(id) && !failed.contains(id) && !self.deferred(id)
                    })
                    .collect();
            if !fresh.contains(&target) {
                let order: Vec<_> = ranked.iter().map(|f| f.login.clone()).collect();
                if let Some(next) = next_waiting(&order, &source, &fresh) {
                    self.pending_bypassed.insert(target);
                    self.pending = Some((source, next));
                } else {
                    // Preserve the source's overdue budget and let ordinary fresh
                    // eligibility reopen it once destruction releases capacity.
                    self.failed_target();
                }
            }
        }
        let mut selectable: Vec<_> = ranked
            .iter()
            .filter(|f| {
                !self.deferred(&f.login)
                    && (!failed.contains(&f.login) || existing.contains(&f.login))
            })
            .cloned()
            .collect();
        // The reserved target must be the channel actually opened. Other slots
        // retain normal priority order; native close acknowledgements still gate
        // all opening in the controller.
        if let Some((_, target)) = &self.pending {
            if let Some(index) = selectable.iter().position(|f| &f.login == target) {
                let reserved = selectable.remove(index);
                selectable.insert(0, reserved);
            }
        }
        let mut desired = crate::select(&selectable, presence, existing, skipped, limit);
        if !allow_rotation
            || self.pending.is_some()
            || desired.len() < limit
            || desired.iter().any(|id| !existing.contains(id))
        {
            return desired;
        }
        let wrap = self.ready_to_wrap();
        let available: HashSet<_> =
            crate::select(ranked, presence, &HashSet::new(), skipped, usize::MAX)
                .into_iter()
                .filter(|id| {
                    !existing.contains(id) && !failed.contains(id) && (wrap || !self.deferred(id))
                })
                .collect();
        let order: Vec<_> = ranked.iter().map(|f| f.login.clone()).collect();
        let mut expired: Vec<_> = desired
            .iter()
            .filter(|id| self.overdue(id))
            .cloned()
            .collect();
        // Oldest deadline first; stable sort retains favorite rank on ties.
        expired.sort_by_key(|id| {
            std::cmp::Reverse(
                self.turns[id]
                    .elapsed
                    .saturating_sub(self.turns[id].budget.unwrap_or_default()),
            )
        });
        for source in &expired {
            if !self.overdue(source) {
                continue;
            }
            if let Some(target) = next_waiting(&order, source, &available) {
                let source = source.clone();
                if wrap {
                    // Release only the channel receiving this turn. Other yielded
                    // channels must not preempt retained wave members on the next tick.
                    self.deferred.remove(&target);
                    self.admitted
                        .retain(|id| !self.turns.get(id).is_some_and(Turn::overdue));
                }
                self.begin(source.clone(), target.clone());
                desired
                    .iter_mut()
                    .filter(|id| **id == source)
                    .for_each(|id| *id = target.clone());
                break;
            }
        }
        desired
    }
    /// Failed target: keep the expired source available as a final recovery option.
    pub fn failed_target(&mut self) -> Option<String> {
        self.pending_bypassed.clear();
        self.pending.take().map(|(source, _)| {
            self.deferred.remove(&source);
            source
        })
    }
}

/// Scan forward from an expired channel, wrapping rank. Exclude all assigned,
/// closing, deferred, skipped, offline/stale-to-open and failed targets upstream.
pub fn next_waiting(
    ranked: &[String],
    source: &str,
    candidates: &HashSet<String>,
) -> Option<String> {
    let start = ranked.iter().position(|s| s == source)?;
    (1..ranked.len())
        .map(|n| &ranked[(start + n) % ranked.len()])
        .find(|id| candidates.contains(*id))
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn set(names: &[&str]) -> HashSet<String> {
        names.iter().map(|s| (*s).into()).collect()
    }
    #[test]
    fn assignment_clock_pauses_ignores_sleep_and_retry_keeps_budget() {
        let mut r = Rotation::default();
        r.opened("a", "b1", Some(1));
        r.advance(Duration::from_secs(5), true, &set(&["a"]));
        r.advance(Duration::from_secs(3600), true, &set(&["a"]));
        r.advance(Duration::from_secs(50), false, &set(&["a"]));
        r.opened("a", "b1", Some(1));
        assert_eq!(r.turns["a"].remaining(), Some(Duration::from_secs(55)));
        r.opened("a", "b2", Some(1));
        assert_eq!(r.turns["a"].elapsed, Duration::ZERO);
    }
    #[test]
    fn simultaneous_expiry_does_not_bounce_displaced_channel() {
        let mut r = Rotation::default();
        for name in ["a", "b"] {
            r.opened(name, "live", Some(1));
            r.turns.get_mut(name).unwrap().elapsed = Duration::from_secs(60);
        }
        r.begin("a".into(), "c".into());
        r.opened("c", "live", Some(1));
        assert!(r.deferred("a"));
        assert!(!r.ready_to_wrap());
        assert!(r.overdue("b"));
        r.turns.get_mut("c").unwrap().elapsed = Duration::from_secs(60);
        assert!(r.ready_to_wrap());
        r.wrap();
        r.begin("b".into(), "a".into());
        r.opened("a", "live", Some(1));
        assert_eq!(r.turns["a"].elapsed, Duration::ZERO);
        assert!(r.deferred("b"));
    }
    #[test]
    fn forward_selection_wraps_and_excludes_ineligible_channels() {
        let ranked = vec!["a".into(), "b".into(), "c".into(), "d".into()];
        assert_eq!(
            next_waiting(&ranked, "a", &set(&["c", "d"])),
            Some("c".into())
        );
        assert_eq!(next_waiting(&ranked, "d", &set(&["a"])), Some("a".into()));
        assert_eq!(next_waiting(&ranked, "a", &set(&["a"])), None);
    }
    #[test]
    fn failed_replacement_recovers_overdue_source_without_fresh_budget() {
        let mut r = Rotation::default();
        r.opened("a", "live", Some(1));
        r.turns.get_mut("a").unwrap().elapsed = Duration::from_secs(60);
        r.begin("a".into(), "b".into());
        assert_eq!(r.failed_target(), Some("a".into()));
        r.opened("a", "live", Some(1));
        assert!(r.overdue("a"));
        assert!(!r.deferred("a"));
    }
    #[test]
    fn unlimited_does_not_wrap_and_timer_edit_resets_only_target() {
        let mut r = Rotation::default();
        r.opened("a", "live", Some(1));
        r.opened("b", "live", Some(1));
        r.advance(Duration::from_secs(5), true, &set(&["a", "b"]));
        r.configure("a", Some(10));
        assert_eq!(r.turns["a"].remaining(), Some(Duration::from_secs(600)));
        assert_eq!(r.turns["b"].remaining(), Some(Duration::from_secs(55)));
        r.begin("a".into(), "c".into());
        r.opened("c", "live", None);
        assert!(!r.ready_to_wrap());
    }
}

#[cfg(test)]
mod policy_tests {
    use super::*;
    use crate::{Favorite, Presence};
    struct Harness {
        r: Rotation,
        ranked: Vec<Favorite>,
        presence: HashMap<String, Presence>,
        active: HashSet<String>,
        failed: HashSet<String>,
        skips: HashMap<String, String>,
        cap: usize,
    }
    impl Harness {
        fn new(names: &[&str], cap: usize) -> Self {
            Self {
                r: Rotation::default(),
                ranked: names
                    .iter()
                    .map(|s| Favorite {
                        login: (*s).into(),
                        enabled: true,
                    })
                    .collect(),
                presence: names
                    .iter()
                    .map(|s| {
                        (
                            (*s).into(),
                            Presence {
                                broadcast_id: Some(format!("{s}-1")),
                                fresh: true,
                                ..Default::default()
                            },
                        )
                    })
                    .collect(),
                active: HashSet::new(),
                failed: HashSet::new(),
                skips: HashMap::new(),
                cap,
            }
        }
        fn desired(&mut self, rotate: bool) -> Vec<String> {
            self.r.desired(
                &self.ranked,
                &self.presence,
                &self.active,
                &self.skips,
                &self.failed,
                self.cap,
                rotate,
            )
        }
        fn apply(&mut self, desired: Vec<String>, minutes: Option<u32>) {
            self.active = desired.into_iter().collect();
            for id in &self.active {
                self.r.opened(
                    id,
                    self.presence[id].broadcast_id.as_ref().unwrap(),
                    minutes,
                );
            }
        }
        fn expire(&mut self, id: &str) {
            self.r.turns.get_mut(id).unwrap().elapsed = Duration::from_secs(60);
        }
        fn assert_set(actual: Vec<String>, expected: &[&str]) {
            assert_eq!(
                actual.into_iter().collect::<HashSet<_>>(),
                expected.iter().map(|s| (*s).into()).collect()
            );
        }
    }
    #[test]
    fn always_is_existing_top_k_for_every_live_subset_and_cap() {
        for mask in 0..16 {
            for cap in 0..6 {
                let mut h = Harness::new(&["a", "b", "c", "d"], cap);
                for (i, name) in ["a", "b", "c", "d"].iter().enumerate() {
                    if mask & (1 << i) == 0 {
                        h.presence.remove(*name);
                    }
                }
                let expected = crate::select(&h.ranked, &h.presence, &h.active, &h.skips, cap);
                assert_eq!(h.desired(true), expected);
                h.apply(expected.clone(), None);
                assert_eq!(h.desired(true), expected);
            }
        }
    }
    #[test]
    fn single_channel_stays_overdue_without_churn() {
        let mut h = Harness::new(&["a"], 1);
        let desired = h.desired(true);
        h.apply(desired, Some(1));
        h.expire("a");
        for _ in 0..20 {
            assert_eq!(h.desired(true), vec!["a"]);
            assert!(h.r.pending.is_none());
        }
    }
    #[test]
    fn one_slot_wraps_full_budgets() {
        let mut h = Harness::new(&["a", "b"], 1);
        let desired = h.desired(true);
        h.apply(desired, Some(1));
        for (source, target) in [("a", "b"), ("b", "a"), ("a", "b")] {
            h.expire(source);
            let desired = h.desired(true);
            assert_eq!(desired, vec![target]);
            // Closing reserves capacity: policy can retain target but cannot begin another wave.
            assert_eq!(h.r.pending, Some((source.into(), target.into())));
            h.apply(desired, Some(1));
            assert_eq!(h.r.turns[target].elapsed, Duration::ZERO);
        }
    }
    #[test]
    fn three_channels_two_slots_follow_no_bounce_wave() {
        let mut h = Harness::new(&["a", "b", "c"], 2);
        let desired = h.desired(true);
        h.apply(desired, Some(1));
        h.expire("a");
        h.expire("b");
        let desired = h.desired(true);
        Harness::assert_set(desired.clone(), &["b", "c"]);
        h.apply(desired, Some(1));
        for _ in 0..5 {
            Harness::assert_set(h.desired(true), &["b", "c"]);
            assert!(h.r.pending.is_none());
        }
        h.expire("c");
        let desired = h.desired(true);
        Harness::assert_set(desired.clone(), &["a", "c"]);
        h.apply(desired, Some(1));
        Harness::assert_set(h.desired(true), &["a", "c"]);
        h.expire("a");
        h.r.turns.get_mut("c").unwrap().elapsed = Duration::from_secs(120);
        let desired = h.desired(true);
        Harness::assert_set(desired.clone(), &["a", "b"]);
    }
    #[test]
    fn offline_or_stale_alternative_waits_then_fresh_live_yields() {
        let mut h = Harness::new(&["a", "b"], 1);
        h.presence.get_mut("b").unwrap().stale();
        let desired = h.desired(true);
        h.apply(desired, Some(1));
        h.expire("a");
        assert_eq!(h.desired(true), vec!["a"]);
        h.presence.get_mut("b").unwrap().fresh = true;
        assert_eq!(h.desired(true), vec!["b"]);
    }
    #[test]
    fn unlimited_rotated_target_stays_selected() {
        let mut h = Harness::new(&["a", "b"], 1);
        let desired = h.desired(true);
        h.apply(desired, Some(1));
        h.expire("a");
        let desired = h.desired(true);
        h.apply(desired, None);
        assert_eq!(h.desired(true), vec!["b"]);
        assert!(!h.r.ready_to_wrap());
    }
    #[test]
    fn failed_candidates_excluded_before_capacity_truncation() {
        let mut h = Harness::new(&["a", "b", "c"], 1);
        h.failed.insert("a".into());
        assert_eq!(h.desired(true), vec!["b"]);
        h.failed.insert("b".into());
        assert_eq!(h.desired(true), vec!["c"]);
    }
    #[test]
    fn target_first_miss_during_close_retargets_and_return_does_not_preempt() {
        let mut h = Harness::new(&["a", "b", "c"], 1);
        let desired = h.desired(true);
        h.apply(desired, Some(1));
        h.expire("a");
        assert_eq!(h.desired(true), vec!["b"]);
        // Native A is still closing/reserving capacity, absent from non-closing set.
        h.active.clear();
        h.presence.get_mut("b").unwrap().observe(None);
        assert_eq!(h.desired(false), vec!["c"]);
        assert_eq!(h.r.pending, Some(("a".into(), "c".into())));
        // Destruction acknowledged; actual successful open resolves pending C.
        let desired = h.desired(false);
        h.apply(desired, Some(1));
        assert!(h.r.pending.is_none());
        h.presence.get_mut("b").unwrap().observe(Some("b-1"));
        assert_eq!(h.desired(true), vec!["c"]);
        h.expire("c");
        assert_eq!(h.desired(true), vec!["a"]);
    }
    #[test]
    fn target_first_miss_without_alternative_recovers_overdue_source() {
        let mut h = Harness::new(&["a", "b"], 1);
        let desired = h.desired(true);
        h.apply(desired, Some(1));
        h.expire("a");
        assert_eq!(h.desired(true), vec!["b"]);
        h.active.clear();
        h.presence.get_mut("b").unwrap().observe(None);
        assert_eq!(h.desired(false), vec!["a"]);
        assert!(h.r.pending.is_none());
        let desired = h.desired(false);
        h.apply(desired, Some(1));
        assert!(h.r.overdue("a"));
        h.presence.get_mut("b").unwrap().observe(Some("b-1"));
        assert_eq!(h.desired(true), vec!["b"]);
    }
    #[test]
    fn chained_stale_targets_recover_without_permanent_bypass_deferrals() {
        let mut h = Harness::new(&["a", "b", "c"], 1);
        let desired = h.desired(true);
        h.apply(desired, Some(1));
        h.expire("a");
        h.desired(true);
        h.active.clear();
        h.presence.get_mut("b").unwrap().stale();
        assert_eq!(h.desired(false), vec!["c"]);
        h.presence.get_mut("c").unwrap().stale();
        assert_eq!(h.desired(false), vec!["a"]);
        let desired = h.desired(false);
        h.apply(desired, Some(1));
        assert!(h.r.overdue("a"));
        h.presence.get_mut("b").unwrap().fresh = true;
        assert_eq!(h.desired(true), vec!["b"]);
    }
    #[test]
    fn pending_target_is_pinned_to_actual_open_after_rank_scan_wraps() {
        let mut h = Harness::new(&["a", "b", "c"], 1);
        h.r.begin("a".into(), "c".into());
        assert_eq!(h.desired(false), vec!["c"]);
        let desired = h.desired(false);
        h.apply(desired, Some(1));
        assert!(h.r.pending.is_none());
    }
    #[test]
    fn five_channels_three_slots_keep_other_deferred_members_out() {
        let mut h = Harness::new(&["a", "b", "c", "d", "e"], 3);
        let desired = h.desired(true);
        h.apply(desired, Some(1));
        for id in ["a", "b", "c"] {
            h.expire(id);
        }
        let desired = h.desired(true);
        Harness::assert_set(desired.clone(), &["b", "c", "d"]);
        h.apply(desired, Some(1));
        let desired = h.desired(true);
        Harness::assert_set(desired.clone(), &["c", "d", "e"]);
        h.apply(desired, Some(1));
        Harness::assert_set(h.desired(true), &["c", "d", "e"]);
        h.expire("d");
        h.r.turns.get_mut("c").unwrap().elapsed = Duration::from_secs(120);
        let desired = h.desired(true);
        Harness::assert_set(desired.clone(), &["a", "d", "e"]);
        h.apply(desired, Some(1));
        Harness::assert_set(h.desired(true), &["a", "d", "e"]);
    }
    #[test]
    fn eligibility_change_releases_deferral_without_resetting_retained_budget() {
        let mut r = Rotation::default();
        r.observe(vec![("a".into(), "1".into()), ("b".into(), "1".into())]);
        r.opened("a", "1", Some(1));
        r.turns.get_mut("a").unwrap().elapsed = Duration::from_secs(60);
        r.begin("a".into(), "b".into());
        r.opened("b", "1", Some(1));
        r.observe(vec![("a".into(), "1".into()), ("b".into(), "1".into())]);
        assert!(r.deferred("a"));
        r.observe(vec![
            ("a".into(), "1".into()),
            ("b".into(), "1".into()),
            ("c".into(), "1".into()),
        ]);
        assert!(!r.deferred("a"));
        assert_eq!(r.turns["b"].remaining(), Some(Duration::from_secs(60)));
        r = Rotation::default();
        assert!(r.turns.is_empty());
        assert!(r.pending.is_none());
    }
    #[test]
    fn closing_blocks_another_rotation_and_skip_wins() {
        let mut h = Harness::new(&["a", "b", "c"], 2);
        let desired = h.desired(true);
        h.apply(desired, Some(1));
        h.expire("a");
        h.expire("b");
        Harness::assert_set(h.desired(false), &["a", "b"]);
        assert!(h.r.pending.is_none());
        h.skips.insert("a".into(), "a-1".into());
        Harness::assert_set(h.desired(true), &["b", "c"]);
    }
    #[test]
    fn higher_priority_preemption_and_stale_retention_remain() {
        let mut h = Harness::new(&["a", "b", "c"], 1);
        h.presence.get_mut("a").unwrap().broadcast_id = None;
        let desired = h.desired(true);
        h.apply(desired, Some(1));
        h.presence.get_mut("b").unwrap().stale();
        assert_eq!(h.desired(true), vec!["b"]);
        h.presence.get_mut("a").unwrap().observe(Some("a-2"));
        assert_eq!(h.desired(true), vec!["a"]);
    }
    #[test]
    fn capacity_increase_fills_before_expiry_and_reorder_keeps_elapsed() {
        let mut h = Harness::new(&["a", "b", "c"], 1);
        let desired = h.desired(true);
        h.apply(desired, Some(1));
        h.expire("a");
        h.cap = 2;
        Harness::assert_set(h.desired(true), &["a", "b"]);
        assert!(h.r.pending.is_none());
        h.r.clear_round();
        h.ranked.reverse();
        assert_eq!(h.r.turns["a"].elapsed, Duration::from_secs(60));
        Harness::assert_set(h.desired(true), &["b", "c"]);
    }
}
