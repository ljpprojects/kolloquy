pub mod errcodes {
    pub const NO_VERSION_SPECIFIED: &str = "ENOVER";
    pub const VERSION_MISMATCH: &str     = "EVERMM";
    pub const INVALID_VERSION: &str      = "EINVVR";
    pub const INVALID_DIGEST_SIZE: &str  = "EDGN32";
    pub const INVALID_DIGEST: &str       = "EINVDG";
    pub const INVALID_SALT_SIZE: &str    = "ESLN24";
    pub const INVALID_SALT: &str         = "EINVSL";
}