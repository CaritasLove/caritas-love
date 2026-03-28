// auth.rs
// Copyright 2026 Patrick Meade.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use base64::{Engine, engine::general_purpose::STANDARD};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

pub fn verify_password(
    password: &str,
    salt_b64: &str,
    expected_hash_b64: &str,
    iterations: i32,
) -> bool {
    if iterations <= 0 {
        return false;
    }

    let Ok(salt) = STANDARD.decode(salt_b64) else {
        return false;
    };

    let Ok(expected_hash) = STANDARD.decode(expected_hash_b64) else {
        return false;
    };

    let derived = pbkdf2_hmac_sha256(
        password.as_bytes(),
        &salt,
        iterations as u32,
        expected_hash.len(),
    );

    derived.ct_eq(&expected_hash).into()
}

fn pbkdf2_hmac_sha256(password: &[u8], salt: &[u8], iterations: u32, dk_len: usize) -> Vec<u8> {
    let hash_len = 32usize;
    let block_count = dk_len.div_ceil(hash_len);
    let mut derived_key = Vec::with_capacity(block_count * hash_len);

    for block_index in 1..=block_count {
        let mut u = pbkdf2_block(password, salt, block_index as u32);
        let mut t = u;

        for _ in 1..iterations {
            u = hmac_sha256(password, &u);

            for (lhs, rhs) in t.iter_mut().zip(u.iter()) {
                *lhs ^= *rhs;
            }
        }

        derived_key.extend_from_slice(&t);
    }

    derived_key.truncate(dk_len);
    derived_key
}

fn pbkdf2_block(password: &[u8], salt: &[u8], block_index: u32) -> [u8; 32] {
    let mut block_input = Vec::with_capacity(salt.len() + 4);
    block_input.extend_from_slice(salt);
    block_input.extend_from_slice(&block_index.to_be_bytes());
    hmac_sha256(password, &block_input)
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts keys of any size");
    mac.update(data);

    let bytes = mac.finalize().into_bytes();
    let mut output = [0u8; 32];
    output.copy_from_slice(&bytes);
    output
}

#[cfg(test)]
mod tests {
    use super::{pbkdf2_hmac_sha256, verify_password};

    const TEST_SALT_B64: &str = "zfRi1EPSWttRorSZSZSg3Q==";
    const TEST_HASH_B64: &str = "12iq4Rb4h6/EmLqFZPrDImY1045zJDZ6GZ7ugknYrU4=";
    const TEST_ITERATIONS: i32 = 60_000;

    #[test]
    fn pbkdf2_matches_known_vector() {
        let derived = pbkdf2_hmac_sha256(b"password", b"salt", 1, 32);

        assert_eq!(
            hex_string(&derived),
            "120fb6cffcf8b32c43e7225256c4f837a86548c92ccc35480805987cb70be17b"
        );
    }

    #[test]
    fn verifies_test_password_fixture() {
        assert!(verify_password(
            "admin",
            TEST_SALT_B64,
            TEST_HASH_B64,
            TEST_ITERATIONS
        ));
    }

    #[test]
    fn rejects_wrong_password() {
        assert!(!verify_password(
            "wrong-password",
            TEST_SALT_B64,
            TEST_HASH_B64,
            TEST_ITERATIONS,
        ));
    }

    fn hex_string(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}
