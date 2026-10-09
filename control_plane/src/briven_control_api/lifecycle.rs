//! Fixed idle-compute policy. No credentials, SQL or commands enter the record.
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

pub(super) const DEFAULT_IDLE_SECONDS: u32 = 300;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Phase {
    Running,
    Stopping,
    Sleeping,
    Starting,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Failure {
    UnexpectedExit,
    WakeIncomplete,
    SleepRequestFailed,
    ActivityUnavailable,
    BindingChanged,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Record {
    pub endpoint_id: String,
    pub project_id: String,
    pub timeline_id: String,
    pub enabled: bool,
    pub idle_seconds: u32,
    pub last_demand: u64,
    pub changed_at: u64,
    pub phase_since: u64,
    pub generation: u64,
    pub phase: Phase,
    pub stop_requested: bool,
    pub failure: Option<Failure>,
    pub starts: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Observation {
    Ready,
    Alive,
    Absent,
    Unknown,
}

#[derive(Default, Clone, Copy)]
pub(super) struct Activity {
    pub clients: i64,
    pub prepared: i64,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Action {
    Keep,
    Sleep,
    Sleeping,
    Wake,
    Running,
    Wait,
    Review(Failure),
}

pub(super) fn valid_endpoint(value: &str) -> bool {
    value.starts_with("ep-")
        && (4..=96).contains(&value.len())
        && value
            .bytes()
            .all(|v| v.is_ascii_lowercase() || v.is_ascii_digit() || v == b'-')
        && !value.ends_with('-')
}

fn valid_id(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|v| v.is_ascii_digit() || matches!(v, b'a'..=b'f'))
}

impl Record {
    pub fn new(endpoint: &str, project: &str, timeline: &str, idle: u32, now: u64) -> Result<Self> {
        let record = Self {
            endpoint_id: endpoint.to_string(),
            project_id: project.to_string(),
            timeline_id: timeline.to_string(),
            enabled: true,
            idle_seconds: idle,
            last_demand: now,
            changed_at: now,
            phase_since: now,
            generation: 0,
            phase: Phase::Running,
            stop_requested: false,
            failure: None,
            starts: 0,
        };
        record.validate()?;
        Ok(record)
    }

    pub fn validate(&self) -> Result<()> {
        if !valid_endpoint(&self.endpoint_id)
            || !valid_id(&self.project_id)
            || !valid_id(&self.timeline_id)
            || !(60..=3600).contains(&self.idle_seconds)
            || self.generation > i64::MAX as u64
            || self.last_demand > i64::MAX as u64
            || self.changed_at > i64::MAX as u64
            || self.phase_since > i64::MAX as u64
        {
            bail!("invalid compute lifecycle record");
        }
        Ok(())
    }

    pub fn matches(&self, endpoint: &str, project: &str, timeline: &str) -> bool {
        self.endpoint_id == endpoint && self.project_id == project && self.timeline_id == timeline
    }

    pub fn transition(&mut self, phase: Phase, now: u64) {
        if self.phase != phase {
            self.phase = phase;
            self.phase_since = now;
        }
    }

    pub fn tick(&self, now: u64, observed: Observation, activity: Option<Activity>) -> Action {
        match self.phase {
            Phase::Stopping if observed == Observation::Absent => Action::Sleeping,
            Phase::Stopping if observed != Observation::Unknown && !self.stop_requested => {
                Action::Sleep
            }
            Phase::Starting if observed == Observation::Ready => Action::Running,
            Phase::Starting if observed == Observation::Absent => {
                Action::Review(Failure::WakeIncomplete)
            }
            Phase::Starting
                if now
                    .checked_sub(self.phase_since)
                    .is_some_and(|age| age >= 120) =>
            {
                Action::Review(Failure::WakeIncomplete)
            }
            Phase::Running if observed == Observation::Absent => {
                Action::Review(Failure::UnexpectedExit)
            }
            Phase::Running
                if self.enabled
                    && observed == Observation::Ready
                    && now
                        .checked_sub(self.last_demand)
                        .is_some_and(|idle| idle >= u64::from(self.idle_seconds))
                    && activity.is_some_and(|v| v.clients == 0 && v.prepared == 0) =>
            {
                Action::Sleep
            }
            _ => Action::Keep,
        }
    }

    pub fn demand(&self, now: u64, observed: Observation) -> Action {
        match (self.phase, observed) {
            (Phase::Sleeping | Phase::Stopping, Observation::Absent) => Action::Wake,
            (Phase::Starting | Phase::Running, Observation::Ready) => Action::Running,
            (Phase::Starting, Observation::Absent) => Action::Review(Failure::WakeIncomplete),
            (Phase::Starting, _)
                if now
                    .checked_sub(self.phase_since)
                    .is_some_and(|age| age >= 120) =>
            {
                Action::Review(Failure::WakeIncomplete)
            }
            (Phase::Running | Phase::Sleeping, Observation::Unknown | Observation::Absent) => {
                Action::Review(Failure::UnexpectedExit)
            }
            _ => Action::Wait,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn running() -> Record {
        Record::new(
            "ep-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            300,
            1_000,
        )
        .unwrap()
    }

    #[test]
    fn protected_sessions_prepared_transactions_and_missing_samples_prevent_sleep() {
        let record = running();
        for activity in [
            None,
            Some(Activity {
                clients: 1,
                prepared: 0,
            }),
            Some(Activity {
                clients: 0,
                prepared: 1,
            }),
        ] {
            assert_eq!(
                record.tick(2_000, Observation::Ready, activity),
                Action::Keep
            );
        }
        assert_eq!(
            record.tick(1_299, Observation::Ready, Some(Activity::default())),
            Action::Keep
        );
        assert_eq!(
            record.tick(1_300, Observation::Ready, Some(Activity::default())),
            Action::Sleep
        );
        assert_eq!(
            record.tick(900, Observation::Ready, Some(Activity::default())),
            Action::Keep
        );
    }

    #[test]
    fn demand_only_wakes_a_verified_normal_sleep_once() {
        let mut record = running();
        record.phase = Phase::Sleeping;
        assert_eq!(record.demand(1_500, Observation::Absent), Action::Wake);
        record.phase = Phase::Starting;
        record.phase_since = 1_500;
        assert_eq!(
            record.demand(1_501, Observation::Absent),
            Action::Review(Failure::WakeIncomplete)
        );
        assert_eq!(record.demand(1_501, Observation::Alive), Action::Wait);
        assert_eq!(record.demand(1_501, Observation::Ready), Action::Running);
        record.phase = Phase::Stopping;
        assert_eq!(record.demand(1_501, Observation::Alive), Action::Wait);
        assert_eq!(record.demand(1_501, Observation::Absent), Action::Wake);
    }

    #[test]
    fn unexpected_exit_and_unknown_identity_are_never_automatic_restarts() {
        let record = running();
        for observation in [Observation::Absent, Observation::Unknown] {
            assert_eq!(
                record.demand(2_000, observation),
                Action::Review(Failure::UnexpectedExit)
            );
            assert_ne!(
                record.tick(2_000, observation, Some(Activity::default())),
                Action::Sleep
            );
        }
        let mut failed = running();
        failed.phase = Phase::Failed;
        assert_eq!(failed.demand(2_000, Observation::Ready), Action::Wait);
        failed.enabled = false;
        assert_eq!(
            failed.tick(2_000, Observation::Ready, Some(Activity::default())),
            Action::Keep
        );
    }

    #[test]
    fn hung_start_has_a_fixed_review_deadline_that_demand_cannot_extend() {
        let mut record = running();
        record.transition(Phase::Starting, 1500);
        record.last_demand = 1700;
        record.changed_at = 1700;
        assert_eq!(
            record.tick(1700, Observation::Alive, None),
            Action::Review(Failure::WakeIncomplete)
        );
        assert_eq!(
            record.demand(1700, Observation::Alive),
            Action::Review(Failure::WakeIncomplete)
        );
        assert_eq!(record.demand(1700, Observation::Ready), Action::Running);
        record.transition(Phase::Running, 1700);
        assert_eq!(record.phase_since, 1700);
    }

    #[test]
    fn stopping_reconciles_only_actual_process_exit_and_uncertain_dispatch_is_safe() {
        let mut record = running();
        record.phase = Phase::Stopping;
        assert_eq!(record.tick(2_000, Observation::Alive, None), Action::Sleep);
        record.stop_requested = true;
        assert_eq!(record.tick(2_000, Observation::Alive, None), Action::Keep);
        assert_eq!(record.tick(2_000, Observation::Unknown, None), Action::Keep);
        assert_eq!(
            record.tick(2_000, Observation::Absent, None),
            Action::Sleeping
        );
    }

    #[test]
    fn records_bind_exact_endpoint_and_timeline_and_validate_bounded_configuration() {
        let record = running();
        assert!(record.matches(&record.endpoint_id, &record.project_id, &record.timeline_id));
        assert!(!record.matches(
            &record.endpoint_id,
            "cccccccccccccccccccccccccccccccc",
            &record.timeline_id
        ));
        for idle in [0, 59, 3_601, u32::MAX] {
            assert!(
                Record::new(
                    &record.endpoint_id,
                    &record.project_id,
                    &record.timeline_id,
                    idle,
                    1
                )
                .is_err()
            );
        }
        assert!(
            Record::new(
                "../../other",
                &record.project_id,
                &record.timeline_id,
                300,
                1
            )
            .is_err()
        );
        assert!(Record::new(&record.endpoint_id, "invalid", &record.timeline_id, 300, 1).is_err());
    }
}
