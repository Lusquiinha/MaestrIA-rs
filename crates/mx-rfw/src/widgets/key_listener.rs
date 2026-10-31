use crate::tree::{Event, Node, Widget};

pub struct KeyListener<'a> {
    pub keys: Vec<String>,
    pub on_key: Vec<&'a Event>,
    pub child: Option<&'a Widget>,
}

impl<'a> KeyListener<'a> {
    pub fn from_node(n: &'a Node) -> Self {
        let keys = n
            .arg("keys")
            .as_list()
            .map(|l| l.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect())
            .unwrap_or_default();
        Self { keys, on_key: n.handlers("onKey"), child: n.optional_child("child") }
    }
}
