pub fn check_values(
    issues: &mut Vec<String>,
    values: impl Iterator<Item = f32>,
    opt_cond: Option<bool>,
    msg: String,
) {
    let mut check_pass = true;
    for value in values {
        if !value.is_finite() {
            check_pass = false;
        }
    }
    if let Some(cond) = opt_cond {
        if cond {
            check_pass = false;
        }
    }
    if check_pass {
        issues.push(msg);
    }
}
