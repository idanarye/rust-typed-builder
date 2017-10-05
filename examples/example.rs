#[macro_use]
extern crate typed_builder;

#[derive(PartialEq, TypedBuilder)]
struct Foo {
    // Mandatory Field:
    x: i32,

    // #[default] without parameter - use the type's default
    #[default]
    y: Option<i32>,

    // Or you can set the default(encoded as string)
    #[default="20"]
    z: i32,
}

fn main() {
    assert!(
        Foo::builder().x(1).y(2).z(3).build()
        == Foo { x: 1, y: Some(2), z: 3 });

    // Change the order of construction:
    assert!(
        Foo::builder().z(1).x(2).y(3).build()
        == Foo { x: 2, y: Some(3), z: 1 });

    // Optional fields are optional:
    assert!(
        Foo::builder().x(1).build()
        == Foo { x: 1, y: None, z: 20 });

    // This will not compile - because we did not set x:
    // Foo::builder().build();

    // This will not compile - because we set y twice:
    // Foo::builder().x(1).y(2).y(3);
}
