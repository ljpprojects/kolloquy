use alloc::string::String;

pub fn hex_encode<T: AsRef<[u8]>>(bytes: T) -> String {
    hex::encode(bytes)
}

pub fn hex_encode_to_slice<T: AsRef<[u8]>>(bytes: T, output: &mut [u8]) {
    hex::encode_to_slice(bytes, output).unwrap();
}
