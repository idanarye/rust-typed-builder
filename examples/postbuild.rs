use typed_builder::{PostBuild, TypedBuilder};

#[derive(Debug, PartialEq, TypedBuilder)]
#[builder(postbuild)]
struct Foo {
    x: i32,
    y: i32,
}

impl PostBuild for Foo {
    type Output = Result<Self, String>;

    fn postbuild(self) -> Self::Output {
        if self.x >= 5 {
            return Err("x too high - must be below or 5".into());
        }

        Ok(self)
    }
}

fn main() {
    let foo = Foo::builder().x(1).y(2).build().unwrap();
    assert_eq!(foo, Foo { x: 1, y: 2 });

    // Fails to validate during runtime
    // let foo = Foo::builder().x(5).y(6).build().unwrap();
    // assert_eq!(foo, Foo { x: 5, y: 6 });
}
