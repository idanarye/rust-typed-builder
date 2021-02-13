use typed_builder::TypedBuilder;

#[test]
fn simple_vec() {
    #[derive(TypedBuilder)]
    struct A {
        #[builder(setter(extend))]
        v: Vec<i8>,
    }

    assert_eq!(A::builder().v_item(2).build().v, vec![2]);
    assert_eq!(A::builder().v(vec![3, 4]).build().v, vec![3, 4]);
    assert_eq!(A::builder().v_item(5).v_item(6).build().v, vec![5, 6]);
    assert_eq!(A::builder().v(vec![7, 8]).v_item(9).build().v, vec![7, 8, 9]);
    assert_eq!(A::builder().v_item(0).v(vec![1, 2]).build().v, vec![0, 1, 2]);
    assert_eq!(A::builder().v(vec![3, 4]).v(vec![5, 6]).build().v, vec![3, 4, 5, 6]);
}

#[test]
fn item_name() {
    #[derive(TypedBuilder)]
    struct A {
        #[builder(setter(extend(from_first, from_iter, item_name = i)))]
        v: Vec<i8>,
    }

    assert_eq!(A::builder().i(2).build().v, vec![2]);
    assert_eq!(A::builder().i(5).i(6).build().v, vec![5, 6]);
    assert_eq!(A::builder().v(vec![7, 8]).i(9).build().v, vec![7, 8, 9]);
    assert_eq!(A::builder().i(0).v(vec![1, 2]).build().v, vec![0, 1, 2]);
}

#[test]
fn extend_default() {
    #[derive(TypedBuilder)]
    struct A {
        #[builder(default = vec![0], setter(extend))]
        v: Vec<i8>,
    }

    assert_eq!(A::builder().v_item(2).build().v, vec![0, 2]);
    assert_eq!(A::builder().v(vec![3, 4]).build().v, vec![0, 3, 4]);
}

#[test]
fn extend_default_explicit_auto() {
    #[derive(TypedBuilder)]
    struct A {
        #[builder(default = vec![0], setter(extend(from_first, from_iter)))]
        v: Vec<i8>,
    }

    assert_eq!(A::builder().v_item(2).build().v, vec![0, 2]);
    assert_eq!(A::builder().v(vec![3, 4]).build().v, vec![0, 3, 4]);
}

#[test]
fn ignore_default() {
    #[derive(TypedBuilder)]
    struct A {
        #[builder(default = vec![0], setter(extend(from_first = |first| vec![first], from_iter = |iter| iter.collect())))]
        v: Vec<i8>,
    }

    assert_eq!(A::builder().v_item(2).build().v, vec![2]);
    assert_eq!(A::builder().v(vec![3, 4]).build().v, vec![3, 4]);
}
