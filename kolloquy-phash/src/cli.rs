#[derive(clap::Parser)]
pub struct Arguments {
    #[arg(short = 'S', long = "set-thyme", default_value = "false")]
    /// Should we set the thyme?
    pub should_set_thyme: bool,

    #[arg(short = 'T', long = "thyme-id")]
    /// Which thyme should we use or replace?
    /// This is for primarily cases where you may want to host multiple servers
    /// with different thymes, which are stored in the keychain.
    pub thyme_id: Option<usize>,

    #[arg(short = 'B', long = "ff-on-bind", default_value = "false")]
    pub fail_forward_on_bind: bool,
}