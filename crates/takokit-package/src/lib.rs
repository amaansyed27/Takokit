mod artifact_io;
mod artifact_reuse;
mod catalog;
mod error;
mod install_support;
mod installed;
mod model;
mod orchestrator;
mod planning;
mod records;
mod registry;
mod resolution;
mod runner;
mod runtime;
mod runtime_command;
mod runtime_onnx;
mod runtime_python;
mod runtime_python_specs;
mod runtime_uv;
mod runtime_whisper;
mod transaction;

pub use catalog::*;
pub use error::*;
pub use installed::InstalledRegistry;
pub use model::*;
pub use orchestrator::install_model_complete;
pub use records::*;
pub use registry::PackageRegistry;
pub use resolution::{
    current_platform_id, model_info_from_plan, plan_model, resolve_execution_plan, resolve_runner,
};
pub use runner::*;
pub use runtime::{initialize_runner_runtime, python_managed_runner_layout, runner_runtime_layout};
pub use runtime_python::{install_python_adapter, python_adapter_record, python_adapter_records};
pub use runtime_python_specs::adapter_for_model;
pub use runtime_uv::{bootstrap_uv, find_uv};

#[cfg(test)]
mod tests;
