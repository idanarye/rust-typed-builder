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

#[derive(PartialEq, TypedBuilder)]
struct Bar(
    i32,
    // #[default] without parameter - use the type's default
    #[default]
    Option<i32>,

    // Or you can set the default(encoded as string)
    #[default="20"]
    i32,
);

#[derive(PartialEq, TypedBuilder)]
struct Baz;

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


    assert!(
        Bar::builder()._0(1)._1(2)._2(3).build()
        == Bar(1, Some(2), 3));

    // Change the order of construction:
    assert!(
        Bar::builder()._2(1)._0(2)._1(3).build()
        == Bar(2, Some(3), 1));

    // Optional fields are optional:
    assert!(
        Bar::builder()._0(1).build()
        == Bar(1, None, 20));

    // This will not compile - because we did not set `0`:
    // Bar::builder().build();

    // This will not compile - because we set `1` twice:
    // Bar::builder()._0(1)._1(2)._1(3);


    assert!(
        Baz::builder().build()
        == Baz
    );
}
