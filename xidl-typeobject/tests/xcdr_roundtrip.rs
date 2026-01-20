use xidl_typeobject::DDS::XTypes::{EquivalenceHash, MemberFlag, TypeObjectHashId, EK_COMPLETE};
use xidl_xcdr::{XcdrDeserialize, XcdrSerialize};

fn serialize_to_vec<T: XcdrSerialize>(value: &T) -> Vec<u8> {
    let mut buf = vec![0u8; 1024];
    let used = value.serialize(&mut buf).expect("serialize");
    buf.truncate(used);
    buf
}

fn deserialize_from_slice<T: XcdrDeserialize>(buf: &[u8]) -> T {
    let mut deserializer = xidl_xcdr::cdr::CdrDeserializer::new(buf);
    T::deserialize(&mut deserializer).expect("deserialize")
}

#[test]
fn member_flag_roundtrip() {
    let value = MemberFlag {
        value: MemberFlag::IS_KEY | MemberFlag::IS_OPTIONAL,
    };
    let buf = serialize_to_vec(&value);
    let out: MemberFlag = deserialize_from_slice(&buf);
    assert_eq!(value.value, out.value);
}

#[test]
fn type_object_hash_id_roundtrip() {
    let hash: EquivalenceHash = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14];
    let value = TypeObjectHashId {
        _d: EK_COMPLETE,
        hash: Some(Box::new(hash)),
    };
    let buf = serialize_to_vec(&value);
    let out: TypeObjectHashId = deserialize_from_slice(&buf);
    assert_eq!(value._d, out._d);
    assert_eq!(value.hash, out.hash);
}
