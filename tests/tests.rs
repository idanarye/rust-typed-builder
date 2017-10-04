#[macro_use]
extern crate typed_builder;

#[test]
fn test_simple() {
    #[derive(PartialEq, TypedBuilder)]
    struct Foo {
        x: i32,
        y: i32,
    }

    assert!(Foo::builder().x(1).y(2).build() == Foo { x: 1, y: 2 });
    assert!(Foo::builder().y(1).x(2).build() == Foo { x: 2, y: 1 });
}

#[test]
fn test_lifetime() {
    #[derive(PartialEq, TypedBuilder)]
    struct Foo<'a, 'b> {
        x: &'a i32,
        y: &'b i32,
    }

    assert!(Foo::builder().x(&1).y(&2).build() == Foo { x: &1, y: &2 });
}

#[test]
fn test_generics() {
    #[derive(PartialEq, TypedBuilder)]
    struct Foo<S, T: Default> {
        x: S,
        y: T,
    }

    assert!(Foo::builder().x(1).y(2).build() == Foo { x: 1, y: 2 });
}

#[test]
fn test_into() {
    #[derive(PartialEq, TypedBuilder)]
    struct Foo {
        x: i32,
    }

    assert!(Foo::builder().x(1u8).build() == Foo { x: 1 });
}

#[test]
fn test_default() {
    #[derive(PartialEq, TypedBuilder)]
    struct Foo {
        #[default]
        x: Option<i32>,
        #[default="-1"]
        y: i32,
    }

    assert!(Foo::builder().build() == Foo { x: None, y: -1 });
    assert!(Foo::builder().x(1).build() == Foo { x: Some(1), y: -1 });
    assert!(Foo::builder().y(2).build() == Foo { x: None, y: 2 });
    assert!(Foo::builder().x(1).y(2).build() == Foo { x: Some(1), y: 2 });
}
