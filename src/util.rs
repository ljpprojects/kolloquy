use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use stack_string::SmallString;

pub trait ArrayB64EncodeSmallString<const N: usize>: Sized {

    fn encode_b64(&self) -> SmallString<{(N * 8 / 6).next_multiple_of(4)}>;
}

impl<const N: usize> ArrayB64EncodeSmallString<N> for [u8; N] {
    fn encode_b64(&self) -> SmallString<{(N * 8 / 6).next_multiple_of(4)}> {
        let mut dest = SmallString::<{(N * 8 / 6).next_multiple_of(4)}>::new();
        BASE64_STANDARD.encode_slice(
            self.as_ref(),
            unsafe {
                dest.as_bytes_mut()
            }
        ).unwrap();

        dest
    }
}

mod test {
    use stack_string::SmallString;
    use crate::util::ArrayB64EncodeSmallString;

    #[test]
    fn compiles() {
        let bytes = [1u8; 18];
        let b64: SmallString<24> = bytes.encode_b64();
    }
}

// Revised phonetic alphabet
// A - Adenosine triphosphate
// B - Balloon
// C - Circle
// D - Damascus
// E - Equestrian
// F - Fuji
// G - GIF
// H - Herb
// I - Ice
// J - Juice
// K - Kolloquy
// L - Laminate (n.)
// M - Mist
// N - kNife
// O - Our
// P - Ping pong
// Q - Quantum
// R - Racquet
// S - tSunami
// T - Tsunami
// U - Unanimous
// V - 5
// W - WNBA
// X - Xylophone
// Y - Yak
// Z - Zed
//
// For example, to helpfully spell out the last name "Montgomery"
// Mist, Our, kNife, Tsunami, GIF, Our, Mist, Equestrian, Racquet, Yak
//
// Or to spell out Ammunition (so we don't confuse La'munition and L'ammunition again)
// Adenosine triphosphate, Mist, Mist, Unanimous, kNife, Ice, Tsunami, Ice, Our, kNife