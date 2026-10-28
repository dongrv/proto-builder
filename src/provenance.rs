use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize)]
pub struct Origin {
    pub kind: String,
    pub name: String,
    pub value: Value,
}
#[derive(Clone, Debug, Default)]
pub struct Origins(pub BTreeMap<String, Vec<Origin>>);
impl Origins {
    pub fn record(&mut self, key: &str, value: Value, kind: &str, name: &str) {
        self.0.entry(key.into()).or_default().push(Origin {
            kind: kind.into(),
            name: name.into(),
            value,
        });
    }
    pub fn layer(&mut self, value: &Value, kind: &str, name: &str) {
        for (key, value) in flatten(value) {
            self.record(&key, value, kind, name);
        }
    }
}
pub fn flatten(value: &Value) -> BTreeMap<String, Value> {
    fn visit(prefix: String, value: &Value, out: &mut BTreeMap<String, Value>) {
        if let Some(object) = value.as_object() {
            for (key, value) in object {
                visit(
                    if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{prefix}.{key}")
                    },
                    value,
                    out,
                );
            }
        } else {
            out.insert(prefix, value.clone());
        }
    }
    let mut out = BTreeMap::new();
    visit(String::new(), value, &mut out);
    out
}
