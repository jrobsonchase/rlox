use crate::*;

use std::mem::replace;

#[derive(Default)]
pub struct Interpreter {
    environment: Environment,
}

impl Interpreter {
    pub fn evaluate(&mut self, expr: &Expr) -> Result<Value, LoxError> {
        self.visit(expr)
    }

    pub fn execute(&mut self, stmt: &Stmt) -> Result<(), LoxError> {
        self.visit(stmt)
    }
}

impl<'a> Visitor<&'a Expr> for Interpreter {
    type Output = Result<Value, LoxError>;

    fn visit(&mut self, expr: &'a Expr) -> Self::Output {
        Ok(match expr {
            Expr::Assign(name, value) => {
                let value = self.evaluate(&*value)?;
                self.environment.assign(&name, value.clone());
                value
            },
            Expr::Binary(left, op, right) => {
                let left = self.evaluate(&*left)?;
                let right = self.evaluate(&*right)?;

                match op.ty {
                    TokenType::Minus => {
                        let (left, right) = number_operands(&op, left, right)?;
                        Value::Number(left - right)
                    },
                    TokenType::Slash => {
                        let (left, right) = number_operands(&op, left, right)?;
                        Value::Number(left / right)
                    },
                    TokenType::Star => {
                        let (left, right) = number_operands(&op, left, right)?;
                        Value::Number(left * right)
                    },
                    TokenType::Plus => {
                        if let (Ok(left), Ok(right)) =
                            (left.number(), right.number())
                        {
                            Value::Number(left + right)
                        } else if let (Ok(left), Ok(right)) =
                            (left.string(), right.string())
                        {
                            let mut left = left.clone();
                            left.push_tendril(right);
                            Value::String(left)
                        } else {
                            return Err(LoxError::runtime(
                                &op,
                                "requires two numbers or two strings",
                            ));
                        }
                    },
                    TokenType::Greater => {
                        let (left, right) = number_operands(&op, left, right)?;
                        Value::Bool(left > right)
                    },
                    TokenType::GreaterEqual => {
                        let (left, right) = number_operands(&op, left, right)?;
                        Value::Bool(left >= right)
                    },
                    TokenType::Less => {
                        let (left, right) = number_operands(&op, left, right)?;
                        Value::Bool(left < right)
                    },
                    TokenType::LessEqual => {
                        let (left, right) = number_operands(&op, left, right)?;
                        Value::Bool(left <= right)
                    },
                    TokenType::BangEqual => Value::Bool(!is_equal(left, right)),
                    TokenType::EqualEqual => Value::Bool(is_equal(left, right)),
                    _ => {
                        return Err(LoxError::runtime(
                            &op,
                            format!("unexpected token type: {:?}", op.ty),
                        ))
                    },
                }
            },
            Expr::Call(callee, paren, args) => {
                let callee = self.evaluate(&*callee)?;

                let args = args
                    .into_iter()
                    .map(|arg| self.evaluate(&arg))
                    .collect::<Result<Vec<_>, _>>()?;

                let function = callee.callable()?;

                let n_args = args.len();
                let arity = function.arity();
                if n_args != arity {
                    return Err(LoxError::runtime(
                        paren,
                        format!(
                            "expected {} arguments but got {}",
                            arity, n_args
                        ),
                    ));
                }

                function.call(self, args)?
            },
            Expr::Grouping(e) => return self.evaluate(e),
            Expr::Literal(v) => v.clone(),
            Expr::Logical(left, op, right) => {
                let left = self.evaluate(&*left)?;

                match op.ty {
                    TokenType::And if is_truthy(&left) => left.clone(),
                    TokenType::Or if is_truthy(&left) => left.clone(),
                    _ => self.evaluate(&*right)?,
                }
            },
            Expr::Unary(op, right) => {
                let right = self.evaluate(&*right)?;
                match op.ty {
                    TokenType::Minus => Value::Number(-*right.number()?),
                    TokenType::Bang => Value::Bool(!is_truthy(&right)),
                    _ => unreachable!(),
                }
            },
            Expr::Variable(name) => self
                .environment
                .get(&name.lexeme)
                .ok_or_else(|| {
                    LoxError::runtime(
                        &name,
                        format!("Undefined variable: {}", name.lexeme),
                    )
                })?
                .clone(),
        })
    }
}

fn is_truthy(v: &Value) -> bool {
    match v {
        Value::Nil => false,
        Value::Bool(b) => *b,
        _ => true,
    }
}

fn is_equal(left: Value, right: Value) -> bool {
    left == right
}

fn number_operand(op: &Token, right: Value) -> Result<f64, LoxError> {
    Ok(*right.number().map_err(|e| LoxError::runtime(op, format!("{}", e)))?)
}
fn number_operands(
    op: &Token,
    left: Value,
    right: Value,
) -> Result<(f64, f64), LoxError> {
    Ok((number_operand(op, left)?, number_operand(op, right)?))
}

impl<'a> Visitor<&'a Stmt> for Interpreter {
    type Output = Result<(), LoxError>;
    fn visit(&mut self, stmt: &'a Stmt) -> Self::Output {
        match stmt {
            Stmt::Block(stmts) => {
                self.execute_block(stmts)?;
            },
            Stmt::Expr(expr) => {
                self.evaluate(expr)?;
            },
            Stmt::If(cond, then, otherwise) => {
                if is_truthy(&self.evaluate(cond)?) {
                    self.execute(&*then)?;
                } else if let Some(otherwise) = otherwise {
                    self.execute(&*otherwise)?;
                }
            },
            Stmt::Print(expr) => {
                println!("{}", self.evaluate(expr)?);
            },
            Stmt::Var(name, expr) => {
                let value = self.evaluate(expr)?;
                self.environment.define(&name, value);
            },
            Stmt::While(cond, body) => {
                while is_truthy(&self.evaluate(cond)?) {
                    self.execute(&*body)?;
                }
            },
        }
        Ok(())
    }
}

impl Interpreter {
    fn execute_block(&mut self, stmts: &Vec<Stmt>) -> Result<(), LoxError> {
        self.with_env(
            |e| Environment::child(e),
            move |interp| {
                stmts.iter().fold(Ok(()), |acc, stmt| {
                    acc.and_then(|_| interp.execute(stmt))
                })
            },
        )
    }

    fn with_env<E, F>(&mut self, env_builder: E, f: F) -> Result<(), LoxError>
    where
        E: FnOnce(Environment) -> Environment,
        F: FnOnce(&mut Interpreter) -> Result<(), LoxError>,
    {
        self.environment =
            env_builder(replace(&mut self.environment, Default::default()));

        let res = f(self);

        self.environment = replace(&mut self.environment, Default::default())
            .into_parent()
            .unwrap();

        res
    }
}
