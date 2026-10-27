//! Exact Windows Python export_cmd/build_proto JSON compatibility. No Python at runtime.
use crate::{config::Settings, scan, util};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Message {
    id: i64,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    module: Option<String>,
}
#[derive(Serialize)]
struct Compact<'a> {
    id: i64,
    name: &'a str,
}
pub fn generate(root: &Path, out: &Path, s: &Settings) -> Result<Vec<PathBuf>> {
    let proto = s.proto(root);
    let exclude = scan::Excludes::new(&s.excludes)?;
    let package = regex::Regex::new(r"package[ \t]+(\w+);")?;
    let sequence = regex::Regex::new(r"//[ \t]+SEQ\[+(.*),(.*)\]")?;
    let messages = regex::Regex::new(r"message[ \t]+(\w+)(Req|Resp|S2C|C2S)")?;
    let module = regex::Regex::new(r"message[ \t]+(\w+).*//module=(\w+).*")?;
    let mut namespaces: Vec<(String, Vec<Message>)> = vec![];
    let mut used = HashMap::<i64, (String, usize)>::new();
    for file in scan::discover(&proto)? {
        let relative = file
            .strip_prefix(&proto)?
            .to_string_lossy()
            .replace('\\', "/");
        if exclude.matches(&relative) {
            continue;
        }
        let text = fs::read_to_string(&file)
            .with_context(|| format!("{}:1: invalid UTF-8", file.display()))?;
        let Some(seq) = sequence.captures(&text) else {
            continue;
        };
        let namespace = package
            .captures(&text)
            .with_context(|| format!("{}:1: missing package", file.display()))?[1]
            .to_owned();
        let min: i64 = seq[1]
            .trim()
            .parse()
            .with_context(|| format!("{}:1: invalid SEQ minimum", file.display()))?;
        let _: i64 = seq[2]
            .trim()
            .parse()
            .with_context(|| format!("{}:1: invalid SEQ maximum", file.display()))?;
        let mut next = if min % 2 == 0 { min + 1 } else { min };
        let mut pairs = HashMap::new();
        // Python dictionaries overwrite duplicate IDs within a file while retaining original insertion order.
        let mut local: Vec<(i64, Message, usize)> = vec![];
        for cap in messages.captures_iter(&text) {
            let key = cap[1].to_owned();
            let suffix = &cap[2];
            let id = match suffix {
                "Req" => {
                    let value = *pairs.entry(key.clone()).or_insert(next);
                    next += 2;
                    value
                }
                "Resp" => *pairs.entry(key.clone()).or_insert(next) + 1,
                _ => {
                    let value = next + 1;
                    next += 2;
                    value
                }
            };
            let entry = Message {
                id,
                name: format!("{namespace}.{key}{suffix}"),
                module: None,
            };
            let line = text[..cap.get(0).unwrap().start()]
                .bytes()
                .filter(|c| *c == b'\n')
                .count()
                + 1;
            if let Some(old) = local.iter_mut().find(|(old, _, _)| *old == id) {
                old.1 = entry;
                old.2 = line;
            } else {
                local.push((id, entry, line));
            }
        }
        for cap in module.captures_iter(&text) {
            let name = format!("{namespace}.{}", &cap[1]);
            if let Some((_, message, _)) = local.iter_mut().find(|(_, m, _)| m.name == name) {
                message.module = Some(cap[2].into());
            } else {
                bail!(
                    "[E_PROTOCOL] {}:1: module annotation references unregistered message {name}",
                    file.display()
                );
            }
        }
        if local.is_empty() {
            continue;
        }
        let pos = if let Some(pos) = namespaces.iter().position(|(name, _)| *name == namespace) {
            pos
        } else {
            namespaces.push((namespace.clone(), vec![]));
            namespaces.len() - 1
        };
        for (id, message, line) in local {
            if let Some((previous, prev_line)) = used.insert(id, (file.display().to_string(), line))
            {
                bail!("[E_PROTOCOL] {}:{line}: duplicate protocol ID {id}, previous={previous}:{prev_line}",file.display());
            }
            namespaces[pos].1.push(message);
        }
    }
    let mut aggregate = serde_json::Map::new();
    let mut outputs = vec![];
    for (namespace, mut entries) in namespaces {
        entries.sort_by_key(|m| m.id);
        let target = if namespace == "cmd" {
            out.join("cmd.json")
        } else {
            out.join(&namespace).join(format!("{namespace}.json"))
        };
        let compact = entries
            .iter()
            .map(|m| Compact {
                id: m.id,
                name: &m.name,
            })
            .collect::<Vec<_>>();
        write_legacy(&target, &compact)?;
        outputs.push(target);
        aggregate.insert(namespace, serde_json::to_value(&entries)?);
    }
    let main = out.join("ProtoConfig.json");
    write_legacy(&main, &aggregate)?;
    outputs.push(main);
    Ok(outputs)
}
fn write_legacy(path: &Path, value: &impl Serialize) -> Result<()> {
    // Python text-mode output on the original Windows platform: CRLF, two spaces, no final newline.
    let text = serde_json::to_string_pretty(value)?.replace('\n', "\r\n");
    util::atomic_write(path, text.as_bytes())
}
pub fn compare(baseline: &Path, actual: &Path) -> Result<serde_json::Value> {
    let baseline_files = util::files(baseline)?;
    let actual_files = util::files(actual)?;
    let map = |root: &Path, files: Vec<PathBuf>| -> BTreeMap<String, PathBuf> {
        files
            .into_iter()
            .map(|p| {
                (
                    p.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                    p,
                )
            })
            .collect()
    };
    let a = map(baseline, baseline_files);
    let b = map(actual, actual_files);
    let mut differences = vec![];
    let mut equal = 0;
    for (name, path) in &a {
        let Some(other) = b.get(name) else {
            differences.push(serde_json::json!({"file":name,"kind":"missing"}));
            continue;
        };
        let left = fs::read(path)?;
        let right = fs::read(other)?;
        if left == right {
            equal += 1;
            continue;
        }
        let offset = left
            .iter()
            .zip(&right)
            .position(|(a, b)| a != b)
            .unwrap_or(left.len().min(right.len()));
        let prefix = &left[..offset.min(left.len())];
        let line = prefix.iter().filter(|c| **c == b'\n').count() + 1;
        let column = prefix
            .iter()
            .rposition(|c| *c == b'\n')
            .map(|p| offset - p)
            .unwrap_or(offset + 1);
        differences.push(serde_json::json!({"file":name,"kind":"changed","byte_offset":offset,"line":line,"column":column,"expected_size":left.len(),"actual_size":right.len(),"expected_hex":hex::encode(&left[offset.saturating_sub(8)..(offset+16).min(left.len())]),"actual_hex":hex::encode(&right[offset.saturating_sub(8).min(right.len())..(offset+16).min(right.len())])}));
    }
    for name in b.keys() {
        if !a.contains_key(name) {
            differences.push(serde_json::json!({"file":name,"kind":"unexpected"}));
        }
    }
    Ok(
        serde_json::json!({"equal_files":equal,"differences":differences,"passed":differences.is_empty()}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn python_bytes_include_crlf_order_and_no_final_newline() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("proto")).unwrap();
        fs::write(dir.path().join("proto/test.proto"),"package cmd;\n// SEQ[100,110]\nmessage LoginReq {} //module=login\nmessage LoginResp {}\n").unwrap();
        generate(dir.path(), &dir.path().join("out"), &Settings::default()).unwrap();
        let bytes = fs::read(dir.path().join("out/cmd.json")).unwrap();
        assert_eq!(bytes,b"[\r\n  {\r\n    \"id\": 101,\r\n    \"name\": \"cmd.LoginReq\"\r\n  },\r\n  {\r\n    \"id\": 102,\r\n    \"name\": \"cmd.LoginResp\"\r\n  }\r\n]");
    }
}
