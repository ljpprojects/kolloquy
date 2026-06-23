#[derive(clap::Parser)]
pub struct Arguments {
    /// Overwrite the thyme indicated by --thyme-id. Persists indefinitely on macOS and until reboot on Linux.
    #[arg(short = 'S', long = "set-thyme", default_value = "false")]
    pub should_set_thyme: bool,

    /// ID of the thyme to use for this server
    #[arg(short = 'T', long = "thyme-id")]
    pub thyme_id: Option<usize>,

    /// Do not exit if an address fails to bind
    #[arg(short = 'B', long = "ff-on-bind", default_value = "false")]
    pub fail_forward_on_bind: bool,
}
