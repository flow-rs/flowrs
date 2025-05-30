use crate::flow::flow::Flow;

use super::infrastructure_config::InfrastructureConfig;
use super::scheduling_config::SchedulingConfig;

pub trait Scheduler {
    /// Creates a new instance from the given infrastructure configuration.
    fn new(infra: InfrastructureConfig) -> Self
    where
        Self: Sized;

    /// Performs scheduling based on the flow and infrastructure.
    fn schedule(&self, flow: &Flow) -> SchedulingConfig;
}
