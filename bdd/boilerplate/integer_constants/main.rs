pub mod core {
    include!("core.rs");
}
pub mod api {
    include!("api.rs");
}

fn main() {
    assert_eq!(core::ZERO32, 0);
    assert_eq!(core::SIGNED_ZERO, 0);
    assert_eq!(core::ALL32, u32::MAX);
    assert_eq!(core::ALL64, u64::MAX);
    assert_eq!(core::ZERO64, 0);
    assert_eq!(core::COMPLEMENT_NEGATIVE, 1);
    assert_eq!(core::MINIMUM, i64::MIN);
    assert_eq!(core::LOGICAL_SHIFT, 1);
    assert_eq!(core::FLAGS, u8::MAX);
    assert_eq!(core::AXES, 2);
    assert_eq!(core::MIXED, 1);
    assert_eq!(core::MIXED64, u64::MAX - 2);
    assert_eq!(core::HIGHBIT, 1_u64 << 63);
    assert_eq!(core::QUOTIENT, 2);
    assert_eq!(core::REMAINDER, 1);
    assert_eq!(core::LOW, 15);
    let slots: core::Slots = [0; 15];
    assert_eq!(slots, [0; 15]);
    assert_eq!(core::policy::LOCAL, 2);
    assert_eq!(core::policy::ROOT, 0);
    assert_eq!(core::PRICE, 1.5);
    assert_eq!(core::TEXT, "~keep!");
    assert_eq!(core::SYMBOL, '~');
    assert!(core::ENABLED);
    assert!(matches!(core::PINNED, core::Channel::sms));
    assert_eq!(api::COPIED, 15);
    assert_eq!(api::ZERO, 0);
    assert_eq!(api::AXES, 2);
    assert_eq!(api::REST, 4_294_967_296);
    assert_eq!(api::WIDE, 18_446_744_069_414_584_320);
    assert!(matches!(api::PINNED, core::Channel::sms));
    println!("PASS: integer constants");
}
