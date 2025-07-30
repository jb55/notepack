#[derive(Debug, Clone)]
pub enum StringType<'a> {
    Bytes(&'a [u8]),
    Str(&'a str),
}
