//! 宽容的反序列化器。
//!
//! Emby 各版本、各刮削插件返回的类型并不一致：同一个字段在 A 服务器上是
//! 字符串，在 B 服务器上就是数字。典型的是 `ProviderIds`——TMDB / TVDB 的
//! id 经常直接是整数：
//!
//! ```json
//! "ProviderIds": { "Tmdb": 757, "Imdb": "tt0120737" }
//! ```
//!
//! 严格按 String 解析会整条记录失败，导致「详情页打不开」。这里统一把
//! 标量转成字符串，宁可宽松也不要让一部片子整个加载不出来。

use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::collections::HashMap;

fn scalar_to_string(v: Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        // null / 数组 / 对象都当作「没有值」
        _ => None,
    }
}

/// 接受字符串或数字的必填字段，缺失时为空串
pub fn flex_string<'de, D>(d: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Option::<Value>::deserialize(d)?;
    Ok(v.and_then(scalar_to_string).unwrap_or_default())
}

/// 接受字符串或数字的可选字段
pub fn flex_opt_string<'de, D>(d: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Option::<Value>::deserialize(d)?;
    Ok(v.and_then(scalar_to_string))
}

/// 值可能是字符串也可能是数字的字典
pub fn flex_string_map<'de, D>(d: D) -> Result<HashMap<String, String>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = Option::<HashMap<String, Value>>::deserialize(d)?.unwrap_or_default();
    Ok(raw
        .into_iter()
        .filter_map(|(k, v)| scalar_to_string(v).map(|s| (k, s)))
        .collect())
}

/// 元素可能混着字符串和数字的数组
pub fn flex_string_vec<'de, D>(d: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = Option::<Vec<Value>>::deserialize(d)?.unwrap_or_default();
    Ok(raw.into_iter().filter_map(scalar_to_string).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct Probe {
        #[serde(deserialize_with = "flex_string_map", default)]
        provider_ids: HashMap<String, String>,
        #[serde(deserialize_with = "flex_opt_string", default)]
        id: Option<String>,
        #[serde(deserialize_with = "flex_string", default)]
        name: String,
        #[serde(deserialize_with = "flex_string_vec", default)]
        tags: Vec<String>,
    }

    #[test]
    fn accepts_numeric_provider_ids() {
        let json = r#"{
            "provider_ids": { "Tmdb": 757, "Imdb": "tt0120737", "Missing": null },
            "id": 12345,
            "name": 9,
            "tags": ["a", 7, null]
        }"#;
        let p: Probe = serde_json::from_str(json).expect("应当解析成功");

        assert_eq!(p.provider_ids.get("Tmdb").map(String::as_str), Some("757"));
        assert_eq!(p.provider_ids.get("Imdb").map(String::as_str), Some("tt0120737"));
        assert!(!p.provider_ids.contains_key("Missing"));
        assert_eq!(p.id.as_deref(), Some("12345"));
        assert_eq!(p.name, "9");
        assert_eq!(p.tags, vec!["a".to_string(), "7".to_string()]);
    }

    #[test]
    fn missing_and_null_fields_are_defaults() {
        let p: Probe = serde_json::from_str(r#"{ "id": null }"#).expect("应当解析成功");
        assert!(p.provider_ids.is_empty());
        assert_eq!(p.id, None);
        assert_eq!(p.name, "");
        assert!(p.tags.is_empty());
    }
}
