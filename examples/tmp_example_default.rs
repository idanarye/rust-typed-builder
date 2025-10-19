#![allow(unused_imports, unused_attributes)]

use typed_builder::TypedBuilder;
use typed_builder::TypedBuilderNextFieldDefault;

#[derive(Debug, PartialEq)]
pub struct Foo {
    pub bar: i32,
    pub baz: String,
}

fn main() {
    println!("{:?}", Foo::builder().bar(42).build());
    println!("{:?}", Foo::builder().bar(42).baz("hello".to_owned()).build());
}

// Recursive expansion of the TypedBuilder macro
// =============================================

#[automatically_derived]
impl Foo {
    #[doc = "\n                Create a builder for building `Foo`.\n                On the builder, call `.bar(...)`, `.baz(...)`(optional) to set the values of the fields.\n                Finally, call `.build()` to create the instance of `Foo`.\n                "]
    #[allow(dead_code, clippy::default_trait_access)]
    pub fn builder() -> FooBuilder<((), ())> {
        FooBuilder {
            fields: ((), ()),
            phantom: ::core::default::Default::default(),
        }
    }
}
#[must_use]
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
pub struct FooBuilder<TypedBuilderFields = ((), ())> {
    fields: TypedBuilderFields,
    phantom: ::core::marker::PhantomData<()>,
}
#[automatically_derived]
impl<TypedBuilderFields> Clone for FooBuilder<TypedBuilderFields>
where
    TypedBuilderFields: Clone,
{
    #[allow(clippy::default_trait_access)]
    fn clone(&self) -> Self {
        Self {
            fields: self.fields.clone(),
            phantom: ::core::default::Default::default(),
        }
    }
}
#[allow(dead_code, non_camel_case_types, missing_docs)]
#[automatically_derived]
impl<__baz> FooBuilder<((), __baz)> {
    #[allow(clippy::used_underscore_binding, clippy::no_effect_underscore_binding)]
    pub fn bar(self, bar: i32) -> FooBuilder<((i32,), __baz)> {
        let bar = (bar,);
        let ((), baz) = self.fields;
        FooBuilder {
            fields: (bar, baz),
            phantom: self.phantom,
        }
    }
}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
#[allow(clippy::exhaustive_enums)]
pub enum FooBuilder_Error_Repeated_field_bar {}

#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, missing_docs)]
#[automatically_derived]
impl<__baz> FooBuilder<((i32,), __baz)> {
    #[deprecated(note = "Repeated field bar")]
    pub fn bar(self, _: FooBuilder_Error_Repeated_field_bar) -> FooBuilder<((i32,), __baz)> {
        self
    }
}
#[allow(dead_code, non_camel_case_types, missing_docs)]
#[automatically_derived]
impl<__bar> FooBuilder<(__bar, ())> {
    #[allow(clippy::used_underscore_binding, clippy::no_effect_underscore_binding)]
    pub fn baz(self, baz: String) -> FooBuilder<(__bar, (String,))> {
        let baz = (baz,);
        let (bar, ()) = self.fields;
        FooBuilder {
            fields: (bar, baz),
            phantom: self.phantom,
        }
    }
}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
#[allow(clippy::exhaustive_enums)]
pub enum FooBuilder_Error_Repeated_field_baz {}

#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, missing_docs)]
#[automatically_derived]
impl<__bar> FooBuilder<(__bar, (String,))> {
    #[deprecated(note = "Repeated field baz")]
    pub fn baz(self, _: FooBuilder_Error_Repeated_field_baz) -> FooBuilder<(__bar, (String,))> {
        self
    }
}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
#[allow(clippy::exhaustive_enums)]
pub enum FooBuilder_Error_Missing_required_field_bar {}

#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, missing_docs, clippy::panic)]
#[automatically_derived]
impl<__baz> FooBuilder<((), __baz)> {
    #[deprecated(note = "Missing required field bar")]
    pub fn build(self, _: FooBuilder_Error_Missing_required_field_bar) -> ! {
        panic!()
    }
}
#[allow(dead_code, non_camel_case_types, missing_docs)]
#[automatically_derived]
impl<__baz> FooBuilder<((i32,), __baz)>
where
    Foo: for<'a> TypedBuilderNextFieldDefault<(&'a i32, __baz), Output = String>,
{
    #[allow(
        clippy::default_trait_access,
        clippy::used_underscore_binding,
        clippy::no_effect_underscore_binding
    )]
    pub fn build(self) -> Foo {
        let (f0, f1) = self.fields;

        let bar = <Foo as TypedBuilderNextFieldDefault<((i32,),)>>::resolve((f0,));
        let baz = <Foo as TypedBuilderNextFieldDefault<(&i32, __baz)>>::resolve((&bar, f1));

        #[allow(deprecated)]
        Foo { bar, baz }.into()
    }
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
