use evalexpr::{
    ContextWithMutableFunctions, ContextWithMutableVariables, DefaultNumericTypes,
    EvalexprError, Function, HashMapContext, Value, eval_with_context,
};

use super::{SearchAction, SearchProvider, SearchResult};

fn to_f64(val: &Value<DefaultNumericTypes>) -> Result<f64, EvalexprError> {
    match val {
        Value::Float(f) => Ok(*f),
        Value::Int(i) => Ok(*i as f64),
        _ => Err(EvalexprError::expected_number(val.clone())),
    }
}

pub struct CalcProvider {
    context: HashMapContext<DefaultNumericTypes>,
}

impl Default for CalcProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl CalcProvider {
    pub fn new() -> Self {
        let mut context: HashMapContext<DefaultNumericTypes> = HashMapContext::new();

        let _ = context.set_value("pi".into(), Value::from_float(std::f64::consts::PI));
        let _ = context.set_value("PI".into(), Value::from_float(std::f64::consts::PI));
        let _ = context.set_value("e".into(), Value::from_float(std::f64::consts::E));
        let _ = context.set_value("E".into(), Value::from_float(std::f64::consts::E));

        let _ = context.set_function(
            "sqrt".into(),
            Function::new(|arg| {
                let n = to_f64(arg)?;
                if n < 0.0 {
                    Err(EvalexprError::CustomMessage("square root of negative number".into()))
                } else {
                    Ok(Value::Float(n.sqrt()))
                }
            }),
        );

        let _ = context.set_function(
            "abs".into(),
            Function::new(|arg| match arg {
                Value::Int(i) => {
                    let val: i64 = *i;
                    Ok(Value::Int(val.abs()))
                }
                Value::Float(f) => {
                    let val: f64 = *f;
                    Ok(Value::Float(val.abs()))
                }
                _ => Err(EvalexprError::expected_number(arg.clone())),
            }),
        );

        let _ = context.set_function(
            "sin".into(),
            Function::new(|arg| Ok(Value::Float(to_f64(arg)?.sin()))),
        );

        let _ = context.set_function(
            "cos".into(),
            Function::new(|arg| Ok(Value::Float(to_f64(arg)?.cos()))),
        );

        let _ = context.set_function(
            "tan".into(),
            Function::new(|arg| Ok(Value::Float(to_f64(arg)?.tan()))),
        );

        let _ = context.set_function(
            "round".into(),
            Function::new(|arg| Ok(Value::Float(to_f64(arg)?.round()))),
        );

        let _ = context.set_function(
            "floor".into(),
            Function::new(|arg| Ok(Value::Float(to_f64(arg)?.floor()))),
        );

        let _ = context.set_function(
            "ceil".into(),
            Function::new(|arg| Ok(Value::Float(to_f64(arg)?.ceil()))),
        );

        let _ = context.set_function(
            "log".into(),
            Function::new(|arg| Ok(Value::Float(to_f64(arg)?.log10()))),
        );

        let _ = context.set_function(
            "ln".into(),
            Function::new(|arg| Ok(Value::Float(to_f64(arg)?.ln()))),
        );

        let _ = context.set_function(
            "pow".into(),
            Function::new(|arg| {
                let tuple = arg.as_fixed_len_tuple(2)?;
                let base = to_f64(&tuple[0])?;
                let exp = to_f64(&tuple[1])?;
                Ok(Value::Float(base.powf(exp)))
            }),
        );

        Self { context }
    }

    fn is_potential_math(query: &str) -> bool {
        let q = query.trim();
        if q.is_empty() {
            return false;
        }

        let has_math_op = q
            .chars()
            .any(|c| matches!(c, '+' | '-' | '*' | '/' | '%' | '^' | '(' | ')'));

        let has_known_func = q.starts_with("sqrt(")
            || q.starts_with("sin(")
            || q.starts_with("cos(")
            || q.starts_with("tan(")
            || q.starts_with("abs(")
            || q.starts_with("log(")
            || q.starts_with("ln(")
            || q.starts_with("pow(");

        has_math_op || has_known_func
    }

    fn format_value(value: &Value<DefaultNumericTypes>) -> Option<String> {
        match value {
            Value::Int(i) => Some(format!("{i}")),
            Value::Float(f) => {
                if f.is_nan() || f.is_infinite() {
                    None
                } else if f.fract() == 0.0 && f.abs() < 1e15 {
                    Some(format!("{:.0}", f))
                } else {
                    let s = format!("{:.8}", f);
                    let trimmed = s.trim_end_matches('0').trim_end_matches('.');
                    Some(trimmed.to_string())
                }
            }
            _ => None,
        }
    }
}

impl SearchProvider for CalcProvider {
    fn id(&self) -> &'static str {
        "calc"
    }

    fn priority_boost(&self) -> i64 {
        1000
    }

    fn matches(&self, query: &str) -> Vec<SearchResult> {
        if !Self::is_potential_math(query) {
            return Vec::new();
        }

        let eval_result = eval_with_context(query, &self.context);
        let Ok(val) = eval_result else {
            return Vec::new();
        };

        let Some(formatted) = Self::format_value(&val) else {
            return Vec::new();
        };

        vec![SearchResult {
            id: format!("calc:{}", query.trim()),
            title: formatted.clone(),
            subtitle: Some(format!("= {} • Enter to copy", query.trim())),
            icon: Some("accessories-calculator".to_string()),
            score: 1000,
            action: SearchAction::CopyToClipboard {
                text: formatted,
            },
            secondary_actions: Vec::new(),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calc_basic_operations() {
        let provider = CalcProvider::new();

        let results = provider.matches("23*4");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "92");

        let results = provider.matches("100 / 4 + 5");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "30");

        let results = provider.matches("2 ^ 8");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "256");
    }

    #[test]
    fn test_calc_functions() {
        let provider = CalcProvider::new();

        let results = provider.matches("sqrt(144)");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "12");

        let results = provider.matches("sin(0)");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "0");

        let results = provider.matches("abs(-42)");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "42");
    }

    #[test]
    fn test_calc_ignores_normal_text() {
        let provider = CalcProvider::new();

        assert!(provider.matches("firefox").is_empty());
        assert!(provider.matches("calc").is_empty());
        assert!(provider.matches("rusted-light").is_empty());
        assert!(provider.matches("42").is_empty());
        assert!(provider.matches("").is_empty());
    }
}
