#[cfg(test)]
pub fn print_hexdump(buf: &[u8]) -> String {
    let mut string = String::new();

    buf.chunks(16).enumerate().for_each(|(i, chunk)| {
        let line_number = i * 10;

        string.push_str(&format!("{:0>4} ", line_number));

        chunk
            .iter()
            .for_each(|c| string.push_str(&format!("{:0>2x} ", c)));

        string.push_str("\n");
    });

    string
}
