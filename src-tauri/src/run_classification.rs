use agentctl_core::domain::{DetectionSource, Lifecycle, ObservedState, RunRecord};

pub fn is_restorable_run(run: &RunRecord) -> bool {
    run.lifecycle == Lifecycle::Active
        && run.observed_state == ObservedState::Unknown
        && run.detection_source == DetectionSource::Unknown
        && run.tmux_window.is_some()
}

pub fn is_stale_run(run: &RunRecord) -> bool {
    run.lifecycle == Lifecycle::Active
        && run.observed_state == ObservedState::Unknown
        && run.detection_source == DetectionSource::Unknown
        && !is_restorable_run(run)
}
