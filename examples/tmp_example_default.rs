#![allow(unused_imports, unused_attributes)]

use typed_builder::TypedBuilder;
use typed_builder::TypedBuilderNextFieldDefault;

#[derive(Debug, PartialEq, TypedBuilder)]
pub struct Foo {
    pub bar: i32,
    #[builder(default = format!("{bar}"))]
    pub baz: String,
}

fn main() {
    println!("{:?}", Foo::builder().bar(42).build());
    println!("{:?}", Foo::builder().bar(42).baz("hello".to_owned()).build());
}

// bar, when set
impl TypedBuilderNextFieldDefault<((i32,),)> for Foo {
    type Output = i32;

    fn resolve((.., (input,)): ((i32,),)) -> Self::Output {
        input
    }
}

// baz, when set
impl TypedBuilderNextFieldDefault<(&i32, (String,))> for Foo {
    type Output = String;

    fn resolve((.., (input,)): (&i32, (String,))) -> Self::Output {
        input
    }
}

// baz, when default
impl TypedBuilderNextFieldDefault<(&i32, ())> for Foo {
    type Output = String;

    fn resolve((bar, ()): (&i32, ())) -> Self::Output {
        format!("bar is {bar} (new style)")
    }
}
