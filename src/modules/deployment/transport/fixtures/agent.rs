//! Answer identity and signature requests using an isolated SSH agent socket.

use ed25519_dalek::{Signer, SigningKey};
use std::{
    io::{Read, Write},
    os::unix::net::UnixListener,
    path::Path,
    thread,
};

fn string(bytes: &[u8]) -> Vec<u8> {
    let mut result = (bytes.len() as u32).to_be_bytes().to_vec();
    result.extend_from_slice(bytes);
    result
}
fn take(bytes: &mut &[u8]) -> Vec<u8> {
    let size = u32::from_be_bytes(bytes[..4].try_into().unwrap()) as usize;
    let value = bytes[4..4 + size].to_vec();
    *bytes = &bytes[4 + size..];
    value
}

pub(crate) fn serve(path: &Path) -> thread::JoinHandle<()> {
    let listener = UnixListener::bind(path).unwrap();
    thread::spawn(move || {
        let key = SigningKey::from_bytes(&[42; 32]);
        let mut identity = string(b"ssh-ed25519");
        identity.extend(string(key.verifying_key().as_bytes()));
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        loop {
            let mut size = [0; 4];
            if socket.read_exact(&mut size).is_err() {
                break;
            }
            let size = u32::from_be_bytes(size) as usize;
            assert!(size < 65536);
            let mut packet = vec![0; size];
            socket.read_exact(&mut packet).unwrap();
            let mut response = Vec::new();
            match packet[0] {
                11 => {
                    response.push(12);
                    response.extend(1_u32.to_be_bytes());
                    response.extend(string(&identity));
                    response.extend(string(b"fixture"));
                }
                13 => {
                    let mut payload = &packet[1..];
                    assert_eq!(take(&mut payload), identity);
                    let data = take(&mut payload);
                    let signature = key.sign(&data);
                    let mut encoded = string(b"ssh-ed25519");
                    encoded.extend(string(&signature.to_bytes()));
                    response.push(14);
                    response.extend(string(&encoded));
                }
                _ => response.push(5),
            }
            socket.write_all(&string(&response)).unwrap();
        }
    })
}
