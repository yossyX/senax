use anyhow::{bail, ensure, Context as _, Result};
use chrono::{DateTime, Utc};
use serde_json::{Number, Value};
use std::cmp::Ordering;

pub fn apply_operator(mut obj: Value, op: &Value, now: DateTime<Utc>) -> Result<Value> {
    let op = op.as_object().context("Illegal Operator")?;
    for (typ, target) in op {
        match typ.as_str() {
            "_currentDate" => {
                let target = target.as_object().context("Incorrect Modifier")?;
                for (field, _) in target {
                    let fields: Vec<_> = field.split('.').collect();
                    obj = update_field(&obj, &fields, |_| Ok(Some(Value::from(now.to_rfc3339()))))?;
                }
            }
            "_inc" => {
                let target = target.as_object().context("Incorrect Modifier")?;
                for (field, inc) in target {
                    ensure!(inc.is_number(), "The value for _inc must be a number.");
                    let fields: Vec<_> = field.split('.').collect();
                    obj = update_field(&obj, &fields, |org| match org {
                        Value::Null => Ok(Some(inc.to_owned())),
                        Value::Number(v) => Ok(Some(fn_inc(v, inc))),
                        _ => bail!(
                            "Cannot apply _inc to a value of non-numeric type.:{}",
                            field
                        ),
                    })?;
                }
            }
            "_min" => {
                let target = target.as_object().context("Incorrect Modifier")?;
                for (field, val) in target {
                    ensure!(val.is_number(), "The value for _min must be a number.");
                    let fields: Vec<_> = field.split('.').collect();
                    obj = update_field(&obj, &fields, |org| match org {
                        Value::Null => Ok(Some(val.to_owned())),
                        Value::Number(v) => Ok(Some(fn_min(v, val))),
                        _ => bail!(
                            "Cannot apply _min to a value of non-numeric type.:{}",
                            field
                        ),
                    })?;
                }
            }
            "_max" => {
                let target = target.as_object().context("Incorrect Modifier")?;
                for (field, val) in target {
                    ensure!(val.is_number(), "The value for _max must be a number.");
                    let fields: Vec<_> = field.split('.').collect();
                    obj = update_field(&obj, &fields, |org| match org {
                        Value::Null => Ok(Some(val.to_owned())),
                        Value::Number(v) => Ok(Some(fn_max(v, val))),
                        _ => bail!(
                            "Cannot apply _max to a value of non-numeric type.:{}",
                            field
                        ),
                    })?;
                }
            }
            "_mul" => {
                let target = target.as_object().context("Incorrect Modifier")?;
                for (field, val) in target {
                    ensure!(val.is_number(), "The value for _mul must be a number.");
                    let fields: Vec<_> = field.split('.').collect();
                    obj = update_field(&obj, &fields, |org| match org {
                        Value::Null => Ok(Some(Value::Null)),
                        Value::Number(v) => Ok(Some(fn_mul(v, val))),
                        _ => bail!(
                            "Cannot apply _mul to a value of non-numeric type.:{}",
                            field
                        ),
                    })?;
                }
            }
            "_rename" => {
                let target = target.as_object().context("Incorrect Modifier")?;
                for (field, val) in target {
                    ensure!(val.is_string(), "The value for _rename must be a string.");
                    let fields: Vec<_> = field.split('.').collect();
                    let mut buf = None;
                    obj = update_field(&obj, &fields, |org| {
                        buf = Some(org.clone());
                        Ok(None)
                    })?;
                    let fields: Vec<_> = val.as_str().unwrap().split('.').collect();
                    obj = update_field(&obj, &fields, |_| Ok(Some(buf.take().unwrap())))?;
                }
            }
            "_replace" => {
                target.as_object().context("Incorrect Modifier")?;
                obj = target.clone();
            }
            "_set" => {
                let target = target.as_object().context("Incorrect Modifier")?;
                for (field, val) in target {
                    let fields: Vec<_> = field.split('.').collect();
                    obj = update_field(&obj, &fields, |_| Ok(Some(val.clone())))?;
                }
            }
            "_unset" => {
                let target = target.as_object().context("Incorrect Modifier")?;
                for (field, _) in target {
                    let fields: Vec<_> = field.split('.').collect();
                    obj = update_field(&obj, &fields, |_| Ok(None))?;
                }
            }
            "_addToSet" => {
                let target = target.as_object().context("Incorrect Modifier")?;
                for (field, val) in target {
                    let fields: Vec<_> = field.split('.').collect();
                    obj = update_field(&obj, &fields, |org| {
                        let mut org_list = org
                            .as_array()
                            .context("Cannot apply _addToSet to a non-array value")?
                            .clone();
                        if let Some(obj) = val.as_object() {
                            if let Some(each) = obj.get("_each") {
                                if let Some(list) = each.as_array() {
                                    for val in list {
                                        if !org_list.iter().any(|o| o == val) {
                                            org_list.push(val.clone());
                                        }
                                    }
                                    return Ok(Some(Value::from(org_list)));
                                }
                            }
                        }
                        if !org_list.iter().any(|o| o == val) {
                            org_list.push(val.clone());
                        }
                        Ok(Some(Value::from(org_list)))
                    })?;
                }
            }
            "_pop" => {
                let target = target.as_object().context("Incorrect Modifier")?;
                for (field, val) in target {
                    ensure!(val.is_i64(), "_pop requires an integer argument.");
                    let fields: Vec<_> = field.split('.').collect();
                    obj = update_field(&obj, &fields, |org| {
                        let mut org_list = org
                            .as_array()
                            .context("Cannot apply _pop to a non-array value")?
                            .clone();
                        if val.as_i64().unwrap() > 0 {
                            org_list.truncate(org_list.len() - 1);
                        } else {
                            org_list.remove(0);
                        }
                        Ok(Some(Value::from(org_list)))
                    })?;
                }
            }
            "_push" => {
                let target = target.as_object().context("Incorrect Modifier")?;
                for (field, val) in target {
                    let fields: Vec<_> = field.split('.').collect();
                    obj = update_field(&obj, &fields, |org| {
                        let mut org_list = org
                            .as_array()
                            .context("Cannot apply _push to a non-array value")?
                            .clone();
                        if let Some(obj) = val.as_object() {
                            if let Some(each) = obj.get("_each") {
                                if let Some(list) = each.as_array() {
                                    if let Some(pos) = obj.get("_position") {
                                        let pos = pos.as_u64().context(
                                            "The value for _position must be an integer value.",
                                        )?;
                                        for val in list {
                                            org_list.insert(pos as usize, val.clone())
                                        }
                                    } else {
                                        for val in list {
                                            org_list.push(val.clone())
                                        }
                                    }
                                    // _sort is not supported
                                    if let Some(slice) = obj.get("_slice") {
                                        if let Some(slice) = slice.as_u64() {
                                            org_list.truncate(slice as usize);
                                        }
                                    }
                                    return Ok(Some(Value::from(org_list)));
                                }
                            }
                        }
                        org_list.push(val.clone());
                        Ok(Some(Value::from(org_list)))
                    })?;
                }
            }
            "_pullAll" => {
                let target = target.as_object().context("Incorrect Modifier")?;
                for (field, val) in target {
                    let val_list = val
                        .as_array()
                        .context("_pullAll requires an array argument.")?;
                    let fields: Vec<_> = field.split('.').collect();
                    obj = update_field(&obj, &fields, |org| {
                        let mut org_list = org
                            .as_array()
                            .context("Cannot apply _pullAll to a non-array value")?
                            .clone();
                        org_list.retain(|o| val_list.iter().all(|val| o != val));
                        Ok(Some(Value::from(org_list)))
                    })?;
                }
            }
            "_bit" => {
                let target = target.as_object().context("Incorrect Modifier")?;
                for (field, val) in target {
                    let fields: Vec<_> = field.split('.').collect();
                    obj = update_field(&obj, &fields, |org| {
                        let mut org = match org {
                            Value::Null => 0,
                            Value::Number(v) => v
                                .as_u64()
                                .context("Cannot apply _bit to a value of non-integral type.")?,
                            _ => bail!("Cannot apply _bit to a value of non-integral type."),
                        };
                        if let Some(val) = val.as_object() {
                            if let Some(and) = val.get("and") {
                                ensure!(and.is_u64(), "The value for _bit must be a integer.");
                                org &= and.as_u64().unwrap();
                            }
                            if let Some(or) = val.get("or") {
                                ensure!(or.is_u64(), "The value for _bit must be a integer.");
                                org |= or.as_u64().unwrap();
                            }
                            if let Some(xor) = val.get("xor") {
                                ensure!(xor.is_u64(), "The value for _bit must be a integer.");
                                org ^= xor.as_u64().unwrap();
                            }
                        }
                        Ok(Some(Value::from(org)))
                    })?;
                }
            }
            _ => bail!("Unsupported Operator:{}", typ),
        }
    }
    Ok(obj)
}

fn fn_inc(a: &Number, b: &Value) -> Value {
    let f64_value = a.as_f64().unwrap() + b.as_f64().unwrap();
    if a.is_f64() || b.is_f64() {
        Value::from(f64_value)
    } else if a.is_u64() && b.is_u64() {
        if let Some(v) = a.as_u64().unwrap().checked_add(b.as_u64().unwrap()) {
            Value::from(v)
        } else {
            Value::from(f64_value)
        }
    } else if let Some(v) = a.as_i64().unwrap().checked_add(b.as_i64().unwrap()) {
        Value::from(v)
    } else {
        Value::from(f64_value)
    }
}

fn fn_min(a: &Number, b: &Value) -> Value {
    if a.is_f64() || b.is_f64() {
        let a = a.as_f64().unwrap();
        let b = b.as_f64().unwrap();
        Value::from(if a.total_cmp(&b) == Ordering::Greater {
            b
        } else {
            a
        })
    } else if a.is_u64() && b.is_u64() {
        Value::from(std::cmp::min(a.as_u64().unwrap(), b.as_u64().unwrap()))
    } else if a.is_i64() && b.is_u64() {
        Value::from(a.as_i64().unwrap())
    } else if a.is_u64() && b.is_i64() {
        Value::from(b.as_i64().unwrap())
    } else {
        Value::from(std::cmp::min(a.as_i64().unwrap(), b.as_i64().unwrap()))
    }
}

fn fn_max(a: &Number, b: &Value) -> Value {
    if a.is_f64() || b.is_f64() {
        let a = a.as_f64().unwrap();
        let b = b.as_f64().unwrap();
        Value::from(if a.total_cmp(&b) == Ordering::Less {
            b
        } else {
            a
        })
    } else if a.is_u64() && b.is_u64() {
        Value::from(std::cmp::max(a.as_u64().unwrap(), b.as_u64().unwrap()))
    } else if a.is_u64() {
        Value::from(a.as_u64().unwrap())
    } else if b.is_u64() {
        Value::from(b.as_u64().unwrap())
    } else {
        Value::from(std::cmp::max(a.as_i64().unwrap(), b.as_i64().unwrap()))
    }
}

fn fn_mul(a: &Number, b: &Value) -> Value {
    let f64_value = a.as_f64().unwrap() * b.as_f64().unwrap();
    if a.is_f64() || b.is_f64() {
        Value::from(f64_value)
    } else if a.is_u64() && b.is_u64() {
        if let Some(v) = a.as_u64().unwrap().checked_mul(b.as_u64().unwrap()) {
            Value::from(v)
        } else {
            Value::from(f64_value)
        }
    } else if let Some(v) = a.as_i64().unwrap().checked_mul(b.as_i64().unwrap()) {
        Value::from(v)
    } else {
        Value::from(f64_value)
    }
}

fn update_field<F>(obj: &Value, fields: &[&str], mut update_func: F) -> Result<Value>
where
    F: FnMut(&Value) -> Result<Option<Value>>,
{
    ensure!(!fields.is_empty() && !fields[0].is_empty(), "Illegal Field");
    let field = fields[0];
    if fields.len() == 1 {
        match obj {
            Value::Array(array) => {
                let pos: usize = field.parse()?;
                let val = array
                    .get(pos)
                    .with_context(|| format!("Incorrect array position: {}", pos))?;
                let mut new_array = array.clone();
                if let Some(new) = update_func(val)? {
                    new_array.push(new);
                    new_array.swap_remove(pos);
                } else {
                    new_array.remove(pos);
                }
                Ok(Value::from(new_array))
            }
            Value::Object(obj) => {
                let val = obj.get(field).unwrap_or(&Value::Null);
                let mut new_obj = obj.clone();
                if let Some(new) = update_func(val)? {
                    new_obj.insert(field.to_string(), new);
                } else {
                    new_obj.remove(field);
                }
                Ok(Value::from(new_obj))
            }
            _ => bail!("Illegal Target Value:{}", obj),
        }
    } else {
        match obj {
            Value::Array(array) => {
                let pos: usize = field.parse()?;
                let val = array
                    .get(pos)
                    .with_context(|| format!("Incorrect array position: {}", pos))?;
                let new = update_field(val, &fields[1..], update_func)?;
                let mut new_array = array.clone();
                new_array.push(new);
                new_array.swap_remove(pos);
                Ok(Value::from(new_array))
            }
            Value::Object(obj) => {
                let val = obj.get(field).unwrap_or(&Value::Null);
                let new = update_field(val, &fields[1..], update_func)?;
                let mut new_obj = obj.clone();
                new_obj.insert(field.to_string(), new);
                Ok(Value::from(new_obj))
            }
            _ => bail!("Illegal Target Value:{}", obj),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_apply_operator() -> Result<()> {
        let now = DateTime::default();
        assert_eq!(
            apply_operator(
                json!({"a":{"b":[1, 1]}}),
                &json!({"_currentDate":{"a.b.1":true}}),
                now
            )?,
            json!({"a":{"b":[1, "1970-01-01T00:00:00+00:00"]}})
        );
        assert_eq!(
            apply_operator(
                json!({"a":{"b":1, "c":1.5, "d":-1, "e":10}}),
                &json!({"_inc":{"a.b":3, "a.c":2, "a.d":-2, "a.e":-2}}),
                now
            )?,
            json!({"a":{"b":4, "c":3.5, "d":-3, "e":8}})
        );
        assert_eq!(
            apply_operator(
                json!({"a":[3.1, 3, 3, -3, -3]}),
                &json!({"_min":{"a.0":2.0, "a.1":4, "a.2":-4, "a.3":4, "a.4":-4}}),
                now
            )?,
            json!({"a":[2.0, 3, -4, -3, -4]})
        );
        assert_eq!(
            apply_operator(
                json!({"a":[3.1, 3, 3, -3, -3]}),
                &json!({"_max":{"a.0":2.0, "a.1":4, "a.2":-4, "a.3":4, "a.4":-4}}),
                now
            )?,
            json!({"a":[3.1, 4, 3, 4, -3]})
        );
        assert_eq!(
            apply_operator(
                json!({"a":{"b":1, "c":1.5, "d":-1, "e":10}}),
                &json!({"_mul":{"a.b":3, "a.c":2, "a.d":-2, "a.e":-2}}),
                now
            )?,
            json!({"a":{"b":3, "c":3.0, "d":2, "e":-20}})
        );
        assert_eq!(
            apply_operator(
                json!({"a":{"b":[3], "c":"cc"}}),
                &json!({"_rename":{"a.b":"a.d", "a.c":"c"}}),
                now
            )?,
            json!({"a":{"d":[3]}, "c":"cc"})
        );
        assert_eq!(
            apply_operator(json!({"a":1, "b":1}), &json!({"_replace":{"a":2}}), now)?,
            json!({"a":2})
        );
        assert_eq!(
            apply_operator(json!({"a":{}}), &json!({"_set":{"a.b":"bb"}}), now)?,
            json!({"a":{"b":"bb"}})
        );
        assert_eq!(
            apply_operator(json!({"a":{"b":"bb"}}), &json!({"_unset":{"a.b":""}}), now)?,
            json!({"a":{}})
        );
        assert_eq!(
            apply_operator(
                json!({"a":{"b":["bb"]}}),
                &json!({"_addToSet":{"a.b":"cc"}}),
                now
            )?,
            json!({"a":{"b":["bb", "cc"]}})
        );
        assert_eq!(
            apply_operator(
                json!({"a":{"b":["bb"]}}),
                &json!({"_addToSet":{"a.b":"bb"}}),
                now
            )?,
            json!({"a":{"b":["bb"]}})
        );
        assert_eq!(
            apply_operator(
                json!({"a":{"b":["bb"]}}),
                &json!({"_addToSet":{"a.b":{"_each": ["cc", "dd", "bb"]}}}),
                now
            )?,
            json!({"a":{"b":["bb", "cc", "dd"]}})
        );
        assert_eq!(
            apply_operator(
                json!({"a":{"b":["1", "2", "3"], "c":["1", "2", "3"]}}),
                &json!({"_pop":{"a.b":1, "a.c":-1}}),
                now
            )?,
            json!({"a":{"b":["1", "2"], "c":["2", "3"]}})
        );
        assert_eq!(
            apply_operator(
                json!({"a":{"b":["1", "2", "3"], "c":["1", "2", "3"]}}),
                &json!({"_push":{"a.b":{"a": 1}, "a.c":{"_each":["4", "5"], "_position": 0, "_slice":4}}}),
                now
            )?,
            json!({"a":{"b":["1", "2", "3", {"a": 1}], "c":["5", "4", "1", "2"]}})
        );
        assert_eq!(
            apply_operator(
                json!({"a":{"b":["1", "2", "3", "4", "5"]}}),
                &json!({"_pullAll":{"a.b":["2", "4"]}}),
                now
            )?,
            json!({"a":{"b":["1", "3", "5"]}})
        );
        assert_eq!(
            apply_operator(
                json!({"a":{"b":3, "c":3, "d":3}}),
                &json!({"_bit":{"a.b":{"and": 2}, "a.c":{"or": 4}, "a.d":{"xor": 1}}}),
                now
            )?,
            json!({"a":{"b":2, "c":7, "d":2}})
        );
        Ok(())
    }
}
