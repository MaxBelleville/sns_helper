use std::collections::BTreeMap;

pub fn check_f32_values(issues: &mut Vec<String>, values: &Vec<f32>, msg: String) {
    if values.into_iter().any(|value| !value.is_finite()) {
        issues.push(msg);
    }
}

pub fn check_f32_opt(
    issues: &mut Vec<String>,
    values: &Vec<f32>,
    opt_cond: Option<bool>,
    msg: String,
) {
    let values_invalid = values.into_iter().any(|value| !value.is_finite());

    if values_invalid || opt_cond == Some(true) {
        issues.push(msg);
    }
}

pub fn check_vec<T>(
    issues: &mut Vec<String>,
    values: &Vec<T>,
    is_invalid: impl Fn(&T) -> bool,
    msg: String,
) {
    let values_invalid = values.iter().any(|value| is_invalid(value));
    if values_invalid {
        issues.push(msg);
    }
}

pub fn check_vec_opt<T>(
    issues: &mut Vec<String>,
    values: &Vec<T>,
    is_invalid: impl Fn(&T) -> bool,
    opt_cond: Option<bool>,
    msg: String,
) {
    let values_invalid = values.iter().any(|value| is_invalid(value));
    if values_invalid || opt_cond == Some(true) {
        issues.push(msg);
    }
}

pub fn check_vec_msg<T>(
    issues: &mut Vec<String>,
    values: &Vec<(String, T)>,
    is_invalid: impl Fn(&T) -> bool,
) {
    for (msg, val) in values {
        if is_invalid(val) {
            issues.push(msg.clone());
        }
    }
}

pub fn check_btree<K, V>(
    issues: &mut Vec<String>,
    values: &BTreeMap<K, V>,
    is_invalid: impl Fn(&K, &V) -> bool,
    msg: String,
) {
    let values_invalid = values.iter().any(|(key, val)| is_invalid(key, val));
    if values_invalid {
        issues.push(msg);
    }
}
