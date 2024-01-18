use std::{
    env,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use anyhow::{Context as AnyhowContext, Result};
use metrics::increment_counter;
#[cfg(feature = "metrics")]
use metrics_exporter_prometheus::PrometheusBuilder;
use thiserror::Error;
use tracing::metadata::LevelFilter;
use tracing::{error, info_span};

#[cfg(feature = "tracing")]
use crate::analytics::otlp_exporter::OtlpExporter;
use crate::{
    exec::{
        execution_controller::ExecutionController,
        execution_state::ExecutionState,
        node_updater::{NodeUpdateError, NodeUpdater, SleepMode},
    },
    flow::flow::Flow,
    node::ChangeObserver,
    scheduler::{Scheduler, SchedulingInfo},
};

cfg_if::cfg_if! {
    if #[cfg(feature = "tracing")] {
        use opentelemetry::trace::TracerProvider as _;
        use opentelemetry_sdk::trace::TracerProvider;
        use opentelemetry_sdk::Resource;
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::Registry;
    }
}
pub struct ExecutionContext {
    pub executor: StandardExecutor,
    pub flow: Flow,
}

impl ExecutionContext {
    pub fn new(executor: StandardExecutor, flow: Flow) -> Self {
        Self {
            executor: executor,
            flow: flow,
        }
    }
}

#[repr(C)]
pub struct ExecutionContextHandle {
    _data: [u8; 0],
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

pub trait Executor {
    fn run<S, U>(&mut self, flow: Flow, scheduler: S, node_updater: U) -> Result<()>
    where
        S: Scheduler + std::marker::Send,
        U: NodeUpdater + Drop;

    fn controller(&self) -> Arc<Mutex<ExecutionController>>;
}

#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("Errors occured while updating nodes: {errors:?}")]
    UpdateErrorCollection { errors: Vec<NodeUpdateError> },
}

pub struct StandardExecutor {
    controller: Arc<Mutex<ExecutionController>>,
    observer: ChangeObserver,
}

impl StandardExecutor {
    pub fn new(observer: ChangeObserver) -> Self {
        Self {
            controller: Arc::new(Mutex::new(ExecutionController::new(
                observer.notifier.clone(),
            ))),
            observer,
        }
    }

    #[tracing::instrument(skip_all)]
    fn run_update_loop<S, U>(
        &mut self,
        flow: &Flow,
        mut scheduler: S,
        mut node_updater: U,
    ) -> Result<(), ExecutionError>
    where
        S: Scheduler,
        U: NodeUpdater,
    {
        self.controller
            .lock()
            .unwrap()
            .set_state(ExecutionState::Running);

        let mut info = SchedulingInfo::new(flow.num_nodes());

        let update_controllers = flow.get_update_controllers();

        while !self.controller.lock().unwrap().cancellation_requested() {
            increment_counter!("flowrs.executions");
            // Run an epoch (an update of each node).
            scheduler.restart_epoch(&mut info);

            //println!("                                                                                                    {:?} NEW EPOCH", std::thread::current().id());
            while !scheduler.epoch_is_over(&mut info) {
                let node_idx = scheduler.get_next_node_idx();
                //println!("                                                                                                    {:?} {}", std::thread::current().id(), node_idx);

                let node = flow.node_by_index(node_idx);
                if let Some(n) = node {
                    let description = flow.node_description_by_id(n.0);
                    node_updater.update(n.clone(), description.cloned());
                }
            }

            // Sleep if necessary.
            {
                let _sleep_span = info_span!("sleep").entered();
                match node_updater.sleep_mode() {
                    SleepMode::None => {}

                    SleepMode::Reactive => {
                        self.controller
                            .lock()
                            .unwrap()
                            .set_state(ExecutionState::Sleeping);

                        self.observer.wait_for_changes();

                        self.controller
                            .lock()
                            .unwrap()
                            .set_state(ExecutionState::Running);
                    }

                    SleepMode::FixedFrequency(fps) => {
                        let actual_duration = info.epoch_duration;
                        let target_duration = Duration::from_millis(1000 / fps);
                        let delta = target_duration.saturating_sub(actual_duration);
                        //println!("AD: {:?} TD: {:?} DELTA: {:?}", actual_duration, target_duration, delta);
                        if delta > Duration::ZERO {
                            thread::sleep(delta);
                        }
                    }
                }
            }

            // Check if async errors occured.
            let errors: Vec<NodeUpdateError> = node_updater
                .errors()
                .into_iter()
                .map(|mut err| {
                    if let Some(id) = err.node_id {
                        err.node_id = Some(id);
                        if let Some(desc) = flow.node_description_by_id(id) {
                            err.node_desc = Some(desc.clone());
                        }
                    }
                    err
                })
                .collect();
            if !errors.is_empty() {
                return Err(ExecutionError::UpdateErrorCollection { errors });
            }
        }

        // Cancel long-running node updates.
        update_controllers
            .iter()
            .for_each(|uc| uc.lock().unwrap().cancel());

        // Drop node updater which destroys all workers.
        drop(node_updater);

        // All done.
        self.controller
            .lock()
            .unwrap()
            .set_state(ExecutionState::Ready);

        Ok(())
    }
}

impl Executor for StandardExecutor {
    fn run<S, U>(&mut self, flow: Flow, scheduler: S, node_updater: U) -> Result<(), anyhow::Error>
    where
        S: Scheduler + std::marker::Send,
        U: NodeUpdater + Drop,
    {
        let runner = || {
            // Trace executed code
            // Spans will be sent to the configured OpenTelemetry exporter
            let _root = info_span!("executor_run").entered();

            //TODO: Fix error flow.

            flow.init_all()
                .context(format!("Unable to init all nodes."));

            flow.ready_all()
                .context(format!("Unable to make all nodes ready."));

            self.run_update_loop(&flow, scheduler, node_updater);

            flow.shutdown_all()
                .context(format!("Unable to shutdown all nodes"));

            #[cfg(feature = "metrics")]
            {
                let pid = std::process::id().to_string();
                let client = reqwest::blocking::Client::new();
                let pushgateway_host =
                    env::var("PUSHGATEWAY_HOST").unwrap_or("http://localhost:9091".to_string());
                let url = format!("{pushgateway_host}/metrics/job/flowrs-{pid}");
                let _ = client.delete(&url).send();
            }

            Ok(())
        };

        #[cfg(feature = "metrics")]
        {
            let pid = std::process::id().to_string();

            let pushgateway_host =
                env::var("PUSHGATEWAY_HOST").unwrap_or("http://localhost:9091".to_string());

            PrometheusBuilder::new()
                .add_global_label("pid", &pid)
                .with_push_gateway(
                    format!("{pushgateway_host}/metrics/job/flowrs-{pid}"),
                    Duration::from_secs(1),
                    None,
                    None,
                )
                .expect("Invalid push gateway configuration")
                .install()
                .expect("failed to install recorder/exporter");
        }

        #[cfg(feature = "tracing")]
        {
            // Create a resource configuration
            let resource =
                Resource::new(vec![opentelemetry::KeyValue::new("service.name", "flowrs")]);

            let tempo_host =
                env::var("TEMPO_HOST").unwrap_or("http://localhost:4318/v1/traces".to_string());

            // Create a new OpenTelemetry trace pipeline that prints to stdout
            let provider = TracerProvider::builder()
                .with_config(opentelemetry_sdk::trace::Config::default().with_resource(resource))
                .with_simple_exporter(OtlpExporter::new(tempo_host))
                .build();
            let tracer = provider.tracer("flowrs");

            // Create a tracing layer with the configured tracer
            let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

            // Use the tracing subscriber `Registry`, or any other subscriber
            // that impls `LookupSpan`
            let subscriber = Registry::default().with(telemetry).with(LevelFilter::INFO);

            tracing::subscriber::set_global_default(subscriber)
                .expect("Failed to set the global default tracing subscriber");

            // Setup LogTracer and env_logger at the same time
            let combined_logger = crate::analytics::combined_logger::CombinedLogger::new();

            // combine both loggers together
            log::set_boxed_logger(Box::new(combined_logger))
                .map(|()| log::set_max_level(log::LevelFilter::Info))
                .expect("Failed to set logger");

            log::info!("Starting flowrs");
            return runner();
        }

        #[cfg(not(feature = "tracing"))]
        {
            env_logger::init_from_env(env_logger::Env::default().default_filter_or("info")).target(env_logger::Target::Stdout).init();

            return runner();
        }
    }

    fn controller(&self) -> Arc<Mutex<ExecutionController>> {
        self.controller.clone()
    }
}
