#![no_std]

pub const KOLLOQUY_VERSION: usize = 1;
pub const KOLLOQUY_VERSION_STR: &str = "1";

// Secret binding names
pub const PHASH_MTLS_SECRET_NAME: &str = "PHASH_MTLS";

// Password hashing params (stage 1)
pub const PASS1_SALT_SIZE: usize = 30;
pub const PASS1_DIGEST_SIZE: usize = 32;
pub const PASS1_PEPPER_SECRET_NAME: &str = "PASS1_PEPPER";

// Password hashing params (stage 2)
pub const PASS2_SALT_SIZE: usize = 24;
pub const PASS2_DIGEST_SIZE: usize = 48;
pub const PASS2_MAGIC_DATA: &[u8] = &[1, 0xAA, 0xBB, 0xAB, 0xCF, 0x77, 0x93, 0x1C];