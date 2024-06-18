use typed_builder::TypedBuilder;

#[test]
fn test_simple() {
    #[derive(PartialEq, Debug, TypedBuilder)]
    enum Foo {
        Bar { x: i32 },
        Baz { y: i32, z: String },
    }

    assert_eq!(Foo::bar().x(1).build(), Foo::Bar { x: 1 });
    assert_eq!(
        Foo::baz().y(2).z("z".to_owned()).build(),
        Foo::Baz { y: 2, z: "z".to_owned() }
    );
}

#[test]
fn test_into() {
    #[derive(PartialEq, Debug, TypedBuilder)]
    #[builder(field_defaults(setter(into)))]
    enum Foo {
        Bar {
            x: i32,
        },
        Baz {
            #[builder(setter(!into))]
            y: u32,
            z: String,
        },
    }

    assert_eq!(Foo::bar().x(1_u8).build(), Foo::Bar { x: 1 });
    assert_eq!(Foo::baz().y(2).z("z").build(), Foo::Baz { y: 2, z: "z".to_owned() });
}

#[test]
fn test_default() {
    #[derive(PartialEq, Debug, TypedBuilder)]
    enum Foo {
        #[builder(field_defaults(default = 1))]
        Bar {
            #[builder(default = 2)]
            x: i32,
            y: i32,
        },
        Baz {
            #[builder(default = Some(3), setter(strip_option))]
            y: Option<i32>,
            #[builder(default = vec![1,2,3], setter(into))]
            z: Vec<i32>,
        },
    }

    assert_eq!(Foo::bar().build(), Foo::Bar { x: 2, y: 1 });
    assert_eq!(Foo::bar().x(3).y(4).build(), Foo::Bar { x: 3, y: 4 });
    assert_eq!(
        Foo::baz().build(),
        Foo::Baz {
            y: Some(3),
            z: vec![1, 2, 3]
        }
    );
    assert_eq!(
        Foo::baz().y(5).z([6, 7, 8]).build(),
        Foo::Baz {
            y: Some(5),
            z: vec![6, 7, 8]
        }
    );
}

#[test]
fn test_skip() {
    #[derive(PartialEq, Debug, TypedBuilder)]
    enum Foo {
        Bar {
            #[builder(default, setter(skip))]
            x: i32,
        },
        Baz {
            #[builder(setter(strip_option))]
            y: Option<i32>,
            #[builder(default = y.into_iter().collect(), setter(skip))]
            z: Vec<i32>,
        },
    }

    assert_eq!(Foo::bar().build(), Foo::Bar { x: 0 });
    assert_eq!(Foo::baz().y(1).build(), Foo::Baz { y: Some(1), z: vec![1] });
}

#[test]
fn test_build_method_name() {
    #[derive(PartialEq, Debug, TypedBuilder)]
    #[builder(doc, build_method(vis="", name=__build), field_defaults(default))]
    pub enum Foo {
        Bar { x: i32 },
        Baz { y: i32 },
    }

    assert_eq!(Foo::bar().x(1).__build(), Foo::Bar { x: 1 });
    assert_eq!(Foo::baz().__build(), Foo::Baz { y: 0 });
}

#[test]
fn test_prefix_and_suffix() {
    #[derive(PartialEq, Debug, TypedBuilder)]
    #[builder(field_defaults(setter(prefix = "with_")))]
    enum Foo {
        Bar {
            x: i32,
        },
        #[builder(field_defaults(setter(suffix = "_value")))]
        Baz {
            y: i32,
            #[builder(setter(prefix = ""))]
            z: i32,
        },
    }

    assert_eq!(Foo::bar().with_x(1).build(), Foo::Bar { x: 1 });
    assert_eq!(Foo::baz().with_y_value(2).z_value(3).build(), Foo::Baz { y: 2, z: 3 });
}

#[test]
fn test_builder_method() {
    #[derive(PartialEq, Debug, TypedBuilder)]
    enum Foo {
        BarBaz {
            x: i32,
        },
        QuxHTTPQuux {
            y: i32,
        },
        #[builder(builder_method(name = custom_builder))]
        Custom {
            z: i32,
        },
    }

    assert_eq!(Foo::bar_baz().x(1).build(), Foo::BarBaz { x: 1 });
    assert_eq!(Foo::qux_http_quux().y(2).build(), Foo::QuxHTTPQuux { y: 2 });
    assert_eq!(Foo::custom_builder().z(3).build(), Foo::Custom { z: 3 });
}

#[test]
fn test_builder_type_visibility() {
    mod foo {
        use typed_builder::TypedBuilder;

        #[derive(PartialEq, Debug, TypedBuilder)]
        enum Foo {
            #[builder(builder_type(vis="pub", name=CustomBuilder))]
            Bar { x: i32 },
        }

        pub fn foo_bar_builder() -> CustomBuilder {
            Foo::bar()
        }

        pub fn build_and_get_x(builder: CustomBuilder, x: i32) -> i32 {
            let Foo::Bar { x } = builder.x(x).build();
            x
        }
    }

    let builder: foo::CustomBuilder = foo::foo_bar_builder();
    assert_eq!(foo::build_and_get_x(builder, 1), 1);
}

#[test]
fn test_builder_on_enum_with_keywords() {
    #[allow(non_camel_case_types)]
    #[derive(PartialEq, Debug, TypedBuilder)]
    enum r#enum {
        Bar { r#type: i32 },
    }

    assert_eq!(r#enum::bar().r#type(1).build(), r#enum::Bar { r#type: 1 });
}
