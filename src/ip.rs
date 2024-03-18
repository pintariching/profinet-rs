pub struct Ipv4Address {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
}

impl Ipv4Address {
    pub fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self { a, b, c, d }
    }
}
