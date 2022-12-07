use typed_builder::TypedBuilder;

#[derive(TypedBuilder)]
pub struct Props<'a, OnInput: FnOnce(usize) -> usize = Box<dyn FnOnce(usize) -> usize>> {
    #[builder(default, setter(into))]
    pub class: Option<&'a str>,
    pub label: &'a str,
    #[builder(setter(into))]
    pub on_input: Option<OnInput>,
}

fn main() {
    let props = Props::builder().label("label").on_input(|x: usize| x).build();
    assert_eq!(props.class, None);
    assert_eq!(props.label, "label");
    assert_eq!((props.on_input.unwrap())(123), 123);
}
