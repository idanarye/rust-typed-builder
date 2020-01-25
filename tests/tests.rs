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
fn test_mutable_borrows() {
    #[derive(PartialEq, TypedBuilder)]
    struct Foo<'a, 'b> {
        x: &'a mut i32,
        y: &'b mut i32,
    }

    let mut a = 1;
    let mut b = 2;
    {
        let foo = Foo::builder().x(&mut a).y(&mut b).build();
        *foo.x *= 10;
        *foo.y *= 100;
    }
    assert!(a == 10);
    assert!(b == 200);
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
        #[builder(setter(into))]
        x: i32,
    }

    assert!(Foo::builder().x(1u8).build() == Foo { x: 1 });
}

#[test]
fn test_default() {
    #[derive(PartialEq, TypedBuilder)]
    struct Foo {
        #[builder(default, setter(strip_option))]
        x: Option<i32>,
        #[builder(default = 10)]
        y: i32,
        #[builder(default = vec![20, 30, 40])]
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
        #[builder(default, setter(strip_option))]
        x: Option<i32>,
        #[builder(default = 10)]
        y: i32,
        #[builder(default = vec![y, 30, 40])]
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

// compile-fail tests for skip are in src/lib.rs out of necessity. These are just the bland
// successful cases.
#[test]
fn test_skip() {
    #[derive(PartialEq, TypedBuilder)]
    struct Foo {
        #[builder(default, setter(skip))]
        x: i32,
        #[builder(setter(into))]
        y: i32,
        #[builder(default = y + 1, setter(skip))]
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
        #[allow(dead_code)]
        x: i32,
        #[builder(
            default = x,
            setter(
                doc = "Set `z`. If you don’t specify a value it’ll default to the value specified for `x`.",
            ),
        )]
        #[allow(dead_code)]
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

// NOTE: `test_builder_type_stability` and `test_builder_type_stability_with_other_generics` are
//       meant to ensure we don't break things for people that use custom `impl`s on the builder
//       type before the tuple field generic param transformation traits are in.
//       See:
//        - https://github.com/idanarye/rust-typed-builder/issues/22
//        - https://github.com/idanarye/rust-typed-builder/issues/23
#[test]
fn test_builder_type_stability() {
    #[derive(PartialEq, TypedBuilder)]
    struct Foo {
        x: i32,
        y: i32,
        z: i32,
    }

    impl<Y> FooBuilder<((), Y, ())> {
        fn xz(self, x: i32, z: i32) -> FooBuilder<((i32,), Y, (i32,))> {
            self.x(x).z(z)
        }
    }

    assert!(Foo::builder().xz(1, 2).y(3).build() == Foo { x: 1, y: 3, z: 2 });
    assert!(Foo::builder().xz(1, 2).y(3).build() == Foo::builder().x(1).z(2).y(3).build());

    assert!(Foo::builder().y(1).xz(2, 3).build() == Foo { x: 2, y: 1, z: 3 });
    assert!(Foo::builder().y(1).xz(2, 3).build() == Foo::builder().y(1).x(2).z(3).build());
}

#[test]
fn test_builder_type_stability_with_other_generics() {
    #[derive(PartialEq, TypedBuilder)]
    struct Foo<X: Default, Y> {
        x: X,
        y: Y,
    }

    impl<X: Default, Y, Y_> FooBuilder<((), Y_), X, Y> {
        fn x_default(self) -> FooBuilder<((X,), Y_), X, Y> {
            self.x(X::default())
        }
    }

    assert!(Foo::builder().x_default().y(1.0).build() == Foo { x: 0, y: 1.0 });
    assert!(Foo::builder().y("hello".to_owned()).x_default().build() == Foo { x: "", y: "hello".to_owned() });
}

#[test]
fn test_builder_type_with_default_on_generic_type() {
    #[derive(PartialEq, TypedBuilder)]
    struct Types<X, Y=()> {
        x: X,
        y: Y,
    }
    assert!(Types::builder().x(()).y(()).build() == Types { x:(), y: () });

    #[derive(PartialEq, TypedBuilder)]
    struct TypeAndLifetime<'a, X,Y:Default, Z=usize> {
        x: X,
        y: Y,
        z:&'a Z,
    }
    let a = 0;
    assert!(TypeAndLifetime::builder().x(()).y(0).z(&a).build() == TypeAndLifetime { x:(), y: 0, z:&0 });

    #[derive(PartialEq, TypedBuilder)]
    struct Foo<'a, X, Y: Default, Z:Default=usize, M =()> {
        x: X,
        y: &'a Y,
        z: Z,
        m: M
    }

    impl<'a, X, Y: Default, M, X_, Y_, M_> FooBuilder<'a, (X_, Y_, (), M_), X, Y, usize, M> {
        fn z_default(self) -> FooBuilder<'a, (X_, Y_, (usize,), M_), X, Y, usize, M> {
            self.z(usize::default())
        }
    }

    impl<'a, X, Y: Default, Z:Default, X_, Y_, Z_> FooBuilder<'a, (X_, Y_, Z_, ()), X, Y, Z, ()> {
        fn m_default(self) -> FooBuilder<'a, (X_, Y_, Z_, ((),)), X, Y, Z, ()> {
            self.m(())
        }
    }

    // compile test if rustc can infer type for `z` and `m`
    Foo::<(), _, _, f64>::builder().x(()).y(&a).z_default().m(1.0).build();
    Foo::<(), _, _, _>::builder().x(()).y(&a).z_default().m_default().build();

    assert!(Foo::builder().x(()).y(&a).z_default().m(1.0).build() == Foo { x:(), y: &0, z: 0, m:1.0 });
    assert!(Foo::builder().x(()).y(&a).z(9).m(1.0).build() == Foo { x:(), y: &0, z: 9, m:1.0 });
}

#[test]
fn test_builder_type_skip_into() {

    #[derive(PartialEq, TypedBuilder)]
    struct Foo<X> {
        x: X,
    }

    // compile test if rustc can infer type for `x`
    Foo::builder().x(()).build();

    assert!(Foo::builder().x(()).build() == Foo { x:()});
}
