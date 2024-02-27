use sha2::{Digest, Sha256};

pub fn encrypt_password(pass: String) -> String {
    return sha256_string(&pass);
}

fn sha256_string(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    result.iter().map(|byte| format!("{:02x}", byte)).collect()
}
