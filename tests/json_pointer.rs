use oas::json_pointer::{escape, tokens};

#[test]
fn string_form_preserves_rfc6901_reference_tokens() {
    for (pointer, expected) in [
        ("", vec![]),
        ("/", vec![""]),
        ("/foo/0", vec!["foo", "0"]),
        ("/a~1b/m~0n/~01", vec!["a/b", "m~n", "~1"]),
        ("/雪/%20/", vec!["雪", "%20", ""]),
    ] {
        assert_eq!(tokens(pointer).unwrap(), expected);
    }
    assert_eq!(escape("a/b~c"), "a~1b~0c");
    assert_eq!(escape("雪%20"), "雪%20");
    for pointer in ["foo", "#/foo", "/~", "/~2"] {
        assert!(tokens(pointer).is_err());
    }
}
