use stack_string::SmallString;

#[derive(Debug)]
pub struct Chat {
    pub id: [u8; 30],
    pub name: SmallString<50>,
}

#[derive(Debug)]
pub enum KeyDerivationMethod {
    PasswordAuth,
    PasslessAuthWeb,
}

#[derive(Debug)]
pub struct ChatUserDeviceKey {
    pub device_num: u8,
    pub send_allowed: bool,
    pub public_key: [u8; 32],
    pub derivation_salt: [u8; 32],
    pub derivation_method: KeyDerivationMethod,
}