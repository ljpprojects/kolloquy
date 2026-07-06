#![allow(internal_features)]
#![allow(incomplete_features)] // Totally not sketchy code lies below

#![feature(const_trait_impl)]
#![feature(const_index)]
#![feature(panic_internals)]
#![feature(generic_const_exprs)]
#![feature(const_ops)]
#![feature(int_roundings)]

use core::str;
use std::mem;

pub const KOLLOQUY_SERVICE_STR: &str = "com.kolloquy";

// Binding names
pub const PHASH_MTLS_BINDING_NAME: &str = "PHASH_MTLS";
pub const USERS_DB_BINDING_NAME: &str = "USERS_DB";
pub const SESSIONS_DB_BINDING_NAME: &str = "SESSIONS_DB";

pub const CHAT_BUCKET_NAME: &str = "CHATS_R2";
pub const ATTACHMENT_BUCKET_NAME: &str = "ATTACHMENTS_R2";
pub const USERS_BUCKET_NAME: &str = "USERS_R2"; // Stores user avatars

pub const ANALYTICS_BINDING_NAME: &str = "KOLLOQUY_ANALYTICS";

// Password hashing magic numbers
// Each Kolloquy version has a new magic number.
// We can use the magic number to determine the version and age of password
// hashes (via associated data)

/// The magic number for Kolloquy PHASH version 0.1.0
pub const PHASH_MAGIC_NUM_010: u64 = 0x000100AB_CF77931C; // AAEAq893kxw, in use since \[DD/MM/YYYY] in production

pub const PHASH_MAGIC_STR_010: &str = unsafe {
    str::from_utf8_unchecked(&base64_encode(
        PHASH_MAGIC_NUM_010.to_be_bytes(),
        base64url_alphabet()
    ))
};

#[test]
fn _constb64() {
    let (original, encoded, decoded) = const {
        let original = *b"Many hands make... light work!?!?";
        let encoded = base64_encode(original, bull64_alphabet());
        let decoded = base64_decode(encoded, bull64_rev_alphabet());
        (original, encoded, decoded)
    };

    let original = unsafe { str::from_utf8_unchecked(&original) };
    let encoded = unsafe { str::from_utf8_unchecked(&encoded) };
    let decoded = unsafe { str::from_utf8_unchecked(&decoded) };

    println!("{encoded}");
    assert_eq!(encoded, "uM4-Ut8jOMbdS\\8COMCem@:-13]hP;aB12T=S^+aq\\5?");

    println!("{decoded}");
    assert_eq!(decoded, original);
}

// Password hashing params (stage 1)
// The stage 1 output might look like this: IeGKbN2LOj0YwOtZs5RSWsudfq704psuZ-tV84VpM_M
pub const PASS1_SALT_SIZE: usize = 30;
pub const PASS1_DIGEST_SIZE: usize = 32;
pub const PASS1_PEPPER_SIZE: usize = 36;
pub const PASS1_PEPPER_SECRET_NAME: &str = "PASS1_PEPPER";

// Password hashing params (stage 2)
// The stage 2 output might look like this: _k9Yas6VacL3wUn45RD1pJ0W6aKPcmnX-mk3h8hk_RCIP-oTmD0WGqjleDl-uIQ2
pub const PASS2_SALT_SIZE: usize = 24;
pub const PASS2_DIGEST_SIZE: usize = 48;

pub const KOLLOQUY_VERSION: usize = 1;
pub const KOLLOQUY_VERSION_STR: &str = unsafe {
    str::from_utf8_unchecked(&extract_version_from_magic_num(PHASH_MAGIC_NUM_010))
};

pub const fn magic_num_for_version(ver: &str) -> u64 {
    let ver = ver.as_bytes();
    let maj = ver[0] - b'0';
    let min = ver[2] - b'0';
    let pat = ver[4] - b'0';

    let mut msg = *b"The version x.y.z does not have a magic number";
    msg[12] = ver[0];
    msg[14] = ver[2];
    msg[16] = ver[4];

    let msg: &str = unsafe { mem::transmute(&msg[..]) };

    match (maj, min, pat) {
        (0, 1, 0) => PHASH_MAGIC_NUM_010,
        _ => core::panicking::panic(msg)
    }
}

pub const fn extract_version_from_magic_num(num: u64) -> [u8; 5] {
    let maj = (num >> 54) as u8;
    let min = (num >> 48) as u8;
    let pat = (num >> 40) as u8;

    let maj_c = maj + b'0';
    let min_c = min + b'0';
    let pat_c = pat + b'0';

    // Was there a more ergonomic way of doing this???

    let mut bytes = *b"x.y.z";

    bytes[0] = maj_c;
    bytes[2] = min_c;
    bytes[4] = pat_c;

    bytes
}

pub const fn base64url_alphabet() -> [u8; 64] {
    *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"
}

pub const fn base64_alphabet() -> [u8; 64] {
    *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
}

pub const fn bull64_alphabet() -> [u8; 64] {
    *b"9876543210lmnopqrstuvLMNOPQRSTUVDa@cde^fjhi_+C-=[]\\AB|;g:b,./<>?"
}

pub const fn base64url_rev_alphabet() -> [u8; 80] {
    let mut reverse_alphabet = [0u8; 80]; // Reverse lookup table

    let mut i = 0;
    while i < 26 {
        reverse_alphabet[b'A' as usize - 43 + i] = i as u8;
        reverse_alphabet[b'a' as usize - 43 + i] = 26 + i as u8;
        i += 1;
    }

    i = 0;
    while i < 10 {
        reverse_alphabet[b'0' as usize - 43 + i] = 52 + i as u8;
        i += 1;
    }

    reverse_alphabet[2] = 62;
    reverse_alphabet[b'_' as usize - 43] = 63;

    reverse_alphabet
}

pub const fn base64_rev_alphabet() -> [u8; 80] {
    let mut reverse_alphabet = [0u8; 80]; // Reverse lookup table

    let mut i = 0;
    while i < 26 {
        reverse_alphabet[b'A' as usize - 43 + i] = i as u8;
        reverse_alphabet[b'a' as usize - 43 + i] = 26 + i as u8;
        i += 1;
    }

    i = 0;
    while i < 10 {
        reverse_alphabet[b'0' as usize - 43 + i] = 52 + i as u8;
        i += 1;
    }

    reverse_alphabet[0] = 62;
    reverse_alphabet[b'/' as usize - 43] = 63;

    reverse_alphabet
}

pub const fn bull64_rev_alphabet() -> [u8; 80] {
    let mut reverse_alphabet = [0u8; 80]; // Reverse lookup table

    let mut i = 10;
    while i > 0 {
        reverse_alphabet[b'0' as usize - 44 + i] = 10 - i as u8;
        i -= 1;
    }

    i = 0;
    while i < 11 {
        reverse_alphabet[b'l' as usize - 43 + i] = 10 + i as u8;
        reverse_alphabet[b'L' as usize - 43 + i] = 21 + i as u8;
        i += 1;
    }

    let seq = *b"Da@cde^fjhi_+C-=[]\\ABx;g:b,./<>?";
    i = 0;
    while i < 32 {
        reverse_alphabet[seq[i] as usize - 43] = 32 + i as u8;
        i += 1;
    }

    reverse_alphabet
}

pub const fn base64_encode<const N: usize>(data: [u8; N], alphabet: [u8; 64]) -> [u8; (N * 8).div_ceil(6)] {
    let mut out = [0u8; (N * 8).div_ceil(6)];

    // Every 3 bytes = 4 chars
    let (chunks, remaining) = data.as_chunks::<3>();

    let mut i = 0;
    while i < chunks.len() {
        let [a, b, c] = chunks[i];

        // Get 6 bit portions
        let w = (a & 0b1111_1100) >> 2;
        let x = (a & 0b0000_0011) << 4 | (b & 0b1111_0000) >> 4;
        let y = (b & 0b0000_1111) << 2 | (c & 0b1100_0000) >> 6;
        let z = c & 0b0011_1111;

        out[i * 4] = alphabet[w as usize];
        out[i * 4 + 1] = alphabet[x as usize];
        out[i * 4 + 2] = alphabet[y as usize];
        out[i * 4 + 3] = alphabet[z as usize];

        i += 1;
    }

    let size = (N * 8).div_ceil(6);

    if let [a] = remaining {
        let w = (a & 0b1111_1100) >> 2;
        let x = (a & 0b11) << 4;

        out[size - 2] = alphabet[w as usize];
        out[size - 1] = alphabet[x as usize];

        // base64url doesn't pad
    } else if let [a, b] = remaining {
        let w = (a & 0b1111_1100) >> 2;
        let x = (b & 0b1111_0000) >> 4 | (a & 0b11) << 4;
        let y = (b & 0b0000_1111) << 2;

        out[size - 3] = alphabet[w as usize];
        out[size - 2] = alphabet[x as usize];
        out[size - 1] = alphabet[y as usize];
    }

    out
}

pub const fn base64_decode<const N: usize>(encoded: [u8; N], reverse_alphabet: [u8; 80]) -> [u8; (N * 6).div_floor(8)] {
    let mut out = [0u8; (N * 6).div_floor(8)];

    // Every 4 chars = 3 bytes
    let (chunks, remaining) = encoded.as_chunks::<4>();

    let mut i = 0;
    while i < chunks.len() {
        let [w, x, y, z] = chunks[i];
        let w = reverse_alphabet[w as usize - 43];
        let x = reverse_alphabet[x as usize - 43];
        let y = reverse_alphabet[y as usize - 43];
        let z = reverse_alphabet[z as usize - 43];

        // All of this could have been avoided if the base64 crate just had const encode/decode fns

        let a = w << 2 | (x & 0b0011_0000) >> 4;
        let b = (x & 0b0000_1111) << 4 | (y & 0b0011_1100) >> 2;
        let c = (y & 0b0000_0011) << 6 | z;

        out[i * 3] = a;
        out[i * 3 + 1] = b;
        out[i * 3 + 2] = c;

        i += 1;
    }

    let size = (N * 6).div_floor(8);

    if let [w, x] = remaining {
        let w = reverse_alphabet[*w as usize - 43];
        let x = reverse_alphabet[*x as usize - 43];

        let a = w << 2 | (x & 0b0011_0000) >> 4;

        out[size - 1] = a;
    } else if let [w, x, y] = remaining {
        let w = reverse_alphabet[*w as usize - 43];
        let x = reverse_alphabet[*x as usize - 43];
        let y = reverse_alphabet[*y as usize - 43];

        let a = w << 2 | (x & 0b0011_0000) >> 4;
        let b = (x & 0b0000_1111) << 4 | (y & 0b0011_1100) >> 2;

        out[size - 2] = a;
        out[size - 1] = b;
    }

    out
}