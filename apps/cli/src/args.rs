//! CLI argument and subcommand definitions.

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Parser)]
#[command(name = "takokit", version, about = "Local voice AI runtime")]
pub(crate) struct Cli {
    #[arg(long, global = true)]
    pub(crate) direct: bool,
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
    Capabilities,
    Models,
    Runners,
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
    Rm {
        model: String,
    },
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
    #[arg(long, default_value = "mock-tts")]
    pub(crate) model: String,
    #[arg(long, default_value = "default")]
    pub(crate) voice: String,
}

#[derive(Debug, Args)]
pub(crate) struct RunArgs {
    pub(crate) model: String,
    pub(crate) text: Option<String>,
    #[arg(long)]
    pub(crate) voice: Option<String>,
    #[arg(long)]
    pub(crate) file: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct PullArgs {
    pub(crate) model: String,
    #[arg(long)]
    pub(crate) metadata_only: bool,
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
}

#[derive(Debug, Args)]
pub(crate) struct TrainArgs {
    pub(crate) samples: PathBuf,
    #[arg(long)]
    pub(crate) name: String,
    #[arg(long, default_value = "gpt-sovits")]
    pub(crate) model: String,
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
