mod common;

use common::content;
use mx_rfw::widgets::key_listener::KeyListener;
use mx_rfw::widgets::{Local, decode};
use mx_rfw::{LocalLibrary, Widget, evaluate, parse_library_file};

fn tree(src: &str, data: &str) -> Widget {
    let lib = parse_library_file(&format!("import core.widgets;\nimport maestria;\n{src}")).unwrap();
    evaluate(&lib, "root", &content(data))
}

fn listener(t: &Widget) -> KeyListener<'_> {
    let Local::KeyListener(k) = decode(t.as_node().unwrap()) else { panic!("not a KeyListener") };
    k
}

#[test]
fn key_listener_is_part_of_the_maestria_library() {
    assert!(LocalLibrary::Maestria.has("KeyListener"));
}

#[test]
fn keys_on_key_and_child_are_read() {
    let t = tree(
        r#"widget root = KeyListener(
  keys: ["ctrl+c", "escape", "ctrl+0", "ctrl+=", "ctrl+-"],
  onKey: event "act" {a: "key"},
  child: Text(text: "canvas"),
);"#,
        "{}",
    );
    let k = listener(&t);
    assert_eq!(k.keys, ["ctrl+c", "escape", "ctrl+0", "ctrl+=", "ctrl+-"]);
    assert_eq!(k.on_key.len(), 1);
    assert_eq!(k.on_key[0].name, "act");
    assert!(k.child.is_some());
}

#[test]
fn keys_from_data_and_non_strings_are_skipped() {
    let t = tree(r#"widget root = KeyListener(keys: data.k, child: Text(text: "x"));"#, r#"{"k": ["escape", 3, "ctrl+c"]}"#);
    assert_eq!(listener(&t).keys, ["escape", "ctrl+c"]);
    let t = tree("widget root = KeyListener();", "{}");
    let k = listener(&t);
    assert!(k.keys.is_empty() && k.on_key.is_empty() && k.child.is_none());
}
