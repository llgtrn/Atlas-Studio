#[derive(rkyv::Archive, rkyv::Serialize)]
#[rkyv(as = rkyv::rend::i32_le)]
enum NotSelf {
    Variant(i32),
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Portable)]
#[rkyv(as = Self)]
enum NotPortable {
    Bar,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Portable)]
#[rkyv(
    as = WithGenerics<T>,
    archive_bounds(
        T: rkyv::Portable,
        T: rkyv::Archive<Archived = T>,
    ),
)]
#[repr(u8)]
enum WithGenerics<T> {
    Foo(T),
}

fn main() {
    rkyv::to_bytes::<rkyv::rancor::Failure>(&WithGenerics::Foo(12i32)).unwrap();
    rkyv::to_bytes::<rkyv::rancor::Failure>(&WithGenerics::Foo(())).unwrap();
}
