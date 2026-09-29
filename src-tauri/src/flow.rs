//! Flow times, read from a topic's events: lead time runs from when it was written down,
//! cycle time from when it first became active, both until it is done (or until now for
//! a topic still going). Blocked time is the part of the cycle spent blocked.

use serde::Serialize;

use crate::model::{Slot, Stage, Time, Topic};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Times {
    /// Seconds from written down to done, or to now.
    pub lead: Time,
    /// Seconds from first active to done, or to now; `None` before it starts.
    pub cycle: Option<Time>,
    /// Seconds blocked, open blocks counted up to now.
    pub blocked: Time,
}

pub fn times(topic: &Topic, now: Time) -> Times {
    let end = topic.finished_at.unwrap_or(now);
    let blocked = topic
        .blocks
        .iter()
        .map(|b| (b.until.unwrap_or(end).min(end) - b.since).max(0))
        .sum();
    Times {
        lead: (end - topic.created_at).max(0),
        cycle: topic.started_at.map(|s| (end - s).max(0)),
        blocked,
    }
}

/// What the done topics of one slot say about how work flows through it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SlotStats {
    pub slot: Slot,
    /// Topics done within the window.
    pub done: usize,
    /// Topics started and not done.
    pub in_progress: usize,
    pub lead_median: Option<Time>,
    pub cycle_median: Option<Time>,
    pub cycle_p85: Option<Time>,
    /// The share of the cycle time spent blocked, over the done topics.
    pub blocked_share: Option<f64>,
    /// How many times work came back on the done topics.
    pub reworks: usize,
    /// Done topics that came back at least once.
    pub reworked: usize,
}

/// The stats of each slot, counting topics done in the last `days` days.
pub fn stats(topics: &[Topic], now: Time, days: u32) -> Vec<SlotStats> {
    let since = now - Time::from(days) * 86_400;
    Slot::ALL
        .iter()
        .map(|&slot| {
            let of_slot = topics.iter().filter(|t| t.slot == slot);
            let done: Vec<&Topic> = of_slot
                .clone()
                .filter(|t| t.stage == Stage::Done && t.finished_at.is_some_and(|f| f >= since))
                .collect();
            let in_progress = of_slot
                .filter(|t| t.stage != Stage::Done && t.started_at.is_some())
                .count();
            let all: Vec<Times> = done.iter().map(|t| times(t, now)).collect();
            let mut leads: Vec<Time> = all.iter().map(|t| t.lead).collect();
            let mut cycles: Vec<Time> = all.iter().filter_map(|t| t.cycle).collect();
            let cycle_total: Time = cycles.iter().sum();
            let blocked_total: Time = all.iter().map(|t| t.blocked).sum();
            SlotStats {
                slot,
                done: done.len(),
                in_progress,
                lead_median: percentile(&mut leads, 50),
                cycle_median: percentile(&mut cycles, 50),
                cycle_p85: percentile(&mut cycles, 85),
                blocked_share: (cycle_total > 0).then(|| blocked_total as f64 / cycle_total as f64),
                reworks: done.iter().map(|t| t.reworks.len()).sum(),
                reworked: done.iter().filter(|t| !t.reworks.is_empty()).count(),
            }
        })
        .collect()
}

/// The nearest-rank percentile `p` of `values`.
fn percentile(values: &mut [Time], p: usize) -> Option<Time> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    let rank = (p * values.len()).div_ceil(100).max(1);
    Some(values[rank - 1])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::tests::topic;
    use crate::model::{Block, Model, Rework};

    const DAY: Time = 86_400;

    fn done(model: &mut Model, created: Time, started: Time, finished: Time) -> Topic {
        let mut t = topic(model, "t", Slot::Feature);
        t.stage = Stage::Done;
        t.created_at = created;
        t.started_at = Some(started);
        t.finished_at = Some(finished);
        t
    }

    #[test]
    fn lead_and_cycle_stop_when_done_and_blocks_count_within_them() {
        let mut model = Model::default();
        let mut t = done(&mut model, 0, 2 * DAY, 5 * DAY);
        t.blocks = vec![
            Block {
                id: 9,
                reason: "review".into(),
                since: 3 * DAY,
                until: Some(4 * DAY),
            },
            Block {
                id: 10,
                reason: "left open".into(),
                since: 4 * DAY + DAY / 2,
                until: None,
            },
        ];
        let times = times(&t, 100 * DAY);
        assert_eq!(times.lead, 5 * DAY);
        assert_eq!(times.cycle, Some(3 * DAY));
        assert_eq!(times.blocked, DAY + DAY / 2);
    }

    #[test]
    fn a_topic_not_started_has_no_cycle_yet() {
        let mut model = Model::default();
        let t = topic(&mut model, "t", Slot::Feature);
        assert_eq!(times(&t, 10).cycle, None);
        assert_eq!(times(&t, 10).lead, 10);
    }

    #[test]
    fn stats_read_the_done_topics_of_the_window() {
        let mut model = Model::default();
        let mut topics = vec![
            done(&mut model, 0, DAY, 2 * DAY),
            done(&mut model, 0, DAY, 4 * DAY),
            done(&mut model, 0, DAY, 11 * DAY),
        ];
        topics[1].reworks.push(Rework {
            id: 20,
            reason: "changes requested".into(),
            at: 3 * DAY,
        });
        let mut started = topic(&mut model, "going", Slot::Feature);
        started.started_at = Some(DAY);
        topics.push(started);
        topics.push(done(&mut model, 0, 0, 0));

        let feature = &stats(&topics, 12 * DAY, 11)[0];
        assert_eq!(feature.done, 3);
        assert_eq!(feature.in_progress, 1);
        assert_eq!(feature.cycle_median, Some(3 * DAY));
        assert_eq!(feature.cycle_p85, Some(10 * DAY));
        assert_eq!(feature.lead_median, Some(4 * DAY));
        assert_eq!(feature.blocked_share, Some(0.0));
        assert_eq!((feature.reworks, feature.reworked), (1, 1));
    }
}
