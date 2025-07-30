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
///
/// Zero-preserving rule (Bitcoin/Base58-style):
/// - Each leading 0x00 byte in `input` becomes a leading '0' digit.
/// - Empty input -> ""
/// - All-zero input of length N -> "0" repeated N times.
pub fn base62_encode(input: &[u8]) -> String {
    const ALPHABET: &[u8; 62] =
        b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

    if input.is_empty() {
        return String::new();
    }

    // Count leading zero bytes to preserve them as leading '0' digits.
    let lz = input.iter().take_while(|&&b| b == 0).count();
    let data = &input[lz..];

    // If the entire input was zeros, return exactly that many '0' digits.
    if data.is_empty() {
        return "0".repeat(lz);
    }

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
            v.push(acc);
        }
        v
    };

    // Upper bound on output length for the non-zero tail.
    let mut out = Vec::with_capacity((data.len() as f64 * 1.35).ceil() as usize);

    // Long division by 62 collecting remainders.
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
        while head < limbs.len() && limbs[head] == 0 {
            head += 1;
        }
    }
    out.reverse();

    // Prefix exactly `lz` '0' digits.
    let mut s = String::with_capacity(lz + out.len());
    for _ in 0..lz {
        s.push('0');
    }
    // SAFETY: ALPHABET is ASCII.
    s.push_str(std::str::from_utf8(&out).unwrap());
    s
}

/// Decodes a Base62 string (alphabet 0-9A-Za-z) into bytes (big-endian).
///
/// Zero-preserving rule (Bitcoin/Base58-style):
/// - Each leading '0' digit becomes a leading 0x00 byte.
/// - "" -> Ok(vec![])
/// - "000" -> Ok(vec![0, 0, 0])
pub fn base62_decode(s: &str) -> Result<Vec<u8>, DecodeError> {
    if s.is_empty() {
        return Ok(Vec::new());
    }

    // Count leading '0' digits to restore them as 0x00 bytes.
    let bytes = s.as_bytes();
    let mut idx = 0usize;
    while idx < bytes.len() && bytes[idx] == b'0' {
        idx += 1;
    }
    let lz = idx;
    let digits = &bytes[idx..];

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

    // If there are only leading zeros (no other digits), return that many zero bytes.
    if digits.is_empty() {
        return Ok(vec![0; lz]);
    }

    // Decode the non-zero tail into little-endian base-256 limbs.
    let mut out: Vec<u8> = vec![0];
    for (i, &b) in digits.iter().enumerate() {
        let d = val(b).ok_or_else(|| DecodeError::InvalidChar {
            ch: b as char,
            index: lz + i, // report index relative to the original string
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
        let mut add_carry: u32 = d;
        for limb in &mut out {
            let acc = (*limb as u32) + add_carry;
            *limb = (acc & 0xFF) as u8;
            add_carry = acc >> 8;
            if add_carry == 0 {
                break;
            }
        }
        while add_carry > 0 {
            out.push((add_carry & 0xFF) as u8);
            add_carry >>= 8;
        }
    }

    // Normalize the numeric part (remove redundant high-order zeros).
    while out.len() > 1 && *out.last().unwrap() == 0 {
        out.pop();
    }
    out.reverse(); // little-endian -> big-endian

    // Prepend exactly `lz` zero bytes.
    let mut res = Vec::with_capacity(lz + out.len());
    res.extend(std::iter::repeat(0).take(lz));
    res.extend(out);
    Ok(res)
}


#[cfg(test)]
mod tests {
    use super::{base62_encode, base62_decode, DecodeError};

    #[test]
    fn basics() {
        assert_eq!(base62_encode(b""), "");
        assert_eq!(base62_encode(&[0]), "0");
        assert_eq!(base62_encode(&[0,0]), "00");
        assert_eq!(base62_encode(&[1]), "1");
        assert_eq!(base62_encode(&[255]), "47"); // 255 = 4*62 + 7
        assert_eq!(base62_encode(b"hello"), "7tQLFHz"); // example
        assert_eq!(base62_encode(b"\x00hello"), "07tQLFHz"); // leading 0s don’t add extra digits
    }

    #[test]
    fn roundtrip_basic() {
        let samples: &[&[u8]] = &[
            b"",
            &[0, 0],
            &[1],
            &[255],
            b"hello",
            b"\x00hello", // leading zero not preserved by design
            b"\x01\x00\x00\x01",
        ];

        for &s in samples {
            let enc = crate::base62_encode(s);
            let dec = base62_decode(&enc).unwrap();
            assert_eq!(dec, s, "enc={} s={:?}", enc, s);
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

    #[test]
    fn base62_preserves_leading_zeros() {
        let input = vec![0x00, 0x00, 0x01, 0x02, 0x03];
        let encoded = base62_encode(&input);
        let decoded = base62_decode(&encoded).unwrap();

        assert_eq!(decoded, input, "Base62 must preserve leading zeros");
        eprintln!("input  = {:02X?}", input);
        eprintln!("encoded= {}", encoded);
        eprintln!("decoded= {:02X?}", decoded);
    }
}
