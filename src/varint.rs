use crate::Error;

pub fn write_varint(buf: &mut Vec<u8>, mut n: u64) -> usize {
    let mut len = 0;
    loop {
        let mut b = (n & 0x7F) as u8; // low 7 bits
        n >>= 7;
        if n != 0 {
            b |= 0x80; // continuation
        }
        buf.push(b);
        len += 1;
        if n == 0 {
            break;
        }
    }
    len
}

pub fn read_varint(input: &mut &[u8]) -> Result<u64, Error> {
    let mut n = 0u64;
    let mut shift = 0u32;

    for i in 0..input.len() {
        let b = input[i];
        let chunk = (b & 0x7F) as u64;
        n |= chunk << shift;

        if b & 0x80 == 0 {
            *input = &input[i + 1..]; // advance the slice handle
            return Ok(n);
        }

        shift += 7;
        if shift >= 64 {
            return Err(Error::VarintOverflow);
        }
    }
    Err(Error::VarintUnterminated)
}

pub fn read_tagged_varint(input: &mut &[u8]) -> Result<(u64, bool), Error> {
    let raw = read_varint(input)?;
    Ok((raw >> 1, (raw & 1) != 0))
}

pub fn write_tagged_varint(buf: &mut Vec<u8>, value: u64, tagged: bool) -> usize {
    let tagged = value
        .checked_shl(1)
        .expect("value too large for tagged varint")
        | (tagged as u64);
    write_varint(buf, tagged)
}
