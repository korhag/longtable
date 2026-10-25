// ------------------------------------------------------------
// Module: fill
// Purpose: Put a character's numbers into an action formula.
//          A negative stat replaces the plus in front of it,
//          so the parser never sees "+-".
// ------------------------------------------------------------

use std::collections::BTreeMap;

pub fn fill_formula(formula: &str, numbers: &BTreeMap<String, i32>) -> Result<String, String> {
    let chars: Vec<char> = formula.chars().collect();
    let mut out = String::new();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] == '@' {
            let start = index + 1;
            let mut end = start;
            while end < chars.len() && (chars[end].is_ascii_alphanumeric() || chars[end] == '_') {
                end += 1;
            }
            if end == start {
                return Err("a stat reference needs a name".into());
            }
            let name: String = chars[start..end].iter().collect();
            let value = *numbers.get(&name).ok_or_else(|| format!("this character has no {name}"))?;
            let previous = out.chars().last();
            let combined = if previous == Some('+') || previous == Some('-') {
                out.pop();
                let sign = if previous == Some('-') { -1 } else { 1 };
                sign * value
            } else {
                value
            };
            if combined < 0 {
                out.push_str(&combined.to_string());
            } else {
                out.push('+');
                out.push_str(&combined.to_string());
            }
            index = end;
        } else {
            out.push(chars[index]);
            index += 1;
        }
    }
    Ok(out)
}

pub fn apply_modifiers(formula: &str, plus: i32, minus: i32) -> String {
    let mut text = formula.trim().to_string();
    let plus = plus.clamp(0, 1000);
    let minus = minus.clamp(0, 1000);
    if plus > 0 {
        text.push_str(&format!("+{plus}"));
    }
    if minus > 0 {
        text.push_str(&format!("-{minus}"));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_negative_stat_does_not_produce_a_double_sign() {
        let mut numbers = BTreeMap::new();
        numbers.insert("str".into(), -1);
        assert_eq!(fill_formula("1d20+@str", &numbers).unwrap(), "1d20-1");
        numbers.insert("str".into(), 3);
        assert_eq!(fill_formula("1d20+@str", &numbers).unwrap(), "1d20+3");
        assert!(!fill_formula("1d20+@str", &numbers).unwrap().contains("+-"));
    }
}
