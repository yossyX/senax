use anyhow::{Context as _, Result};
use fancy_regex::Regex;
use once_cell::sync::Lazy;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    convert::TryInto,
    fs,
    path::Path,
    sync::Mutex,
};

use crate::{schema::BAD_KEYWORDS, DOMAIN_PATH};

pub const DEFAULT_SRID: u32 = 4326;

pub fn hash(v: &str) -> u64 {
    let mut hasher = Sha256::new();
    hasher.update(v);
    let result = hasher.finalize();
    let (int_bytes, _rest) = result.split_at(std::mem::size_of::<u64>());
    u64::from_ne_bytes(int_bytes.try_into().unwrap())
}

pub fn rel_hash(key: String) -> u64 {
    static SET: Lazy<Mutex<HashSet<u64>>> = Lazy::new(|| Mutex::new(HashSet::new()));
    static MAP: Lazy<Mutex<HashMap<String, u64>>> = Lazy::new(|| Mutex::new(HashMap::new()));
    let mut set = SET.lock().unwrap();
    let mut map = MAP.lock().unwrap();
    if let Some(v) = map.get(&key) {
        return *v;
    }
    let mut hash = hash(&key);
    loop {
        if hash < 10 {
            hash = 10;
        }
        if set.contains(&hash) {
            hash = hash.wrapping_add(1);
            continue;
        }
        set.insert(hash);
        map.insert(key, hash);
        return hash;
    }
}

pub fn check_struct_name(name: &str) {
    if ["box", "vec", "option"].iter().any(|&x| x == name) {
        error_exit!("{} is an incorrect name.", name)
    }
}

pub fn check_name(name: &str) -> &str {
    static RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"^\p{XID_Start}\p{XID_Continue}*(?<!_)$").unwrap());
    if !RE.is_match(name).unwrap() || BAD_KEYWORDS.iter().any(|&x| x == name) {
        error_exit!("{} is an incorrect name.", name)
    }
    name
}

pub fn check_column_name(name: &str) -> &str {
    static RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"^\p{XID_Start}\p{XID_Continue}*(?<!_)$").unwrap());
    if !RE.is_match(name).unwrap() || BAD_KEYWORDS.iter().any(|&x| x == name) {
        error_exit!("{} is an incorrect name.", name)
    }
    name
}

pub fn check_ascii_name(name: &str) -> &str {
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[A-Za-z][_0-9A-Za-z]*(?<!_)$").unwrap());
    if !RE.is_match(name).unwrap() || BAD_KEYWORDS.iter().any(|&x| x == name) {
        error_exit!("{} is an incorrect name.", name)
    }
    name
}

macro_rules! if_then_else {
    ( $if:expr, $then:expr, $else:expr ) => {
        if $if {
            $then
        } else {
            $else
        }
    };
}
pub(crate) use if_then_else;

pub fn fs_write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> Result<()> {
    fn inner(path: &Path, contents: &[u8]) -> Result<()> {
        static LAST_PATH: Mutex<String> = Mutex::new(String::new());
        let update = if let Ok(buf) = fs::read(path) {
            !buf.eq(contents)
        } else {
            true
        };
        if update {
            let mut last_path = LAST_PATH.lock().unwrap();
            let path_str = path.display().to_string();
            if !path_str.eq(last_path.as_str()) {
                println!("{}", path_str);
                last_path.clear();
                last_path.push_str(&path_str);
            }
            fs::write(path, contents)?;
        }
        Ok(())
    }
    inner(path.as_ref(), contents.as_ref())
}

pub fn yaml_value_to_str(value: &serde_yaml::Value) -> anyhow::Result<String> {
    match value {
        serde_yaml::Value::Null => Ok("".to_string()),
        serde_yaml::Value::Bool(v) => Ok(format!("{}", v)),
        serde_yaml::Value::Number(v) => Ok(format!("{}", v)),
        serde_yaml::Value::String(v) => Ok(v.to_string()),
        serde_yaml::Value::Sequence(_) => anyhow::bail!("yaml_value_to_str error!"),
        serde_yaml::Value::Mapping(_) => anyhow::bail!("yaml_value_to_str error!"),
    }
}

pub fn simplify_yml(yml: String) -> anyhow::Result<String> {
    let yml: serde_yaml::Value = serde_yaml::from_str(&yml)?;
    let mut buf = String::new();
    output_yml(&mut buf, 0, false, false, yml);
    Ok(buf)
}

fn output_yml(
    buf: &mut String,
    indent: usize,
    mut new_line: bool,
    space: bool,
    yml: serde_yaml::Value,
) {
    match yml {
        serde_yaml::Value::Null => {
            buf.push('\n');
        }
        serde_yaml::Value::Bool(v) => {
            if space {
                buf.push(' ');
            }
            buf.push_str(&v.to_string());
            buf.push('\n');
        }
        serde_yaml::Value::Number(v) => {
            if space {
                buf.push(' ');
            }
            buf.push_str(&v.to_string());
            buf.push('\n');
        }
        serde_yaml::Value::String(v) => {
            let v = v.replace("\r\n", "\n").replace('\r', "\n");
            let v = v.trim();
            if space {
                buf.push(' ');
            }
            if v.contains('\n') {
                buf.push_str("|\n");
                buf.push_str(&"  ".repeat(indent));
                buf.push_str(&v.replace('\n', &format!("\n{}", "  ".repeat(indent))));
            } else if matches!(serde_yaml::from_str(v), Ok(serde_yaml::Value::String(_))) {
                buf.push_str(v);
            } else {
                buf.push_str(&format!("{:?}", v));
            }
            buf.push('\n');
        }
        serde_yaml::Value::Sequence(list) => {
            if list.is_empty() {
                if space {
                    buf.push_str(" []");
                }
                buf.push('\n');
            } else {
                buf.push('\n');
                for row in list {
                    buf.push_str(&"  ".repeat(indent));
                    buf.push_str("- ");
                    output_yml(buf, indent + 1, false, false, row);
                }
            }
        }
        serde_yaml::Value::Mapping(map) => {
            if map.is_empty() {
                if space {
                    buf.push_str(" {}");
                }
                buf.push('\n');
            } else {
                if new_line {
                    buf.push('\n');
                }
                for (key, value) in map {
                    if new_line {
                        buf.push_str(&"  ".repeat(indent));
                    }
                    new_line = true;
                    buf.push_str(key.as_str().unwrap());
                    buf.push(':');
                    output_yml(buf, indent + 1, true, true, value);
                }
            }
        }
    }
}

pub fn to_singular(name: &str) -> String {
    static RE: Lazy<regex::Regex> =
        Lazy::new(|| regex::Regex::new(r"^(.+[^_0-9])([_0-9]+)$").unwrap());
    if let Some(c) = RE.captures(name) {
        format!(
            "{}{}",
            senax_inflector::singularize::to_singular(c.get(1).unwrap().as_str()),
            c.get(2).unwrap().as_str()
        )
    } else {
        senax_inflector::singularize::to_singular(name)
    }
}

pub fn to_plural(name: &str) -> String {
    static RE: Lazy<regex::Regex> =
        Lazy::new(|| regex::Regex::new(r"^(.+[^_0-9])([_0-9]+)$").unwrap());
    if let Some(c) = RE.captures(name) {
        format!(
            "{}{}",
            senax_inflector::pluralize::to_plural(c.get(1).unwrap().as_str()),
            c.get(2).unwrap().as_str()
        )
    } else {
        senax_inflector::pluralize::to_plural(name)
    }
}

pub fn trim_yml_comment(v: &str) -> String {
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^(#.*|---)$").unwrap());
    RE.replace_all(v, "").trim().to_string()
}

pub fn parse_yml<T: DeserializeOwned + Default>(content: &str) -> Result<T> {
    if !trim_yml_comment(content).is_empty() {
        Ok(serde_yaml::from_str(content)
            .map_err(|err| format_serde_error::SerdeError::new(content.to_string(), err))?)
    } else {
        Ok(T::default())
    }
}

pub fn parse_yml_file<T: DeserializeOwned + Default>(path: &Path) -> Result<T> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Cannot read file: {:?}", path))?;
    if !trim_yml_comment(&content).is_empty() {
        Ok(serde_yaml::from_str(&content)
            .map_err(|err| format_serde_error::SerdeError::new(content.to_string(), err))?)
    } else {
        Ok(T::default())
    }
}

pub fn check_non_snake_case() -> Result<bool> {
    let file_path = Path::new(DOMAIN_PATH).join("src").join("lib.rs");
    if file_path.exists() {
        let content = fs::read_to_string(&file_path)?;
        Ok(content.contains("#![allow(non_snake_case)]"))
    } else {
        Ok(false)
    }
}

pub fn check_js(script: &str) -> anyhow::Result<()> {
    use rquickjs::{Context, Error::Exception, Runtime};
    let rt = Runtime::new()?;
    let ctx = Context::full(&rt)?;
    ctx.enable_big_num_ext(true);
    ctx.with(|ctx| match ctx.eval::<(), _>(script) {
        Ok(_) => Ok(()),
        Err(Exception) => anyhow::bail!("js_update error::{:?}", ctx.catch()),
        Err(e) => anyhow::bail!("js_update error::{:?}", e),
    })
}
