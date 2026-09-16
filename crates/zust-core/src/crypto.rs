//! 加密模块：CAS AES-CBC + Wisedu RSA PKCS#1 v1.5
//!
//! 精确对应 Python newjwxt/auth.py 的加密逻辑

use crate::error::{Result, ZustError};
use aes::Aes128;
use block_padding::Pkcs7;
use cbc::cipher::{BlockEncryptMut, KeyIvInit};
use cbc::Encryptor;
use rand::Rng;
use rsa::{BigUint, Pkcs1v15Encrypt, RsaPublicKey};

type Aes128CbcEnc = Encryptor<Aes128>;

/// 随机字符集（与 Python CHARS 一致，不含 0/O/1/l/9）
const CHARS: &[u8] = b"ABCDEFGHJKMNPQRSTWXYZabcdefhijkmnprstwxyz2345678";

/// 生成随机字符串
fn random_chars(len: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| {
            let idx = rng.gen_range(0..CHARS.len());
            CHARS[idx] as char
        })
        .collect()
}

/// CAS AES-CBC 密码加密
///
/// 对应 Python `_cas_encrypt_password(password, salt_key)`
///
/// 流程：
/// 1. 生成 64 字符随机前缀 + 16 字符随机 IV
/// 2. 明文 = 前缀 + 密码（UTF-8）
/// 3. AES-CBC 加密，PKCS7 填充
/// 4. 返回 Base64 密文
pub fn cas_encrypt_password(password: &str, salt_key: &str) -> Result<String> {
    let prefix = random_chars(64);
    let iv_str = random_chars(16);

    let plain = prefix + password;
    let key_bytes = salt_key.as_bytes();
    let iv_bytes = iv_str.as_bytes();

    // AES-128 requires 16-byte key
    if key_bytes.len() != 16 {
        return Err(ZustError::Crypto(format!(
            "CAS salt_key must be 16 bytes, got {}",
            key_bytes.len()
        )));
    }

    // 手动 PKCS7 填充
    let plain_bytes = plain.as_bytes();
    let block_size: usize = 16;
    let pad_len = block_size - (plain_bytes.len() % block_size);
    let mut buf = Vec::with_capacity(plain_bytes.len() + pad_len);
    buf.extend_from_slice(plain_bytes);
    buf.resize(plain_bytes.len() + pad_len, pad_len as u8);

    let ct = Aes128CbcEnc::new(key_bytes.into(), iv_bytes.into())
        .encrypt_padded_mut::<Pkcs7>(&mut buf, plain_bytes.len())
        .map_err(|e| ZustError::Crypto(format!("AES encryption failed: {e}")))?;

    Ok(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        ct,
    ))
}

/// Wisedu RSA PKCS#1 v1.5 密码加密
///
/// 对应 Python `_wisedu_encrypt_password(password, modulus_b64, exponent_b64)`
///
/// modulus 和 exponent 是 Base64 编码的大端序字节串
pub fn wisedu_encrypt_password(
    password: &str,
    modulus_b64: &str,
    exponent_b64: &str,
) -> Result<String> {
    use base64::Engine;

    // 解码 modulus 和 exponent（Base64 → 字节 → BigUint）
    let n_bytes = base64::engine::general_purpose::STANDARD
        .decode(modulus_b64)
        .map_err(|e| ZustError::Crypto(format!("modulus base64 decode: {e}")))?;
    let n = BigUint::from_bytes_be(&n_bytes);

    let e_bytes = base64::engine::general_purpose::STANDARD
        .decode(exponent_b64)
        .map_err(|e| ZustError::Crypto(format!("exponent base64 decode: {e}")))?;
    let e = BigUint::from_bytes_be(&e_bytes);

    let pub_key = RsaPublicKey::new(n, e)
        .map_err(|e| ZustError::Crypto(format!("RSA key construction: {e}")))?;

    let mut rng = rand::thread_rng();
    let cipher = pub_key
        .encrypt(&mut rng, Pkcs1v15Encrypt, password.as_bytes())
        .map_err(|e| ZustError::Crypto(format!("RSA encryption: {e}")))?;

    Ok(base64::engine::general_purpose::STANDARD.encode(&cipher))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_chars_length() {
        let s = random_chars(64);
        assert_eq!(s.len(), 64);
        let s = random_chars(16);
        assert_eq!(s.len(), 16);
    }

    #[test]
    fn test_random_chars_no_ambiguous() {
        let s = random_chars(1000);
        assert!(!s.contains('0'));
        assert!(!s.contains('O'));
        assert!(!s.contains('1'));
        assert!(!s.contains('l'));
        assert!(!s.contains('9'));
    }

    #[test]
    fn test_cas_encrypt_basic() {
        let salt = "1234567890123456"; // exactly 16 bytes
        let result = cas_encrypt_password("test_password", salt);
        assert!(result.is_ok(), "Encryption failed: {result:?}");
        let ct = result.unwrap();
        assert!(!ct.is_empty());
    }

    #[test]
    fn test_cas_encrypt_wrong_key_size() {
        let salt = "short"; // not 16 bytes
        let result = cas_encrypt_password("test", salt);
        assert!(result.is_err());
    }
}
