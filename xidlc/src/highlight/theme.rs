#[derive(Clone, Copy, Debug)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct Base16Theme {
    pub base00: Rgb,
    pub base01: Rgb,
    pub base02: Rgb,
    pub base03: Rgb,
    pub base04: Rgb,
    pub base05: Rgb,
    pub base06: Rgb,
    pub base07: Rgb,
    pub base08: Rgb,
    pub base09: Rgb,
    pub base0a: Rgb,
    pub base0b: Rgb,
    pub base0c: Rgb,
    pub base0d: Rgb,
    pub base0e: Rgb,
    pub base0f: Rgb,
}

impl Base16Theme {
    pub fn dracula() -> Self {
        Self {
            base00: parse_hex("282936"),
            base01: parse_hex("3a3c4e"),
            base02: parse_hex("4d4f68"),
            base03: parse_hex("626483"),
            base04: parse_hex("62d6e8"),
            base05: parse_hex("e9e9f4"),
            base06: parse_hex("f1f2f8"),
            base07: parse_hex("f7f7fb"),
            base08: parse_hex("ea51b2"),
            base09: parse_hex("b45bcf"),
            base0a: parse_hex("00f769"),
            base0b: parse_hex("ebff87"),
            base0c: parse_hex("a1efe4"),
            base0d: parse_hex("62d6e8"),
            base0e: parse_hex("b45bcf"),
            base0f: parse_hex("00f769"),
        }
    }
}

fn parse_hex(hex: &str) -> Rgb {
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    Rgb { r, g, b }
}
