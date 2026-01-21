pub trait XidlTypeObject {
    fn minimal_type_object() -> crate::DDS::XTypes::TypeObject;
    fn complete_type_object() -> crate::DDS::XTypes::TypeObject;
}

impl<T> XidlTypeObject for Vec<T>
where
    T: XidlTypeObject,
{
    fn minimal_type_object() -> crate::DDS::XTypes::TypeObject {
        todo!()
    }

    fn complete_type_object() -> crate::DDS::XTypes::TypeObject {
        todo!()
    }
}

macro_rules! impl_mock_for {
    ($($ty:ty)*) => {
        $(

            impl XidlTypeObject for $ty {
                fn minimal_type_object() -> crate::DDS::XTypes::TypeObject {
                    todo!()
                }

                fn complete_type_object() -> crate::DDS::XTypes::TypeObject {
                    todo!()
                }
            }
        )*
    };
}

impl_mock_for!(u16 u8 [u8; 4] u32 String i32 [u8; 14]);
