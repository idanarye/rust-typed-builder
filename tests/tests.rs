extern crate typed_builder;

use typed_builder::TypedBuilder;

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
        #[builder(default)]
        x: Option<i32>,
        #[builder(default = 10)]
        y: i32,
        #[builder(default_code = "vec![20, 30, 40]")]
        z: Vec<i32>,
    }

    assert!(
        Foo::builder().build()
            == Foo {
                x: None,
                y: 10,
                z: vec![20, 30, 40]
            }
    );
    assert!(
        Foo::builder().x(1).build()
            == Foo {
                x: Some(1),
                y: 10,
                z: vec![20, 30, 40]
            }
    );
    assert!(
        Foo::builder().y(2).build()
            == Foo {
                x: None,
                y: 2,
                z: vec![20, 30, 40]
            }
    );
    assert!(
        Foo::builder().x(1).y(2).build()
            == Foo {
                x: Some(1),
                y: 2,
                z: vec![20, 30, 40]
            }
    );
    assert!(
        Foo::builder().z(vec![1, 2, 3]).build()
            == Foo {
                x: None,
                y: 10,
                z: vec![1, 2, 3]
            }
    );
}

#[test]
fn test_field_dependencies_in_build() {
    #[derive(PartialEq, TypedBuilder)]
    struct Foo {
        #[builder(default)]
        x: Option<i32>,
        #[builder(default = 10)]
        y: i32,
        #[builder(default_code = "vec![y, 30, 40]")]
        z: Vec<i32>,
    }

    assert!(
        Foo::builder().build()
            == Foo {
                x: None,
                y: 10,
                z: vec![10, 30, 40]
            }
    );
    assert!(
        Foo::builder().x(1).build()
            == Foo {
                x: Some(1),
                y: 10,
                z: vec![10, 30, 40]
            }
    );
    assert!(
        Foo::builder().y(2).build()
            == Foo {
                x: None,
                y: 2,
                z: vec![2, 30, 40]
            }
    );
    assert!(
        Foo::builder().x(1).y(2).build()
            == Foo {
                x: Some(1),
                y: 2,
                z: vec![2, 30, 40]
            }
    );
    assert!(
        Foo::builder().z(vec![1, 2, 3]).build()
            == Foo {
                x: None,
                y: 10,
                z: vec![1, 2, 3]
            }
    );
}

// compile-fail tests for exclude are in src/lib.rs out of necessity. These are just the bland
// successful cases.
#[test]
fn test_exclude() {
    #[derive(PartialEq, TypedBuilder)]
    struct Foo {
        #[builder(exclude, default)]
        x: i32,
        y: i32,
        #[builder(exclude, default_code = "y + 1")]
        z: i32,
    }

    assert!(Foo::builder().y(1u8).build() == Foo { x: 0, y: 1, z: 2 });
}

#[test]
fn test_docs() {
    #[derive(TypedBuilder)]
    #[builder(
        builder_method_doc = "Point::builder() method docs",
        builder_type_doc = "PointBuilder type docs",
        build_method_doc = "PointBuilder.build() method docs"
    )]
    struct Point {
        x: i32,
        #[builder(
            doc = "
                Set `z`. If you don’t specify a value it’ll default to the value specified for `x`.
                ",
            default_code = "x"
        )]
        y: i32,
    }

    let _ = Point::builder();
}

#[test]
fn test_builder_name() {
    #[derive(TypedBuilder)]
    struct Foo {}

    let _: FooBuilder<_> = Foo::builder();
}
