use anyhow::Result;
use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;

use super::scheduling_types::RuntimeId;

#[derive(Debug, Deserialize)]
pub struct InfrastructureConfig {
    pub machines: Vec<MachineConfig>,
}

#[derive(Debug, Deserialize)]
pub struct MachineConfig {
    pub ip: String,
    pub runtime_id: RuntimeId,
    pub capabilities: Vec<String>,
}

impl InfrastructureConfig {
    pub fn from_yaml_file(path: &str) -> Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let config: InfrastructureConfig = serde_yaml::from_reader(reader)?;
        Ok(config)
    }

    /// Returns a MachineConfig by the given runtime ID
    pub fn get_machine_by_runtime_id(&self, id: RuntimeId) -> Option<&MachineConfig> {
        self.machines.iter().find(|m| m.runtime_id == id)
    }

    /// Returns a list of all unique capabilities (excluding the orchestrator machine)
    pub fn get_all_capabilities(&self) -> Vec<String> {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        for m in &self.machines {
            if m.runtime_id == 0 {
                continue; // Skip orchestrator
            }
            for cap in &m.capabilities {
                set.insert(cap.clone());
            }
        }
        set.into_iter().collect()
    }

    /// Returns all machines (excluding orchestrator machine) that have *all* required capabilities
    pub fn find_machines_with_capabilities(&self, required: &[String]) -> Vec<&MachineConfig> {
        self.machines
            .iter()
            .filter(|m| m.runtime_id != 0) // Exclude orchestrator machine
            .filter(|m| required.iter().all(|cap| m.capabilities.contains(cap)))
            .collect()
    }
}
