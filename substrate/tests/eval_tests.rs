//! Pruebas para Eval (ejecución de código en sandbox)

use substrate::eval::{Eval, EvalError};

#[test]
fn test_eval_python() {
    let result = Eval::eval("python", "print('hello world')", 5);
    assert!(result.is_ok());
    assert!(result.unwrap().contains("hello world"));
}

#[test]
fn test_eval_timeout() {
    let result = Eval::eval("python", "import time; time.sleep(10)", 1);
    assert!(matches!(result, Err(EvalError::Timeout)));
}

#[test]
fn test_eval_invalid_language() {
    let result = Eval::eval("invalid", "print('hello')", 5);
    assert!(matches!(result, Err(EvalError::LanguageNotSupported)));
}

#[test]
fn test_eval_dangerous_code_rm() {
    let result = Eval::eval("bash", "rm -rf /tmp", 5);
    assert!(matches!(result, Err(EvalError::InvalidCode)));
}
