#![warn(clippy::pedantic)]

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
fn test_lifetime_bounded() {
    #[derive(PartialEq, TypedBuilder)]
    struct Foo<'a, 'b: 'a> {
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

    assert!(Foo::builder().x(1_u8).build() == Foo { x: 1 });
}

#[test]
fn test_strip_option_with_into() {
    #[derive(PartialEq, TypedBuilder)]
    struct Foo {
        #[builder(setter(strip_option, into))]
        x: Option<i32>,
    }

    assert!(Foo::builder().x(1_u8).build() == Foo { x: Some(1) });
}

#[test]
fn test_into_with_strip_option() {
    #[derive(PartialEq, TypedBuilder)]
    struct Foo {
        #[builder(setter(into, strip_option))]
        x: Option<i32>,
    }

    assert!(Foo::builder().x(1_u8).build() == Foo { x: Some(1) });
}

#[test]
fn test_strip_bool() {
    #[derive(PartialEq, TypedBuilder)]
    struct Foo {
        #[builder(setter(into, strip_bool))]
        x: bool,
    }

    assert!(Foo::builder().x().build() == Foo { x: true });
    assert!(Foo::builder().build() == Foo { x: false });
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

    assert!(Foo::builder().y(1_u8).build() == Foo { x: 0, y: 1, z: 2 });
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
                    doc = "Set `z`. If you don't specify a value it'll default to the value specified for `x`.",
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
    assert!(
        Foo::builder().y("hello".to_owned()).x_default().build()
            == Foo {
                x: "",
                y: "hello".to_owned()
            }
    );
}

#[test]
#[allow(clippy::items_after_statements)]
fn test_builder_type_with_default_on_generic_type() {
    #[derive(PartialEq, TypedBuilder)]
    struct Types<X, Y = ()> {
        x: X,
        y: Y,
    }
    assert!(Types::builder().x(()).y(()).build() == Types { x: (), y: () });

    #[derive(PartialEq, TypedBuilder)]
    struct TypeAndLifetime<'a, X, Y: Default, Z = usize> {
        x: X,
        y: Y,
        z: &'a Z,
    }
    let a = 0;
    assert!(TypeAndLifetime::builder().x(()).y(0).z(&a).build() == TypeAndLifetime { x: (), y: 0, z: &0 });

    #[derive(PartialEq, TypedBuilder)]
    struct Foo<'a, X, Y: Default, Z: Default = usize, M = ()> {
        x: X,
        y: &'a Y,
        z: Z,
        m: M,
    }

    impl<'a, X, Y: Default, M, X_, Y_, M_> FooBuilder<'a, (X_, Y_, (), M_), X, Y, usize, M> {
        fn z_default(self) -> FooBuilder<'a, (X_, Y_, (usize,), M_), X, Y, usize, M> {
            self.z(usize::default())
        }
    }

    impl<'a, X, Y: Default, Z: Default, X_, Y_, Z_> FooBuilder<'a, (X_, Y_, Z_, ()), X, Y, Z, ()> {
        fn m_default(self) -> FooBuilder<'a, (X_, Y_, Z_, ((),)), X, Y, Z, ()> {
            self.m(())
        }
    }

    // compile test if rustc can infer type for `z` and `m`
    Foo::<(), _, _, f64>::builder().x(()).y(&a).z_default().m(1.0).build();
    Foo::<(), _, _, _>::builder().x(()).y(&a).z_default().m_default().build();

    assert!(
        Foo::builder().x(()).y(&a).z_default().m(1.0).build()
            == Foo {
                x: (),
                y: &0,
                z: 0,
                m: 1.0
            }
    );
    assert!(
        Foo::builder().x(()).y(&a).z(9).m(1.0).build()
            == Foo {
                x: (),
                y: &0,
                z: 9,
                m: 1.0
            }
    );
}

#[test]
fn test_builder_type_skip_into() {
    #[derive(PartialEq, TypedBuilder)]
    struct Foo<X> {
        x: X,
    }

    // compile test if rustc can infer type for `x`
    Foo::builder().x(()).build();

    assert!(Foo::builder().x(()).build() == Foo { x: () });
}

#[test]
fn test_default_code() {
    #[derive(PartialEq, TypedBuilder)]
    struct Foo {
        #[builder(default_code = "\"text1\".to_owned()")]
        x: String,

        #[builder(default_code = r#""text2".to_owned()"#)]
        y: String,
    }

    assert!(
        Foo::builder().build()
            == Foo {
                x: "text1".to_owned(),
                y: "text2".to_owned()
            }
    );
}

#[test]
fn test_field_defaults_default_value() {
    #[derive(PartialEq, TypedBuilder)]
    #[builder(field_defaults(default = 12))]
    struct Foo {
        x: i32,
        #[builder(!default)]
        y: String,
        #[builder(default = 13)]
        z: i32,
    }

    assert!(
        Foo::builder().y("bla".to_owned()).build()
            == Foo {
                x: 12,
                y: "bla".to_owned(),
                z: 13
            }
    );
}

#[test]
fn test_field_defaults_setter_options() {
    #[derive(PartialEq, TypedBuilder)]
    #[builder(field_defaults(setter(strip_option)))]
    struct Foo {
        x: Option<i32>,
        #[builder(setter(!strip_option))]
        y: i32,
    }

    assert!(Foo::builder().x(1).y(2).build() == Foo { x: Some(1), y: 2 });
}

#[test]
fn test_clone_builder() {
    #[derive(PartialEq, Default)]
    struct Uncloneable;

    #[derive(PartialEq, TypedBuilder)]
    struct Foo {
        x: i32,
        y: i32,
        #[builder(default)]
        z: Uncloneable,
    }

    let semi_built = Foo::builder().x(1);

    assert!(
        semi_built.clone().y(2).build()
            == Foo {
                x: 1,
                y: 2,
                z: Uncloneable
            }
    );
    assert!(
        semi_built.y(3).build()
            == Foo {
                x: 1,
                y: 3,
                z: Uncloneable
            }
    );
}

#[test]
#[allow(clippy::items_after_statements)]
fn test_clone_builder_with_generics() {
    #[derive(PartialEq, Default)]
    struct Uncloneable;

    #[derive(PartialEq, TypedBuilder)]
    struct Foo<T> {
        x: T,
        y: i32,
    }

    let semi_built1 = Foo::builder().x(1);

    assert!(semi_built1.clone().y(2).build() == Foo { x: 1, y: 2 });
    assert!(semi_built1.y(3).build() == Foo { x: 1, y: 3 });

    let semi_built2 = Foo::builder().x("four");

    assert!(semi_built2.clone().y(5).build() == Foo { x: "four", y: 5 });
    assert!(semi_built2.clone().y(6).build() == Foo { x: "four", y: 6 });

    // Just to make sure it can build with generic bounds
    #[allow(dead_code)]
    #[derive(TypedBuilder)]
    struct Bar<T: std::fmt::Debug>
    where
        T: std::fmt::Display,
    {
        x: T,
    }
}

#[test]
fn test_builder_on_struct_with_keywords() {
    #[allow(non_camel_case_types)]
    #[derive(PartialEq, TypedBuilder)]
    struct r#struct {
        r#fn: u32,
        #[builder(default, setter(strip_option))]
        r#type: Option<u32>,
        #[builder(default = Some(()), setter(skip))]
        r#enum: Option<()>,
        #[builder(setter(into))]
        r#union: String,
    }

    assert!(
        r#struct::builder().r#fn(1).r#union("two").build()
            == r#struct {
                r#fn: 1,
                r#type: None,
                r#enum: Some(()),
                r#union: "two".to_owned(),
            }
    );
}

#[test]
fn test_field_setter_transform() {
    #[derive(PartialEq)]
    struct Point {
        x: i32,
        y: i32,
    }

    #[derive(PartialEq, TypedBuilder)]
    struct Foo {
        #[builder(setter(transform = |x: i32, y: i32| Point { x, y }))]
        point: Point,
    }

    assert!(
        Foo::builder().point(1, 2).build()
            == Foo {
                point: Point { x: 1, y: 2 }
            }
    );
}
