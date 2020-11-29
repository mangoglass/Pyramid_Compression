pub fn u8_to_string(val: u8) -> String {
    if val < 0x80 {
        (val as char).to_string()
    } else {
        format!("{}", val)
    }
}
