use typed_builder::TypedBuilder;

#[derive(TypedBuilder)]
pub struct Props<'a, OnInput: FnOnce(usize) -> usize = Box<dyn FnOnce(usize) -> usize>> {
    #[builder(default, setter(into))]
    pub class: Option<&'a str>,
    pub label: &'a str,
    #[builder(setter(into))]
    pub on_input: Option<OnInput>,
}

#[derive(TypedBuilder)]
struct Foo<T = usize> {
    #[builder(default = 12)]
    x: T,
}

#[allow(dead_code)]
#[derive(TypedBuilder)]
struct Bar<T, U = usize, V = usize> {
    t: T,
    #[builder(default = 12)]
    u: U,
    v: (T, U, V),
}

fn main() {
    let props = Props::builder().label("label").on_input(|x: usize| x).build();
    assert_eq!(props.class, None);
    assert_eq!(props.label, "label");
    assert_eq!((props.on_input.unwrap())(123), 123);

    assert_eq!(Foo::builder().build().x, 12);

    assert_eq!(Bar::builder().t("test").v(("t", 0, 3.14f64)).build().v.0, "t");
}
