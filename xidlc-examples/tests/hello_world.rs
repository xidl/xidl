use xidlc_examples::hello_world::*;

#[test]
fn test_basic() {
    assert_eq!(EnumFirstDefault::default(), EnumFirstDefault::First);
    assert_eq!(EnumSecondDefault::default(), EnumSecondDefault::Second);
}
