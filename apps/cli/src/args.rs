//! CLI argument and subcommand definitions.

use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use uuid::Uuid;

mod rvc;
pub(crate) use rvc::*;
pub(crate) use takokit_core::SpeechRequest;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum OutputFormat {
    Human,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum RvcF0MethodArg {
    Rmvpe,
    Harvest,
    Crepe,
    Pm,
}

impl From<RvcF0MethodArg> for takokit_core::RvcF0Method {
    fn from(value: RvcF0MethodArg) -> Self {
        match value {
            RvcF0MethodArg::Rmvpe => Self::Rmvpe,
            RvcF0MethodArg::Harvest => Self::Harvest,
            RvcF0MethodArg::Crepe => Self::Crepe,
            RvcF0MethodArg::Pm => Self::Pm,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum StorageScope {
    Uv,
    Downloads,
    Unused,
    AllSafe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum UpdateChannelArg {
    Stable,
    Preview,
}

impl UpdateChannelArg {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Preview => "preview",
        }
    }
}

#[derive(Debug, Parser)]
#[command(name = "takokit", version, about = "Local voice AI runtime")]
pub(crate) struct Cli {
    #[arg(long, global = true)]
    pub(crate) direct: bool,
    /// Override the output format. When omitted, terminal output stays human-readable.
    #[arg(long, global = true, value_enum, default_value = "human")]
    pub(crate) output: Option<OutputFormat>,
    /// Project directory whose `.tako` folder stores sessions and outputs.
    #[arg(long, global = true)]
    pub(crate) workspace: Option<PathBuf>,
    /// Resume a specific project session.
    #[arg(long, global = true)]
    pub(crate) session: Option<Uuid>,
    #[command(subcommand)]
    pub(crate) command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    Serve {
        #[arg(long, hide = true)]
        daemon_child: bool,
        #[arg(long, hide = true)]
        instance_id: Option<Uuid>,
    },
    Daemon {
        #[command(subcommand)]
        command: DaemonCommand,
    },
    Gui,
    Doctor(DoctorArgs),
    Version,
    Status,
    Storage(StorageArgs),
    Update {
        #[command(subcommand)]
        command: UpdateCommand,
    },
    Reset(ResetArgs),
    Licenses {
        #[command(subcommand)]
        command: LicenseCommand,
    },
    Capabilities,
    Models,
    Runners,
    CustomModel {
        #[command(subcommand)]
        command: CustomModelCommand,
    },
    Voice {
        #[command(subcommand)]
        command: VoiceCommand,
    },
    Library {
        #[command(subcommand)]
        target: LibraryTarget,
    },
    Speak(SpeakArgs),
    Pull(PullArgs),
    Show {
        model: String,
    },
    Plan(PlanArgs),
    Rm(RmArgs),
    List {
        #[command(subcommand)]
        target: Option<ListTarget>,
    },
    Run(RunArgs),
    Ps,
    Runner {
        #[command(subcommand)]
        command: RunnerCommand,
    },
    Adapter {
        #[command(subcommand)]
        command: AdapterCommand,
    },
    Sessions {
        #[command(subcommand)]
        command: SessionsCommand,
    },
    Quickstart(QuickstartArgs),
    Deps {
        #[command(subcommand)]
        command: DepsCommand,
    },
    Samples {
        #[command(subcommand)]
        command: SamplesCommand,
    },
    Test(TestArgs),
    Transcribe {
        audio: PathBuf,
        #[arg(long, default_value = "whisper-base")]
        model: String,
    },
    Clone(CloneArgs),
    Convert(ConvertArgs),
    Train(TrainArgs),
}

#[derive(Debug, Subcommand)]
pub(crate) enum DaemonCommand {
    Start,
    Stop,
    Restart,
    Status,
    Logs,
}

#[derive(Debug, Args)]
pub(crate) struct SpeakArgs {
    pub(crate) text: String,
    /// Real installed model to use. Takokit has no release-facing mock default.
    #[arg(long)]
    pub(crate) model: String,
    #[arg(long, default_value = "default")]
    pub(crate) voice: String,
    #[arg(long)]
    pub(crate) language: Option<String>,
    /// Natural-language voice or delivery instruction for compatible models.
    #[arg(long)]
    pub(crate) instruction: Option<String>,
    /// Transcript of the reference sample for cloning models that require it.
    #[arg(long)]
    pub(crate) reference_text: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct RunArgs {
    pub(crate) model: String,
    pub(crate) text: Option<String>,
    #[arg(long)]
    pub(crate) voice: Option<String>,
    #[arg(long)]
    pub(crate) file: Option<PathBuf>,
    #[arg(long)]
    pub(crate) language: Option<String>,
    #[arg(long)]
    pub(crate) instruction: Option<String>,
    #[arg(long)]
    pub(crate) reference_text: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct PullArgs {
    pub(crate) model: String,
    #[arg(long)]
    pub(crate) metadata_only: bool,
    /// Explicitly accept a model license in non-interactive use (for example CPML).
    #[arg(long, value_name = "LICENSE")]
    pub(crate) accept_license: Option<String>,
}

#[derive(Debug, Subcommand)]
pub(crate) enum LicenseCommand {
    /// List durable license acceptance receipts.
    List,
    /// Show receipts for one license identifier.
    Show { license: String },
    /// Revoke acceptance for a license, optionally for one model only.
    Revoke {
        license: String,
        #[arg(long)]
        model: Option<String>,
    },
}

#[derive(Debug, Args)]
pub(crate) struct RmArgs {
    pub(crate) model: String,
    /// Preview model and dependency garbage collection without changing files.
    #[arg(long)]
    pub(crate) dry_run: bool,
}

#[derive(Debug, Args)]
pub(crate) struct DoctorArgs {
    #[arg(long)]
    pub(crate) json: bool,
}

#[derive(Debug, Args)]
pub(crate) struct PlanArgs {
    pub(crate) model: String,
    #[arg(long)]
    pub(crate) json: bool,
}

#[derive(Debug, Args)]
pub(crate) struct StorageArgs {
    /// Emit the storage report as JSON.
    #[arg(long)]
    pub(crate) json: bool,
    #[command(subcommand)]
    pub(crate) command: Option<StorageCommand>,
}

#[derive(Debug, Subcommand)]
pub(crate) enum StorageCommand {
    /// Show the last automatic cleanup result and whether background cleanup is enabled.
    Status,
    /// Remove only reconstructible data in the requested provider-aware scope.
    Clean {
        /// Show what would be removed without changing files.
        #[arg(long)]
        dry_run: bool,
        /// Safe cleanup class. Provider checkpoint caches are never implicit cleanup targets.
        #[arg(long, value_enum, default_value = "all-safe")]
        scope: StorageScope,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum UpdateCommand {
    /// Inspect the configured signed release manifest.
    Check(UpdateSourceArgs),
    /// Stage a verified update and launch the external updater helper.
    Apply(UpdateSourceArgs),
    /// Show local update channel, distribution mode, and update journal state.
    Status,
    /// Persist the release channel used for automatic/manual checks.
    Channel { channel: UpdateChannelArg },
}

#[derive(Debug, Args)]
pub(crate) struct UpdateSourceArgs {
    /// Override the release manifest URL/path. Intended for private/test channels.
    #[arg(long)]
    pub(crate) manifest: Option<String>,
    /// Override the detached signature URL/path.
    #[arg(long)]
    pub(crate) signature: Option<String>,
    /// Allow the repository's deterministic non-production test signing key.
    #[arg(long, hide = true)]
    pub(crate) allow_test: bool,
}

#[derive(Debug, Args)]
pub(crate) struct ResetArgs {
    /// Preview exact global-data cleanup without deleting anything.
    #[arg(long)]
    pub(crate) dry_run: bool,
    /// Remove the Takokit global data root after explicit acknowledgement.
    #[arg(long)]
    pub(crate) all: bool,
    /// Required acknowledgement for destructive global reset.
    #[arg(long, value_name = "RESOLVED_TAKOKIT_HOME")]
    pub(crate) confirm: Option<PathBuf>,
    /// Also remove one explicitly selected project's `.tako` directory.
    #[arg(long, value_name = "WORKSPACE")]
    pub(crate) project_data: Option<PathBuf>,
    /// Separate acknowledgement for project `.tako` deletion.
    #[arg(long, value_name = "RESOLVED_TAKO_DIR")]
    pub(crate) confirm_project_data: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct TestArgs {
    pub(crate) model: Option<String>,
    #[arg(long)]
    pub(crate) suite: Option<String>,
    #[arg(long)]
    pub(crate) json: bool,
    #[arg(long)]
    pub(crate) file: Option<PathBuf>,
    #[arg(long)]
    pub(crate) run: bool,
    #[arg(long)]
    pub(crate) category: Option<String>,
    #[arg(long)]
    pub(crate) include_heavy: bool,
}

#[derive(Debug, Args)]
pub(crate) struct QuickstartArgs {
    #[arg(long)]
    pub(crate) full: bool,
}

#[derive(Debug, Subcommand)]
pub(crate) enum DepsCommand {
    Doctor,
    Bootstrap,
}

#[derive(Debug, Subcommand)]
pub(crate) enum SamplesCommand {
    Create,
}

#[derive(Debug, Args)]
pub(crate) struct CloneArgs {
    pub(crate) sample: PathBuf,
    #[arg(long)]
    pub(crate) name: String,
    #[arg(long, default_value = "xtts-v2")]
    pub(crate) model: String,
    /// Confirm that you own the voice or have explicit permission to clone it.
    #[arg(long)]
    pub(crate) consent: bool,
}

#[derive(Debug, Args)]
pub(crate) struct ConvertArgs {
    pub(crate) source: PathBuf,
    #[arg(long)]
    pub(crate) target_voice: String,
    #[arg(long, default_value = "rvc")]
    pub(crate) model: String,
    #[arg(long, value_enum, default_value = "rmvpe")]
    pub(crate) f0_method: RvcF0MethodArg,
    #[arg(long, default_value_t = 0, value_parser = clap::value_parser!(i32).range(-24..=24))]
    pub(crate) pitch_shift: i32,
    #[arg(long, default_value_t = 0.75, value_parser = parse_unit_interval)]
    pub(crate) index_rate: f32,
    #[arg(long, default_value_t = 0.25, value_parser = parse_unit_interval)]
    pub(crate) rms_mix_rate: f32,
    #[arg(long, default_value_t = 0.33, value_parser = parse_protect)]
    pub(crate) protect: f32,
    #[arg(long, default_value_t = 3, value_parser = clap::value_parser!(u32).range(0..=7))]
    pub(crate) filter_radius: u32,
    /// Confirm ownership or explicit permission for the source and target voices.
    #[arg(long)]
    pub(crate) consent: bool,
}

#[derive(Debug, Args)]
pub(crate) struct TrainArgs {
    pub(crate) samples: PathBuf,
    #[arg(long)]
    pub(crate) name: String,
    #[arg(long, default_value = "gpt-sovits")]
    pub(crate) model: String,
    #[arg(long)]
    pub(crate) epochs: Option<u32>,
    /// Confirm ownership or explicit permission for every training sample.
    #[arg(long)]
    pub(crate) consent: bool,
}

#[derive(Debug, Subcommand)]
pub(crate) enum SessionsCommand {
    List {
        #[arg(short, long)]
        query: Option<String>,
    },
    New {
        #[arg(long)]
        title: Option<String>,
    },
    Show {
        id: Uuid,
    },
    Open {
        id: Uuid,
    },
    Rm {
        id: Uuid,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum ListTarget {
    Models,
    Runners,
    Voices,
}

#[derive(Debug, Subcommand)]
pub(crate) enum LibraryTarget {
    Models,
    Runners,
    Sync,
    Show { model: String },
}

#[derive(Debug, Subcommand)]
pub(crate) enum CustomModelCommand {
    Add { manifest: PathBuf },
    List,
    Show { model: String },
    Rm { model: String },
}

#[derive(Debug, Subcommand)]
pub(crate) enum VoiceCommand {
    List,
    Show {
        model: String,
    },
    Add {
        sample: PathBuf,
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "xtts-v2")]
        model: String,
        #[arg(long)]
        consent: bool,
    },
    /// Advanced RVC voice projects, training, checkpoints, testing and packages.
    Rvc {
        #[command(subcommand)]
        command: RvcVoiceCommand,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum RunnerCommand {
    Pull {
        runner: String,
    },
    Install {
        runner: String,
    },
    Doctor {
        runner: String,
        #[arg(long)]
        json: bool,
    },
    Show {
        runner: String,
    },
    Rm {
        runner: String,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum AdapterCommand {
    List,
    Install {
        adapter: String,
    },
    Doctor {
        adapter: String,
        #[arg(long)]
        json: bool,
    },
}

fn parse_unit_interval(value: &str) -> Result<f32, String> {
    parse_range(value, 0.0, 1.0, "value")
}

fn parse_protect(value: &str) -> Result<f32, String> {
    parse_range(value, 0.0, 0.5, "protect")
}

fn parse_range(value: &str, minimum: f32, maximum: f32, label: &str) -> Result<f32, String> {
    let parsed = value
        .parse::<f32>()
        .map_err(|_| format!("{label} must be a number"))?;
    if parsed.is_finite() && (minimum..=maximum).contains(&parsed) {
        Ok(parsed)
    } else {
        Err(format!("{label} must be between {minimum} and {maximum}"))
    }
}
