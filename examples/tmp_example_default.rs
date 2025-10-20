#![allow(unused_imports, unused_attributes)]

use typed_builder::TypedBuilder;
use typed_builder::NextFieldDefault;

#[derive(Debug, PartialEq, TypedBuilder)]
pub struct Foo {
    #[builder(default)]
    pub bar: i32,
    #[builder(default = format!("{bar}"))]
    pub baz: String,
    #[builder(default = *bar)]
    pub qux: i32,
}

fn main() {
    println!("{:?}", Foo::builder().bar(42).build());
    println!("{:?}", Foo::builder().bar(42).baz("hello".to_owned()).build());
}
