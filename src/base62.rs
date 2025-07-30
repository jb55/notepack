/// Error type for Base62 decoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    InvalidChar { ch: char, index: usize },
}

impl core::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DecodeError::InvalidChar { ch, index } => {
                write!(f, "invalid Base62 character '{}' at index {}", ch, index)
            }
        }
    }
}

impl std::error::Error for DecodeError {}


/// Base62-encodes arbitrary bytes using the alphabet 0-9A-Za-z.
/// Interprets `input` as a big-endian integer.
/// - `b""` -> `""`
/// - any all-zero input -> `"0"`
pub fn base62_encode(input: &[u8]) -> String {
    const ALPHABET: &[u8; 62] =
        b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

    // Empty -> empty (preserves your original behavior)
    if input.is_empty() {
        return String::new();
    }

    // Skip leading zero bytes once; if all zeros -> "0"
    let mut first_nz = 0usize;
    while first_nz < input.len() && input[first_nz] == 0 {
        first_nz += 1;
    }
    if first_nz == input.len() {
        return "0".to_string();
    }
    let data = &input[first_nz..];

    // Pack into big-endian u64 limbs: limbs[0] is the most-significant limb.
    let mut limbs: Vec<u64> = {
        let mut v = Vec::with_capacity((data.len() + 7) / 8);
        let mut acc: u64 = 0;
        let mut cnt: usize = 0;
        for &b in data {
            acc = (acc << 8) | (b as u64);
            cnt += 1;
            if cnt == 8 {
                v.push(acc);
                acc = 0;
                cnt = 0;
            }
        }
        if cnt != 0 {
            // Partial least-significant limb (fewer than 8 bytes) is fine as-is.
            v.push(acc);
        }
        v
    };

    // Approximate upper bound on output length: ceil(n_bytes * 8 / log2(62)).
    // 8 / log2(62) ≈ 1.3427. We'll just reserve 1.35× to avoid re-allocs.
    let mut out = Vec::with_capacity((data.len() as f64 * 1.35).ceil() as usize);

    // Perform division by 62 on 64-bit limbs, collecting remainders.
    // We slide a "head" index instead of removing from the front.
    let mut head = 0usize;
    while head < limbs.len() {
        let mut carry: u128 = 0;
        for j in head..limbs.len() {
            let cur = (carry << 64) | (limbs[j] as u128);
            let q = (cur / 62) as u64;
            carry = cur % 62;
            limbs[j] = q;
        }
        out.push(ALPHABET[carry as usize]);

        // Skip newly-zero leading limbs in O(1) amortized time.
        while head < limbs.len() && limbs[head] == 0 {
            head += 1;
        }
    }

    out.reverse();
    // ALPHABET is ASCII; if you prefer to avoid the UTF-8 check:
    // unsafe { String::from_utf8_unchecked(out) }
    String::from_utf8(out).unwrap()
}


/// Decodes a Base62 string (alphabet 0-9A-Za-z) into bytes.
/// - `""` -> `Ok(vec![])`
/// - `"0"` -> `Ok(vec![0])`
///
/// Note: This variant does **not** preserve leading zero bytes that may have
/// been present before encoding. For example, both `b"\x00"` and `b"\x00\x00"`
/// encode to `"0"`, which decodes back to `vec![0]`.
pub fn base62_decode(s: &str) -> Result<Vec<u8>, DecodeError> {
    // Empty string -> empty bytes (mirrors encode(b"") -> "")
    if s.is_empty() {
        return Ok(Vec::new());
    }

    // Work on little-endian base-256 limbs for easy carry handling.
    let mut out: Vec<u8> = vec![0];

    // A small helper: map ASCII byte to Base62 value for 0-9A-Za-z.
    #[inline]
    fn val(b: u8) -> Option<u32> {
        match b {
            b'0'..=b'9' => Some((b - b'0') as u32),              // 0..=9
            b'A'..=b'Z' => Some((b - b'A') as u32 + 10),         // 10..=35
            b'a'..=b'z' => Some((b - b'a') as u32 + 36),         // 36..=61
            _ => None,
        }
    }

    for (i, &b) in s.as_bytes().iter().enumerate() {
        let d = val(b).ok_or_else(|| DecodeError::InvalidChar {
            ch: b as char,
            index: i,
        })?;

        // out = out * 62
        let mut carry: u32 = 0;
        for limb in &mut out {
            let acc = (*limb as u32) * 62 + carry;
            *limb = (acc & 0xFF) as u8;
            carry = acc >> 8;
        }
        while carry > 0 {
            out.push((carry & 0xFF) as u8);
            carry >>= 8;
        }

        // out = out + d
        let mut carry_add: u32 = d;
        for limb in &mut out {
            let acc = (*limb as u32) + carry_add;
            *limb = (acc & 0xFF) as u8;
            carry_add = acc >> 8;
            if carry_add == 0 {
                break;
            }
        }
        while carry_add > 0 {
            out.push((carry_add & 0xFF) as u8);
            carry_add >>= 8;
        }
    }

    // Normalize (remove redundant high-order zeros), but keep a single zero if value is zero.
    while out.len() > 1 && *out.last().unwrap() == 0 {
        out.pop();
    }

    out.reverse(); // convert little-endian to big-endian
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{base62_encode, base62_decode, DecodeError};

    #[test]
    fn basics() {
        assert_eq!(base62_encode(b""), "");
        assert_eq!(base62_encode(&[0]), "0");
        assert_eq!(base62_encode(&[1]), "1");
        assert_eq!(base62_encode(&[255]), "47"); // 255 = 4*62 + 7
        assert_eq!(base62_encode(b"hello"), "7tQLFHz"); // example
        assert_eq!(base62_encode(b"\x00hello"), "7tQLFHz"); // leading 0s don’t add extra digits
    }

    #[test]
    fn roundtrip_basic() {
        let samples: &[&[u8]] = &[
            b"",
            &[0],
            &[1],
            &[255],
            b"hello",
            b"\x00hello", // leading zero not preserved by design
            b"\x01\x00\x00\x01",
        ];

        for &s in samples {
            let enc = crate::base62_encode(s);
            let dec = base62_decode(&enc).unwrap();
            // Because the encoder is canonical (no preserved leading zeros),
            // this roundtrip compares the *numeric value*, not exact prefix zeros.
            // For empty input, empty output.
            if s.is_empty() {
                assert_eq!(dec, b"");
            } else {
                // Strip leading zeros from original, to match decode semantics.
                let i = s.iter().position(|&b| b != 0).unwrap_or(s.len() - 1);
                let canonical = if s.iter().all(|&b| b == 0) {
                    vec![0]
                } else {
                    s[i..].to_vec()
                };
                assert_eq!(dec, canonical, "enc={} s={:?}", enc, s);
            }
        }
    }

    #[test]
    fn rejects_bad_chars() {
        let err = base62_decode("7tQLFHz!").unwrap_err();
        match err {
            DecodeError::InvalidChar { index, .. } => assert_eq!(index, 7),
        }
    }

    #[test]
    fn specific_values() {
        assert_eq!(base62_decode("").unwrap(), b"");
        assert_eq!(base62_decode("0").unwrap(), vec![0]);
        assert_eq!(base62_decode("1").unwrap(), vec![1]);
        assert_eq!(base62_decode("47").unwrap(), vec![255]); // from earlier example
        assert_eq!(base62_decode("7tQLFHz").unwrap(), b"hello"); // matches encoder example
    }
}
