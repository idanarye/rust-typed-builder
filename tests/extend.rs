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
fn default_and_implicit_initializers() {
    #[derive(TypedBuilder)]
    struct A {
        #[builder(default = vec![0], setter(extend))]
        v: Vec<i8>,
    }

    assert_eq!(A::builder().build().v, vec![0]);
    assert_eq!(A::builder().v_item(2).build().v, vec![2]);
    assert_eq!(A::builder().v(vec![3, 4]).build().v, vec![3, 4]);
}

#[test]
fn default_and_explicit_auto_initializers() {
    #[derive(TypedBuilder)]
    struct A {
        #[builder(default = vec![0], setter(extend(from_first, from_iter)))]
        v: Vec<i8>,
    }

    assert_eq!(A::builder().build().v, vec![0]);
    assert_eq!(A::builder().v_item(2).build().v, vec![2]);
    assert_eq!(A::builder().v(vec![3, 4]).build().v, vec![3, 4]);
}

#[test]
fn default_and_explicit_initializer_closures() {
    #[derive(TypedBuilder)]
    struct A {
        #[builder(default = vec![0], setter(extend(from_first = |first| vec![first], from_iter = |iter| iter.collect())))]
        v: Vec<i8>,
    }

    assert_eq!(A::builder().build().v, vec![0]);
    assert_eq!(A::builder().v_item(2).build().v, vec![2]);
    assert_eq!(A::builder().v(vec![3, 4]).build().v, vec![3, 4]);
}

#[test]
fn generic_inference() {
    #[derive(TypedBuilder)]
    struct A<T> {
        #[builder(setter(extend))]
        v: Vec<T>,
    }

    #[derive(TypedBuilder)]
    struct B<S, T> {
        #[builder(setter(extend))]
        s: Vec<S>,
        #[builder(setter(extend))]
        t: Vec<T>,
    }

    let A { v: _v } = A::builder().v(vec![true]).build();
    let _ = A::builder().v_item(0).build();

    let B { s: _s, t: _t } = B::builder().s(vec![true]).t(vec![false]).build();
    let _ = B::builder().s(vec![0]).t_item(1).build();
    let _ = B::builder().s_item('a').t(vec![false]).build();
    let _ = B::builder().s_item("b").t_item(1).build();
}

#[test]
fn strip_option() {
    #[derive(TypedBuilder)]
    struct A {
        #[builder(default, setter(strip_option, extend))]
        v: Option<Vec<u8>>,
    }

    assert_eq!(A::builder().build().v, None);
    assert_eq!(A::builder().v_item(2).build().v, Some(vec![2]));
    assert_eq!(A::builder().v(vec![3, 4]).build().v, Some(vec![3, 4]));
    assert_eq!(A::builder().v_item(5).v_item(6).build().v, Some(vec![5, 6]));
    assert_eq!(A::builder().v(vec![7, 8]).v_item(9).build().v, Some(vec![7, 8, 9]));
    assert_eq!(A::builder().v_item(0).v(vec![1, 2]).build().v, Some(vec![0, 1, 2]));
    assert_eq!(A::builder().v(vec![3, 4]).v(vec![5, 6]).build().v, Some(vec![3, 4, 5, 6]));
}

#[test]
fn strip_option_generic_inference() {
    #[derive(TypedBuilder)]
    struct A<T> {
        #[builder(default, setter(strip_option, extend))]
        v: Option<Vec<T>>,
    }

    #[derive(TypedBuilder)]
    struct B<S, T> {
        #[builder(default, setter(strip_option, extend))]
        s: Option<Vec<S>>,
        #[builder(default, setter(strip_option, extend))]
        t: Option<Vec<T>>,
    }

    let A { v: _v } = A::builder().v(vec![true]).build();
    let _ = A::builder().v_item(0).build();

    let B { s: _s, t: _t } = B::builder().s(vec![true]).t(vec![false]).build();
    let _ = B::builder().s(vec![0]).t_item(1).build();
    let _ = B::builder().s_item('a').t(vec![false]).build();
    let _ = B::builder().s_item("b").t_item(1).build();
}
