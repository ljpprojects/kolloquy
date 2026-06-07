use std::{
    env,
    fs,
    io::stdin,
    process::exit,
};
use std::io::Read;

fn main() {
    if env::args().len() < 3 {
        eprintln!("Usage: corruptor <file> <n> <output file>");
        exit(1);
    }

    let path = env::args().nth(1).unwrap();
    let n: usize = env::args().nth(2).unwrap().parse().unwrap();
    let out_path = env::args().nth(3).unwrap();
    let run = env::args().nth(4).map(|a| a.parse::<usize>().unwrap()).unwrap_or(1);

    eprintln!("Corrupting {path} to {out_path}");

    // Read file
    let mut bytes = fs::read(path.clone()).unwrap();

    let mut rbytes = vec![0u8; bytes.len()];
    stdin().read_exact(&mut rbytes).unwrap();

    eprintln!("Read {path}, size = {}", bytes.len());

    // Replace every nth byte from file with a byte from stdin and write to buffer
    let mut i = 0;
    while i < bytes.len() {
        if i % n == 0 && i != 0 {
            let end = (i + run).min(bytes.len() - 1);

            if end - i < 1 {
                break;
            }

            bytes[i..end].copy_from_slice(&rbytes[i..end]);

            eprintln!("Bytes {i} to {} of {} modified", end, bytes.len());

            i += end - i;
        } else {
            i += 1;
        }
    }

    eprintln!("Writing corrupted file to {out_path}");

    fs::write(out_path, bytes).unwrap();

    eprintln!("Done");

    exit(0);
}