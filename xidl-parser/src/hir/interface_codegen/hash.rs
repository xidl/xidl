use super::ScopedName;

pub fn exception_hash_const_name(exception: &ScopedName) -> String {
    format!("{}_Ex_Hash", exception_unqualified(exception))
}

pub fn op_hash_const_name(interface: &str, op_name: &str) -> String {
    format!("{interface}_{op_name}_Hash")
}

pub fn hash_string(value: &str) -> u32 {
    let digest = md5::compute(value.as_bytes());
    let bytes = digest.0;
    (bytes[0] as u32)
        + ((bytes[1] as u32) << 8)
        + ((bytes[2] as u32) << 16)
        + ((bytes[3] as u32) << 24)
}

fn exception_unqualified(exception: &ScopedName) -> String {
    exception
        .name
        .last()
        .cloned()
        .unwrap_or_else(|| "Exception".to_string())
}
