use xidlc_examples::hello_world::*;

#[test]
fn test_basic() {
    assert_eq!(EnumFirstDefault::default(), EnumFirstDefault::First);
    assert_eq!(EnumSecondDefault::default(), EnumSecondDefault::Second);

    let _ = SimpleStruct::default();
    let c = ComplexStruct::default();
    assert_eq!(c.c, EnumFirstDefault::First);
    assert_eq!(c.d, EnumSecondDefault::Second);
}
