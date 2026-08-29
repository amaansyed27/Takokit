use clap::{Args, Subcommand, ValueEnum};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum UpdateToggleArg {
    On,
    Off,
}

impl UpdateToggleArg {
    pub(crate) fn enabled(self) -> bool {
        matches!(self, Self::On)
    }
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
    /// Download and verify the current update without installing it.
    Download(UpdateSourceArgs),
    /// Stage a verified update and launch the external updater helper.
    Apply(UpdateSourceArgs),
    /// Show version, channel, automatic-check settings, and update journal state.
    Status,
    /// Persist the release channel used for automatic/manual checks.
    Channel { channel: UpdateChannelArg },
    /// Configure conservative automatic checking and opt-in background download.
    Configure(UpdateConfigureArgs),
    /// Internal background check launched opportunistically by installed Takokit surfaces.
    #[command(hide = true)]
    AutoCheck,
}

#[derive(Debug, Args)]
pub(crate) struct UpdateConfigureArgs {
    /// Enable or disable automatic signed-manifest checks.
    #[arg(long, value_enum)]
    pub(crate) automatic_checks: Option<UpdateToggleArg>,
    /// Enable or disable automatic verified download. Installation always remains manual.
    #[arg(long, value_enum)]
    pub(crate) automatic_download: Option<UpdateToggleArg>,
}

#[derive(Debug, Args, Clone, Default)]
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
