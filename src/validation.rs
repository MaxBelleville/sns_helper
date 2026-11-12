use std::collections::BTreeMap;

pub fn check_f32_values(issues: &mut Vec<String>, values: impl Iterator<Item = f32>, msg: String) {
    let mut check_pass = true;
    for value in values {
        if !value.is_finite() {
            check_pass = false;
        }
    }
    if !check_pass {
        issues.push(msg);
    }
}

pub fn check_f32_opt(
    issues: &mut Vec<String>,
    values: impl Iterator<Item = f32>,
    opt_cond: Option<bool>,
    msg: String,
) {
    let values_invalid = values.into_iter().any(|value| !value.is_finite());

    if values_invalid || opt_cond == Some(true) {
        issues.push(msg);
    }
}

pub fn check_f32_invalid(
    issues: &mut Vec<String>,
    values: impl Iterator<Item = f32>,
    is_invalid: impl Fn(f32) -> bool,
    msg: String,
) {
    let values_invalid = values
        .into_iter()
        .any(|value| !value.is_finite() || is_invalid(value));

    if values_invalid {
        issues.push(msg);
    }
}

pub fn check_invalid(
    issues: &mut Vec<String>,
    fields: impl Iterator<Item = (f32, String)>,
    is_invalid: impl Fn(f32) -> bool,
) {
    for (value, msg) in fields {
        if !value.is_finite() || is_invalid(value) {
            issues.push(msg);
        }
    }
}

pub fn check_f32(
    issues: &mut Vec<String>,
    values: impl IntoIterator<Item = f32>,
    is_invalid: impl Fn(f32) -> bool,
    opt_cond: Option<bool>,
    msg: String,
) {
    let values_invalid = values
        .into_iter()
        .any(|value| !value.is_finite() || is_invalid(value));

    if values_invalid || opt_cond == Some(true) {
        issues.push(msg);
    }
}

pub fn check_vec<T>(
    issues: &mut Vec<String>,
    vals: &Vec<T>,
    contains_key: impl Fn(usize, &T) -> bool,
    msg: String,
) {
    for (idx, val) in vals.iter().enumerate() {
        if contains_key(idx, val) {
            issues.push(msg.clone());
        }
    }
}

pub fn check_btree<K, V>(
    issues: &mut Vec<String>,
    vals: &BTreeMap<K, V>,
    contains_key: impl Fn(&K, &V) -> bool,
    msg: String,
) {
    for (key, val) in vals {
        if contains_key(key, val) {
            issues.push(msg.clone());
        }
    }
}
