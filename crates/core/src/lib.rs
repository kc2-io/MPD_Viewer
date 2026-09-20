//! Pure scheduling policy. No browser, networking, storage, clock, or runtime dependencies.
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Favorite {
    pub login: String,
    pub enabled: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Presence {
    pub broadcast_id: Option<String>,
    pub viewer_count: Option<u32>,
    pub missing_polls: u8,
    pub fresh: bool,
}

impl Presence {
    /// Only call for a complete, successful observation of this channel's batch.
    pub fn observe(&mut self, broadcast: Option<&str>) {
        self.fresh = true;
        self.viewer_count = None;
        if let Some(id) = broadcast {
            self.broadcast_id = Some(id.to_owned());
            self.missing_polls = 0;
        } else {
            self.missing_polls = self.missing_polls.saturating_add(1);
            if self.missing_polls >= 2 {
                self.broadcast_id = None;
            }
        }
    }

    /// Counts are metadata for a positive observation, never a selection signal.
    pub fn observe_stream(&mut self, broadcast: Option<&str>, viewer_count: Option<u32>) {
        self.observe(broadcast);
        self.viewer_count = broadcast.and(viewer_count);
    }

    pub fn stale(&mut self) {
        self.fresh = false;
    }
}

/// Fresh positive observations can open players. Stale/one-miss observations can
/// retain an existing player but cannot open a new one. Skips are broadcast-scoped.
pub fn select(
    ranked: &[Favorite],
    presence: &HashMap<String, Presence>,
    existing: &HashSet<String>,
    skipped: &HashMap<String, String>,
    limit: usize,
) -> Vec<String> {
    let mut seen = HashSet::new();
    ranked
        .iter()
        .filter(|f| f.enabled && seen.insert(f.login.clone()))
        .filter(|f| {
            let Some(p) = presence.get(&f.login) else { return false };
            let Some(id) = &p.broadcast_id else { return false };
            if skipped.get(&f.login) == Some(id) { return false; }
            existing.contains(&f.login) || (p.fresh && p.missing_polls == 0)
        })
        .take(limit)
        .map(|f| f.login.clone())
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diff {
    pub close: Vec<String>,
    pub keep: Vec<String>,
    pub open: Vec<String>,
}

pub fn diff(current: &[String], desired: &[String]) -> Diff {
    let old: HashSet<_> = current.iter().collect();
    let new: HashSet<_> = desired.iter().collect();
    Diff {
        close: current.iter().filter(|id| !new.contains(id)).cloned().collect(),
        keep: desired.iter().filter(|id| old.contains(id)).cloned().collect(),
        open: desired.iter().filter(|id| !old.contains(id)).cloned().collect(),
    }
}

/// Strictly parse a login or a Twitch channel URL. Never accepts an arbitrary URL.
pub fn normalize_login(input: &str) -> Result<String, &'static str> {
    let trimmed = input.trim();
    let lower = trimmed.to_ascii_lowercase();
    let mut value = lower.as_str();
    let mut is_url = false;
    for prefix in ["https://www.twitch.tv/", "http://www.twitch.tv/",
        "https://twitch.tv/", "http://twitch.tv/", "www.twitch.tv/", "twitch.tv/"] {
        if let Some(rest) = value.strip_prefix(prefix) {
            value = rest;
            is_url = true;
            break;
        }
    }
    if is_url {
        value = value.split(['?', '#']).next().unwrap_or("").trim_end_matches('/');
    } else {
        value = value.strip_prefix('@').unwrap_or(value);
    }
    if value.is_empty() || value.len() > 25
        || !value.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_') {
        return Err("Use a Twitch login or a direct twitch.tv/channel URL (1–25 letters, digits, or underscores).");
    }
    Ok(value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn f(names: &[&str]) -> Vec<Favorite> {
        names.iter().map(|s| Favorite { login: s.to_string(), enabled: true }).collect()
    }
    fn live(names: &[&str]) -> HashMap<String, Presence> {
        names.iter().map(|s| (s.to_string(), Presence {
            broadcast_id: Some(format!("{s}-1")), missing_polls: 0, fresh: true, viewer_count: None,
        })).collect()
    }
    fn strings(names: &[&str]) -> Vec<String> { names.iter().map(|s| s.to_string()).collect() }
    #[test] fn top_three_not_all_live() {
        assert_eq!(select(&f(&["a","b","c","d"]), &live(&["b","c","d"]),
            &HashSet::new(), &HashMap::new(), 3), strings(&["b","c","d"]));
    }
    #[test] fn higher_priority_preempts() {
        let result = select(&f(&["a","b","c","d"]), &live(&["a","b","c","d"]),
            &strings(&["b","c","d"]).into_iter().collect(), &HashMap::new(), 3);
        let d = diff(&strings(&["b","c","d"]), &result);
        assert_eq!(d.close, strings(&["d"]));
        assert_eq!(d.keep, strings(&["b","c"]));
        assert_eq!(d.open, strings(&["a"]));
    }
    #[test] fn reordering_retains_players() {
        let d = diff(&strings(&["a","b"]), &strings(&["b","a"]));
        assert!(d.close.is_empty() && d.open.is_empty());
        assert_eq!(d.keep, strings(&["b","a"]));
    }
    #[test] fn two_successful_misses_confirm_offline() {
        let mut p = live(&["a"]).remove("a").unwrap();
        p.observe(None); assert!(p.broadcast_id.is_some());
        p.observe(None); assert!(p.broadcast_id.is_none());
    }
    #[test] fn failure_is_not_an_offline_observation() {
        let mut p = live(&["a"]).remove("a").unwrap();
        p.stale(); assert_eq!(p.missing_polls, 0);
        assert!(p.broadcast_id.is_some());
    }
    #[test] fn stale_can_retain_but_not_open() {
        let mut p = live(&["a"]); p.get_mut("a").unwrap().stale();
        assert!(select(&f(&["a"]), &p, &HashSet::new(), &HashMap::new(), 3).is_empty());
        assert_eq!(select(&f(&["a"]), &p, &strings(&["a"]).into_iter().collect(),
            &HashMap::new(), 3), strings(&["a"]));
    }
    #[test] fn one_miss_cannot_open_new_player() {
        let mut p = live(&["a"]); p.get_mut("a").unwrap().observe(None);
        assert!(select(&f(&["a"]), &p, &HashSet::new(), &HashMap::new(), 3).is_empty());
    }
    #[test] fn skip_expires_on_new_broadcast() {
        let skips = HashMap::from([("a".into(), "a-1".into())]);
        let mut p = live(&["a"]);
        assert!(select(&f(&["a"]), &p, &HashSet::new(), &skips, 3).is_empty());
        p.get_mut("a").unwrap().observe(Some("a-2"));
        assert_eq!(select(&f(&["a"]), &p, &HashSet::new(), &skips, 3), strings(&["a"]));
    }
    #[test] fn disabled_is_not_selected() {
        let mut list = f(&["a","b"]); list[0].enabled = false;
        assert_eq!(select(&list, &live(&["a","b"]), &HashSet::new(), &HashMap::new(), 3), strings(&["b"]));
    }
    #[test] fn duplicate_defense() {
        assert_eq!(select(&f(&["a","a","b"]), &live(&["a","b"]), &HashSet::new(),
            &HashMap::new(), 3), strings(&["a","b"]));
    }
    #[test] fn zero_limit_is_safe() {
        assert!(select(&f(&["a"]), &live(&["a"]), &HashSet::new(), &HashMap::new(), 0).is_empty());
    }
    #[test] fn cap_exceeding_favorites_does_not_add_sessions() {
        assert_eq!(select(&f(&["a"]), &live(&["a"]), &HashSet::new(), &HashMap::new(), 999), strings(&["a"]));
    }
    #[test] fn all_small_live_subsets_obey_order_and_capacity() {
        let names = ["a","b","c","d","e"];
        for mask in 0u32..32 {
            let online: Vec<_> = names.iter().enumerate().filter(|(i,_)| mask & (1 << i) != 0).map(|(_,s)| *s).collect();
            for cap in 0..7 {
                let selected = select(&f(&names), &live(&online), &HashSet::new(), &HashMap::new(), cap);
                assert_eq!(selected, online.iter().take(cap).map(|s| s.to_string()).collect::<Vec<_>>());
            }
        }
    }
    #[test] fn login_normalization() {
        for value in [" ModPackDad ", "@ModPackDad", "https://www.twitch.tv/ModPackDad/?x=1"] {
            assert_eq!(normalize_login(value), Ok("modpackdad".into()));
        }
    }
    #[test] fn reject_non_channel_inputs() {
        for value in ["", "https://evil.example/a", "twitch.tv.evil.example/a",
            "https://twitch.tv/a/videos", "a b", "<script>", "a/b", "éclair"] {
            assert!(normalize_login(value).is_err(), "{value}");
        }
    }
}


#[cfg(test)]
mod audience_tests {
    use super::*;
    #[test]
    fn counts_follow_complete_observations_and_clear_on_first_miss() {
        let mut p = Presence::default();
        p.observe_stream(Some("broadcast-1"), Some(0));
        assert_eq!(p.viewer_count, Some(0));
        p.observe_stream(Some("broadcast-1"), Some(1234));
        p.stale();
        assert_eq!(p.viewer_count, Some(1234));
        assert!(!p.fresh);
        p.observe_stream(None, None);
        assert_eq!(p.viewer_count, None);
        assert_eq!(p.broadcast_id.as_deref(), Some("broadcast-1"));
        p.observe_stream(None, None);
        assert_eq!(p.broadcast_id, None);
        p.observe_stream(Some("broadcast-2"), None);
        assert_eq!(p.viewer_count, None);
        assert!(p.fresh);
    }
    #[test]
    fn counts_cannot_override_rank_or_enable_a_non_live_channel() {
        let ranked = vec![Favorite { login: "alpha".into(), enabled: true },
            Favorite { login: "beta".into(), enabled: true }];
        let mut alpha = Presence::default();
        let mut beta = Presence::default();
        alpha.observe_stream(Some("a1"), Some(0));
        beta.observe_stream(Some("b1"), Some(u32::MAX));
        let presence = HashMap::from([("alpha".into(), alpha), ("beta".into(), beta)]);
        assert_eq!(select(&ranked, &presence, &HashSet::new(), &HashMap::new(), 1), vec!["alpha"]);
        let mut offline = Presence::default();
        offline.observe_stream(None, Some(100));
        assert_eq!(offline.viewer_count, None);
    }
    #[test]
    fn demo_observation_has_no_audience_metadata() {
        let mut p = Presence::default();
        p.observe_stream(Some("a1"), Some(100));
        p.observe(Some("demo-1"));
        assert_eq!(p.viewer_count, None);
    }
}
