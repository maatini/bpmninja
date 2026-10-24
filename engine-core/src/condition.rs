//! Condition evaluator for gateway routing.
//!
//! Die Bedingungssprache ist bewusst klein und unabhängig von Rhai: ein eigener
//! Lexer und ein Recursive-Descent-Parser mit Kurzschlussauswertung.
//!
//! Unterstützt werden:
//! - logische Operatoren `&&`/`and`, `||`/`or`, `!`/`not` (Kurzschluss)
//! - Vergleiche `==`, `!=`, `>`, `<`, `>=`, `<=`
//! - Arithmetik `+`, `-`, `*`, `/`, `%` und unäres Minus
//! - Klammern, Zahlen, Strings (einfache und doppelte Anführungszeichen inkl. Escapes),
//!   `true`/`false`/`null`
//! - Variablenpfade wie `order.amount` oder `items[0].id`
//! - eine vollständige `${…}`- oder `#{…}`-Umhüllung
//!
//! Fehlende Variablen und Pfade werden zu einem internen „undefined“ und
//! vergleichen sich immer als `false`. Laufzeitfehler (Division durch Null,
//! Typfehler) liefern `false` und eine `tracing::warn!`-Meldung, niemals einen Panic.

use std::cmp::Ordering;
use std::collections::HashMap;

use serde_json::Value;

/// Wertet eine Bedingung gegen die Token-Variablen aus.
///
/// Nicht-boolesche Ergebnisse werden über die bisherige Truthiness-Semantik
/// interpretiert: nicht `null`, nicht `false`, nicht `0`, nicht leerer String.
/// Fehlende Variablen, Parse- und Laufzeitfehler ergeben `false`.
pub fn evaluate_condition(expr: &str, variables: &HashMap<String, Value>) -> bool {
    if expr.trim().is_empty() {
        return false;
    }

    let ast = match parse_condition(expr) {
        Ok(ast) => ast,
        Err(err) => {
            tracing::warn!(condition = %expr, error = %err, "Ungültiger Bedingungsausdruck");
            return false;
        }
    };

    match evaluate(&ast, variables) {
        Ok(value) => truthy(&value),
        Err(err) => {
            tracing::warn!(condition = %expr, error = %err, "Bedingung konnte nicht ausgewertet werden");
            false
        }
    }
}

/// Prüft einen Bedingungsausdruck auf Syntaxfehler.
///
/// Der zurückgegebene Fehler enthält die 1-basierte Zeichenposition in der
/// ursprünglichen Zeichenkette.
pub fn validate_condition(expr: &str) -> Result<(), String> {
    parse_condition(expr).map(|_| ())
}

// ---------------------------------------------------------------------------
// AST
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Expr {
    Literal(EvalValue),
    Path(Vec<PathSegment>),
    Not(Box<Expr>),
    Neg(Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Compare(CmpOp, Box<Expr>, Box<Expr>),
    Arith(ArithOp, Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
enum PathSegment {
    Field(String),
    Index(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CmpOp {
    Eq,
    Ne,
    Gt,
    Lt,
    Ge,
    Le,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArithOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

impl ArithOp {
    fn symbol(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::Rem => "%",
        }
    }
}

/// Interner Wert, der „fehlende Variable“ von einem echten `null` unterscheidet.
#[derive(Debug, Clone, PartialEq)]
enum EvalValue {
    Undefined,
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    /// Arrays und Objekte bleiben als `serde_json::Value` erhalten.
    Json(Value),
}

impl EvalValue {
    fn from_json(value: &Value) -> Self {
        match value {
            Value::Null => Self::Null,
            Value::Bool(b) => Self::Bool(*b),
            Value::Number(n) => n
                .as_f64()
                .map_or_else(|| Self::Json(value.clone()), Self::Number),
            Value::String(s) => Self::String(s.clone()),
            Value::Array(_) | Value::Object(_) => Self::Json(value.clone()),
        }
    }

    fn type_name(&self) -> &'static str {
        match self {
            Self::Undefined => "undefined",
            Self::Null => "null",
            Self::Bool(_) => "boolean",
            Self::Number(_) => "number",
            Self::String(_) => "string",
            Self::Json(Value::Array(_)) => "array",
            Self::Json(Value::Object(_)) => "object",
            Self::Json(_) => "value",
        }
    }
}

// ---------------------------------------------------------------------------
// Lexer
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum TokenKind {
    Number(f64),
    Str(String),
    Ident(String),
    True,
    False,
    Null,
    Or,
    And,
    Not,
    Eq,
    Ne,
    Gt,
    Lt,
    Ge,
    Le,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Dot,
    Eof,
}

impl TokenKind {
    fn describe(&self) -> String {
        match self {
            Self::Number(n) => format!("number '{n}'"),
            Self::Str(_) => "string literal".to_string(),
            Self::Ident(name) => format!("identifier '{name}'"),
            Self::True => "'true'".to_string(),
            Self::False => "'false'".to_string(),
            Self::Null => "'null'".to_string(),
            Self::Or => "'||'".to_string(),
            Self::And => "'&&'".to_string(),
            Self::Not => "'!'".to_string(),
            Self::Eq => "'=='".to_string(),
            Self::Ne => "'!='".to_string(),
            Self::Gt => "'>'".to_string(),
            Self::Lt => "'<'".to_string(),
            Self::Ge => "'>='".to_string(),
            Self::Le => "'<='".to_string(),
            Self::Plus => "'+'".to_string(),
            Self::Minus => "'-'".to_string(),
            Self::Star => "'*'".to_string(),
            Self::Slash => "'/'".to_string(),
            Self::Percent => "'%'".to_string(),
            Self::LParen => "'('".to_string(),
            Self::RParen => "')'".to_string(),
            Self::LBracket => "'['".to_string(),
            Self::RBracket => "']'".to_string(),
            Self::Dot => "'.'".to_string(),
            Self::Eof => "end of expression".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
struct SpannedToken {
    kind: TokenKind,
    /// 1-basierte Zeichenposition in der ursprünglichen Zeichenkette.
    pos: usize,
}

fn error(pos: usize, message: impl std::fmt::Display) -> String {
    format!("invalid condition at position {pos}: {message}")
}

/// Entfernt eine vollständige `${…}`/`#{…}`-Umhüllung und liefert die inneren
/// Zeichen samt Offset für die Positionsberechnung.
fn prepare(expr: &str) -> Result<(Vec<char>, usize), String> {
    let chars: Vec<char> = expr.chars().collect();
    let Some(start) = chars.iter().position(|c| !c.is_whitespace()) else {
        return Err("invalid condition: expression is empty".to_string());
    };
    let end = chars
        .iter()
        .rposition(|c| !c.is_whitespace())
        .map_or(start, |i| i + 1);
    let inner = &chars[start..end];

    let wrapped = inner.len() >= 2 && (inner[0] == '$' || inner[0] == '#') && inner[1] == '{';
    if wrapped {
        if inner.last() != Some(&'}') {
            return Err(error(
                start + 1,
                "expression wrapper must surround the whole expression",
            ));
        }
        let body = &inner[2..inner.len() - 1];
        if body.iter().all(|c| c.is_whitespace()) {
            return Err(error(start + 3, "expression is empty"));
        }
        return Ok((body.to_vec(), start + 2));
    }

    Ok((inner.to_vec(), start))
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn tokenize(chars: &[char], base: usize) -> Result<Vec<SpannedToken>, String> {
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }

        let pos = base + i + 1;

        // Zweizeichen-Operatoren zuerst, damit `>=` nicht als `>` + `=` endet.
        if i + 1 < chars.len() {
            let pair = (c, chars[i + 1]);
            let kind = match pair {
                ('|', '|') => Some(TokenKind::Or),
                ('&', '&') => Some(TokenKind::And),
                ('=', '=') => Some(TokenKind::Eq),
                ('!', '=') => Some(TokenKind::Ne),
                ('>', '=') => Some(TokenKind::Ge),
                ('<', '=') => Some(TokenKind::Le),
                _ => None,
            };
            if let Some(kind) = kind {
                tokens.push(SpannedToken { kind, pos });
                i += 2;
                continue;
            }
        }

        match c {
            '(' => {
                tokens.push(SpannedToken {
                    kind: TokenKind::LParen,
                    pos,
                });
                i += 1;
            }
            ')' => {
                tokens.push(SpannedToken {
                    kind: TokenKind::RParen,
                    pos,
                });
                i += 1;
            }
            '[' => {
                tokens.push(SpannedToken {
                    kind: TokenKind::LBracket,
                    pos,
                });
                i += 1;
            }
            ']' => {
                tokens.push(SpannedToken {
                    kind: TokenKind::RBracket,
                    pos,
                });
                i += 1;
            }
            '.' => {
                tokens.push(SpannedToken {
                    kind: TokenKind::Dot,
                    pos,
                });
                i += 1;
            }
            '+' => {
                tokens.push(SpannedToken {
                    kind: TokenKind::Plus,
                    pos,
                });
                i += 1;
            }
            '-' => {
                tokens.push(SpannedToken {
                    kind: TokenKind::Minus,
                    pos,
                });
                i += 1;
            }
            '*' => {
                tokens.push(SpannedToken {
                    kind: TokenKind::Star,
                    pos,
                });
                i += 1;
            }
            '/' => {
                tokens.push(SpannedToken {
                    kind: TokenKind::Slash,
                    pos,
                });
                i += 1;
            }
            '%' => {
                tokens.push(SpannedToken {
                    kind: TokenKind::Percent,
                    pos,
                });
                i += 1;
            }
            '>' => {
                tokens.push(SpannedToken {
                    kind: TokenKind::Gt,
                    pos,
                });
                i += 1;
            }
            '<' => {
                tokens.push(SpannedToken {
                    kind: TokenKind::Lt,
                    pos,
                });
                i += 1;
            }
            '!' => {
                tokens.push(SpannedToken {
                    kind: TokenKind::Not,
                    pos,
                });
                i += 1;
            }
            '"' | '\'' => {
                let (value, next) = read_string(chars, i, base)?;
                tokens.push(SpannedToken {
                    kind: TokenKind::Str(value),
                    pos,
                });
                i = next;
            }
            '0'..='9' => {
                let (value, next) = read_number(chars, i, base)?;
                tokens.push(SpannedToken {
                    kind: TokenKind::Number(value),
                    pos,
                });
                i = next;
            }
            '=' => {
                return Err(error(
                    pos,
                    "single '=' is not a comparison operator; use '=='",
                ));
            }
            '&' => return Err(error(pos, "unexpected '&'; use '&&'")),
            '|' => return Err(error(pos, "unexpected '|'; use '||'")),
            '$' | '#' if chars.get(i + 1) == Some(&'{') => {
                return Err(error(
                    pos,
                    "expression wrapper is only allowed around the entire expression",
                ));
            }
            _ if is_ident_start(c) => {
                let mut end = i + 1;
                while end < chars.len() && is_ident_continue(chars[end]) {
                    end += 1;
                }
                let text: String = chars[i..end].iter().collect();
                let kind = match text.as_str() {
                    "and" => TokenKind::And,
                    "or" => TokenKind::Or,
                    "not" => TokenKind::Not,
                    "true" => TokenKind::True,
                    "false" => TokenKind::False,
                    "null" => TokenKind::Null,
                    _ => TokenKind::Ident(text),
                };
                tokens.push(SpannedToken { kind, pos });
                i = end;
            }
            _ => return Err(error(pos, format!("unexpected character '{c}'"))),
        }
    }

    tokens.push(SpannedToken {
        kind: TokenKind::Eof,
        pos: base + chars.len() + 1,
    });
    Ok(tokens)
}

fn read_number(chars: &[char], start: usize, base: usize) -> Result<(f64, usize), String> {
    let mut end = start;
    while end < chars.len() && chars[end].is_ascii_digit() {
        end += 1;
    }
    if end < chars.len()
        && chars[end] == '.'
        && chars.get(end + 1).is_some_and(|c| c.is_ascii_digit())
    {
        end += 1;
        while end < chars.len() && chars[end].is_ascii_digit() {
            end += 1;
        }
    }

    let text: String = chars[start..end].iter().collect();
    let value = text
        .parse::<f64>()
        .map_err(|_| error(base + start + 1, format!("invalid number '{text}'")))?;
    Ok((value, end))
}

fn read_string(chars: &[char], start: usize, base: usize) -> Result<(String, usize), String> {
    let quote = chars[start];
    let mut i = start + 1;
    let mut out = String::new();

    while i < chars.len() {
        let c = chars[i];
        if c == quote {
            return Ok((out, i + 1));
        }
        if c != '\\' {
            out.push(c);
            i += 1;
            continue;
        }

        let Some(&escaped) = chars.get(i + 1) else {
            return Err(error(base + i + 1, "unterminated escape sequence"));
        };
        match escaped {
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            '0' => out.push('\0'),
            '\\' => out.push('\\'),
            '\'' => out.push('\''),
            '"' => out.push('"'),
            '/' => out.push('/'),
            'u' => {
                let hex: String = chars
                    .get(i + 2..i + 6)
                    .ok_or_else(|| error(base + i + 1, "incomplete unicode escape"))?
                    .iter()
                    .collect();
                let code = u32::from_str_radix(&hex, 16).map_err(|_| {
                    error(base + i + 1, format!("invalid unicode escape '\\u{hex}'"))
                })?;
                let decoded = char::from_u32(code).ok_or_else(|| {
                    error(
                        base + i + 1,
                        format!("invalid unicode code point '\\u{hex}'"),
                    )
                })?;
                out.push(decoded);
                i += 6;
                continue;
            }
            other => {
                return Err(error(
                    base + i + 1,
                    format!("invalid escape sequence '\\{other}'"),
                ));
            }
        }
        i += 2;
    }

    Err(error(base + start + 1, "unterminated string literal"))
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

struct Parser {
    tokens: Vec<SpannedToken>,
    index: usize,
}

impl Parser {
    fn new(tokens: Vec<SpannedToken>) -> Self {
        Self { tokens, index: 0 }
    }

    fn peek(&self) -> &SpannedToken {
        // Der Eof-Token ist immer vorhanden; `advance` läuft nicht darüber hinaus.
        &self.tokens[self.index]
    }

    fn advance(&mut self) {
        if self.index + 1 < self.tokens.len() {
            self.index += 1;
        }
    }

    fn parse(&mut self) -> Result<Expr, String> {
        let expr = self.parse_or()?;
        if !matches!(self.peek().kind, TokenKind::Eof) {
            return Err(error(
                self.peek().pos,
                format!("unexpected token {}", self.peek().kind.describe()),
            ));
        }
        Ok(expr)
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_and()?;
        while matches!(self.peek().kind, TokenKind::Or) {
            self.advance();
            let rhs = self.parse_and()?;
            lhs = Expr::Or(Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_not()?;
        while matches!(self.peek().kind, TokenKind::And) {
            self.advance();
            let rhs = self.parse_not()?;
            lhs = Expr::And(Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_not(&mut self) -> Result<Expr, String> {
        if matches!(self.peek().kind, TokenKind::Not) {
            self.advance();
            let inner = self.parse_not()?;
            return Ok(Expr::Not(Box::new(inner)));
        }
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let lhs = self.parse_additive()?;
        let op = match self.peek().kind {
            TokenKind::Eq => Some(CmpOp::Eq),
            TokenKind::Ne => Some(CmpOp::Ne),
            TokenKind::Gt => Some(CmpOp::Gt),
            TokenKind::Lt => Some(CmpOp::Lt),
            TokenKind::Ge => Some(CmpOp::Ge),
            TokenKind::Le => Some(CmpOp::Le),
            _ => None,
        };
        if let Some(op) = op {
            self.advance();
            let rhs = self.parse_additive()?;
            return Ok(Expr::Compare(op, Box::new(lhs), Box::new(rhs)));
        }
        Ok(lhs)
    }

    fn parse_additive(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_multiplicative()?;
        loop {
            let op = match self.peek().kind {
                TokenKind::Plus => ArithOp::Add,
                TokenKind::Minus => ArithOp::Sub,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_multiplicative()?;
            lhs = Expr::Arith(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_unary()?;
        loop {
            let op = match self.peek().kind {
                TokenKind::Star => ArithOp::Mul,
                TokenKind::Slash => ArithOp::Div,
                TokenKind::Percent => ArithOp::Rem,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_unary()?;
            lhs = Expr::Arith(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        if matches!(self.peek().kind, TokenKind::Minus) {
            self.advance();
            let inner = self.parse_unary()?;
            return Ok(Expr::Neg(Box::new(inner)));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        let token = self.peek().clone();
        match token.kind {
            TokenKind::Number(n) => {
                self.advance();
                Ok(Expr::Literal(EvalValue::Number(n)))
            }
            TokenKind::Str(s) => {
                self.advance();
                Ok(Expr::Literal(EvalValue::String(s)))
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr::Literal(EvalValue::Bool(true)))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::Literal(EvalValue::Bool(false)))
            }
            TokenKind::Null => {
                self.advance();
                Ok(Expr::Literal(EvalValue::Null))
            }
            TokenKind::LParen => {
                self.advance();
                let inner = self.parse_or()?;
                if !matches!(self.peek().kind, TokenKind::RParen) {
                    return Err(error(self.peek().pos, "expected ')'"));
                }
                self.advance();
                Ok(inner)
            }
            TokenKind::Ident(name) => {
                self.advance();
                self.parse_path(name)
            }
            TokenKind::Eof => Err(error(token.pos, "unexpected end of expression")),
            _ => Err(error(
                token.pos,
                format!("unexpected token {}", token.kind.describe()),
            )),
        }
    }

    fn parse_path(&mut self, root: String) -> Result<Expr, String> {
        let mut segments = vec![PathSegment::Field(root)];

        loop {
            match self.peek().kind {
                TokenKind::Dot => {
                    self.advance();
                    let TokenKind::Ident(name) = self.peek().kind.clone() else {
                        return Err(error(self.peek().pos, "expected field name after '.'"));
                    };
                    self.advance();
                    segments.push(PathSegment::Field(name));
                }
                TokenKind::LBracket => {
                    self.advance();
                    let TokenKind::Number(value) = self.peek().kind else {
                        return Err(error(self.peek().pos, "expected array index"));
                    };
                    if value < 0.0 || value.fract() != 0.0 || value > usize::MAX as f64 {
                        return Err(error(
                            self.peek().pos,
                            "array index must be a non-negative integer",
                        ));
                    }
                    self.advance();
                    if !matches!(self.peek().kind, TokenKind::RBracket) {
                        return Err(error(self.peek().pos, "expected ']'"));
                    }
                    self.advance();
                    // Der Wertebereich ist oben geprüft; der Cast ist verlustfrei.
                    segments.push(PathSegment::Index(value as usize));
                }
                _ => break,
            }
        }

        Ok(Expr::Path(segments))
    }
}

fn parse_condition(expr: &str) -> Result<Expr, String> {
    let (chars, base) = prepare(expr)?;
    let tokens = tokenize(&chars, base)?;
    Parser::new(tokens).parse()
}

// ---------------------------------------------------------------------------
// Auswertung
// ---------------------------------------------------------------------------

fn evaluate(expr: &Expr, variables: &HashMap<String, Value>) -> Result<EvalValue, String> {
    match expr {
        Expr::Literal(value) => Ok(value.clone()),
        Expr::Path(segments) => Ok(resolve_path(segments, variables)),
        Expr::Not(inner) => {
            let value = evaluate(inner, variables)?;
            Ok(EvalValue::Bool(!truthy(&value)))
        }
        Expr::Neg(inner) => match evaluate(inner, variables)? {
            EvalValue::Undefined => Ok(EvalValue::Undefined),
            EvalValue::Number(n) => Ok(EvalValue::Number(-n)),
            other => Err(format!("cannot negate {}", other.type_name())),
        },
        Expr::And(lhs, rhs) => {
            if !truthy(&evaluate(lhs, variables)?) {
                return Ok(EvalValue::Bool(false));
            }
            Ok(EvalValue::Bool(truthy(&evaluate(rhs, variables)?)))
        }
        Expr::Or(lhs, rhs) => {
            if truthy(&evaluate(lhs, variables)?) {
                return Ok(EvalValue::Bool(true));
            }
            Ok(EvalValue::Bool(truthy(&evaluate(rhs, variables)?)))
        }
        Expr::Compare(op, lhs, rhs) => {
            let lhs = evaluate(lhs, variables)?;
            let rhs = evaluate(rhs, variables)?;
            Ok(EvalValue::Bool(compare(*op, &lhs, &rhs)?))
        }
        Expr::Arith(op, lhs, rhs) => {
            let lhs = evaluate(lhs, variables)?;
            let rhs = evaluate(rhs, variables)?;
            arithmetic(*op, lhs, rhs)
        }
    }
}

fn resolve_path(segments: &[PathSegment], variables: &HashMap<String, Value>) -> EvalValue {
    let Some(PathSegment::Field(root)) = segments.first() else {
        return EvalValue::Undefined;
    };
    let Some(value) = variables.get(root) else {
        return EvalValue::Undefined;
    };
    let mut current = EvalValue::from_json(value);

    for segment in &segments[1..] {
        let next = match (&current, segment) {
            (EvalValue::Json(Value::Object(map)), PathSegment::Field(name)) => {
                map.get(name).map(EvalValue::from_json)
            }
            (EvalValue::Json(Value::Array(items)), PathSegment::Index(index)) => {
                items.get(*index).map(EvalValue::from_json)
            }
            _ => None,
        };
        match next {
            Some(value) => current = value,
            None => return EvalValue::Undefined,
        }
    }

    current
}

fn truthy(value: &EvalValue) -> bool {
    match value {
        EvalValue::Undefined | EvalValue::Null => false,
        EvalValue::Bool(b) => *b,
        EvalValue::Number(n) => *n != 0.0,
        EvalValue::String(s) => !s.is_empty(),
        // Arrays und Objekte sind truthy (bisheriges Verhalten).
        EvalValue::Json(_) => true,
    }
}

fn compare(op: CmpOp, lhs: &EvalValue, rhs: &EvalValue) -> Result<bool, String> {
    match op {
        CmpOp::Eq => Ok(values_eq(lhs, rhs)),
        // Fehlende Werte sind mit nichts ungleich; das entspricht dem bisherigen
        // Verhalten, bei dem ein fehlender Operand immer `false` ergab.
        CmpOp::Ne => {
            if is_undefined(lhs) || is_undefined(rhs) {
                return Ok(false);
            }
            Ok(!values_eq(lhs, rhs))
        }
        CmpOp::Gt | CmpOp::Lt | CmpOp::Ge | CmpOp::Le => {
            if is_undefined(lhs) || is_undefined(rhs) {
                return Ok(false);
            }
            let (a, b) = match (lhs, rhs) {
                (EvalValue::Number(a), EvalValue::Number(b)) => (*a, *b),
                _ => {
                    return Err(format!(
                        "cannot compare {} with {}",
                        lhs.type_name(),
                        rhs.type_name()
                    ));
                }
            };
            let ordering = a
                .partial_cmp(&b)
                .ok_or_else(|| "cannot compare NaN with a number".to_string())?;
            Ok(match op {
                CmpOp::Gt => ordering == Ordering::Greater,
                CmpOp::Lt => ordering == Ordering::Less,
                CmpOp::Ge => ordering != Ordering::Less,
                CmpOp::Le => ordering != Ordering::Greater,
                CmpOp::Eq | CmpOp::Ne => false,
            })
        }
    }
}

fn is_undefined(value: &EvalValue) -> bool {
    matches!(value, EvalValue::Undefined)
}

/// Gleichheit mit `f64`-Epsilon für Zahlen; `undefined` ist mit nichts gleich.
fn values_eq(lhs: &EvalValue, rhs: &EvalValue) -> bool {
    match (lhs, rhs) {
        (EvalValue::Undefined, _) | (_, EvalValue::Undefined) => false,
        (EvalValue::Null, EvalValue::Null) => true,
        (EvalValue::Bool(a), EvalValue::Bool(b)) => a == b,
        (EvalValue::Number(a), EvalValue::Number(b)) => (a - b).abs() < f64::EPSILON,
        (EvalValue::String(a), EvalValue::String(b)) => a == b,
        (EvalValue::Json(a), EvalValue::Json(b)) => a == b,
        _ => false,
    }
}

fn arithmetic(op: ArithOp, lhs: EvalValue, rhs: EvalValue) -> Result<EvalValue, String> {
    // Fehlende Werte propagieren, statt als Typfehler zu gelten.
    if is_undefined(&lhs) || is_undefined(&rhs) {
        return Ok(EvalValue::Undefined);
    }

    let (a, b) = match (&lhs, &rhs) {
        (EvalValue::Number(a), EvalValue::Number(b)) => (*a, *b),
        _ => {
            return Err(format!(
                "cannot apply '{}' to {} and {}",
                op.symbol(),
                lhs.type_name(),
                rhs.type_name()
            ));
        }
    };

    let result = match op {
        ArithOp::Add => a + b,
        ArithOp::Sub => a - b,
        ArithOp::Mul => a * b,
        ArithOp::Div | ArithOp::Rem if b == 0.0 => {
            return Err(format!("division by zero in '{}'", op.symbol()));
        }
        ArithOp::Div => a / b,
        ArithOp::Rem => a % b,
    };
    Ok(EvalValue::Number(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn map(pairs: &[(&str, Value)]) -> HashMap<String, Value> {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_string(), value.clone()))
            .collect()
    }

    fn variables() -> HashMap<String, Value> {
        map(&[
            ("x", json!(1)),
            ("y", json!(2)),
            ("amount", json!(150)),
            ("status", json!("approved")),
            ("approved", json!(true)),
            ("flag", json!(true)),
            ("vip", json!(false)),
            ("name", json!("Bob")),
            ("empty", json!("")),
            ("nothing", json!(null)),
            ("zero", json!(0)),
        ])
    }

    #[test]
    fn existing_expression_forms_still_work() {
        let vars = variables();
        assert!(evaluate_condition("x == 1", &vars));
        assert!(evaluate_condition("amount > 100", &vars));
        assert!(evaluate_condition("status == 'approved'", &vars));
        assert!(evaluate_condition("flag", &vars));
        assert!(!evaluate_condition("x == 2", &vars));
    }

    #[test]
    fn equality_and_inequality() {
        let vars = variables();
        assert!(evaluate_condition("status == 'approved'", &vars));
        assert!(evaluate_condition("status == \"approved\"", &vars));
        assert!(!evaluate_condition("status == 'rejected'", &vars));
        assert!(evaluate_condition("status != 'rejected'", &vars));
        assert!(!evaluate_condition("status != 'approved'", &vars));

        assert!(evaluate_condition("x == 1", &vars));
        assert!(!evaluate_condition("x == 1.0001", &vars));
        assert!(evaluate_condition("approved == true", &vars));
        assert!(evaluate_condition("vip == false", &vars));
        assert!(evaluate_condition("approved != false", &vars));
    }

    #[test]
    fn numeric_comparisons() {
        let vars = map(&[("amount", json!(150.5)), ("score", json!(10))]);
        assert!(evaluate_condition("amount > 100", &vars));
        assert!(evaluate_condition("amount >= 150.5", &vars));
        assert!(evaluate_condition("amount < 200", &vars));
        assert!(evaluate_condition("amount <= 150.5", &vars));
        assert!(!evaluate_condition("amount > 200", &vars));
        assert!(!evaluate_condition("amount < 100", &vars));
        assert!(evaluate_condition("score > 5", &vars));
        assert!(evaluate_condition("score < 15", &vars));
    }

    #[test]
    fn numeric_equality_uses_epsilon() {
        let a = EvalValue::Number(1.0);
        let b = EvalValue::Number(1.0);
        assert!(values_eq(&a, &b));

        let sum = EvalValue::Number(0.1 + 0.2);
        let expected = EvalValue::Number(0.3);
        assert!(values_eq(&sum, &expected));

        let different = EvalValue::Number(2.0);
        assert!(!values_eq(&a, &different));

        // Genau eine EPSILON-Differenz ist bereits ungleich.
        let edge = EvalValue::Number(1.0 + f64::EPSILON);
        assert!(!values_eq(&a, &edge));
    }

    #[test]
    fn truthiness_of_non_bool_values() {
        let vars = variables();
        assert!(evaluate_condition("name", &vars));
        assert!(!evaluate_condition("empty", &vars));
        assert!(!evaluate_condition("nothing", &vars));
        assert!(!evaluate_condition("zero", &vars));
        assert!(evaluate_condition("amount", &vars));
        assert!(!evaluate_condition("missing_var", &vars));
    }

    #[test]
    fn logical_precedence() {
        let vars = variables();
        // `&&` bindet stärker als `||`.
        assert!(evaluate_condition("x == 1 && y == 2 || vip", &vars));
        assert!(!evaluate_condition("x == 99 && y == 2 || vip", &vars));
        assert!(evaluate_condition(
            "true || false && false",
            &HashMap::new()
        ));
        assert!(!evaluate_condition(
            "(true || false) && false",
            &HashMap::new()
        ));
        // `not` bindet stärker als `&&`.
        assert!(evaluate_condition("not vip and approved", &vars));
    }

    #[test]
    fn short_circuit_avoids_runtime_errors() {
        let vars = HashMap::new();
        // Der Division-durch-Null-Zweig darf nicht ausgewertet werden.
        assert!(!evaluate_condition("false && 1 / 0 == 1", &vars));
        assert!(evaluate_condition("true || 1 / 0 == 1", &vars));
        // Ohne Kurzschluss ist der Ausdruck ein Laufzeitfehler → false.
        assert!(!evaluate_condition("true && 1 / 0 == 1", &vars));
    }

    #[test]
    fn parentheses_override_precedence() {
        let vars = map(&[("a", json!(false)), ("b", json!(false)), ("c", json!(true))]);
        assert!(!evaluate_condition("(a || b) && c", &vars));
        assert!(evaluate_condition("a || b || c", &vars));
        assert!(evaluate_condition("!(a || b) || c", &vars));
        assert!(evaluate_condition("(1 + 2) * 3 == 9", &vars));
        assert!(!evaluate_condition("1 + 2 * 3 == 9", &vars));
    }

    #[test]
    fn unary_not_and_keyword_not() {
        let vars = variables();
        assert!(!evaluate_condition("!(x == 1)", &vars));
        assert!(!evaluate_condition("not (x == 1)", &vars));
        assert!(evaluate_condition("!(x == 2)", &vars));
        assert!(!evaluate_condition("!approved", &vars));
        assert!(evaluate_condition("not vip", &vars));
        assert!(evaluate_condition("not not approved", &vars));
        assert!(evaluate_condition("!missing", &vars));
    }

    #[test]
    fn expression_wrappers() {
        let vars = variables();
        assert!(evaluate_condition("${amount > 100}", &vars));
        assert!(evaluate_condition("#{status == 'approved'}", &vars));
        assert!(evaluate_condition(
            "  ${ amount > 100 && approved }  ",
            &vars
        ));
        assert!(!evaluate_condition("${amount > 100 && vip}", &vars));
        assert!(!evaluate_condition("${amount > 1000}", &vars));
        assert!(evaluate_condition(
            "note == '${literal}'",
            &map(&[("note", json!("${literal}"))])
        ));
    }

    #[test]
    fn wrapper_misuse_is_rejected() {
        for expr in [
            "${amount > 100",
            "#{status == 'ok'",
            "${a == 1} && b",
            "a == 1 && ${b}",
            "a ${b}",
            "#{}",
            "${ }",
        ] {
            let err = validate_condition(expr)
                .expect_err(&format!("expression should be rejected: {expr}"));
            assert!(
                err.contains("wrapper") || err.contains("empty") || err.contains("unexpected"),
                "unexpected error for '{expr}': {err}"
            );
        }
    }

    #[test]
    fn strings_may_contain_logical_operators() {
        let vars = map(&[
            ("label", json!("a && b || c")),
            ("hint", json!("x || y")),
            ("quoted", json!("'single'")),
        ]);
        assert!(evaluate_condition("label == 'a && b || c'", &vars));
        assert!(!evaluate_condition("label == 'a && b'", &vars));
        assert!(evaluate_condition("hint == \"x || y\"", &vars));
        assert!(evaluate_condition("quoted == \"'single'\"", &vars));
        assert!(evaluate_condition("label != 'x' && hint != 'z'", &vars));
    }

    #[test]
    fn string_escapes() {
        let vars = map(&[
            ("note", json!("he said \"hi\"")),
            ("owner", json!("it's")),
            ("text", json!("a\nb")),
            ("path", json!("C:\\tmp")),
            ("sym", json!("é")),
        ]);
        assert!(evaluate_condition(r#"note == "he said \"hi\"""#, &vars));
        assert!(evaluate_condition(r#"owner == 'it\'s'"#, &vars));
        assert!(evaluate_condition(r#"text == 'a\nb'"#, &vars));
        assert!(evaluate_condition(r#"path == 'C:\\tmp'"#, &vars));
        assert!(evaluate_condition(r#"sym == '\u00e9'"#, &vars));
    }

    #[test]
    fn paths_and_array_indices() {
        let vars = map(&[
            ("order", json!({"amount": 150, "customer": {"vip": true}})),
            ("items", json!([{"id": 7, "tags": ["a"]}, {"id": 8}])),
        ]);
        assert!(evaluate_condition("order.amount > 100", &vars));
        assert!(evaluate_condition("order.customer.vip", &vars));
        assert!(evaluate_condition("items[0].id == 7", &vars));
        assert!(evaluate_condition("items[1].id == 8", &vars));
        assert!(evaluate_condition("items[0].tags[0] == 'a'", &vars));
        assert!(!evaluate_condition("items[2].id == 7", &vars));
        assert!(!evaluate_condition("order.missing == 1", &vars));
        assert!(!evaluate_condition("order.amount.value == 1", &vars));
        assert!(!evaluate_condition("items.id == 7", &vars));
    }

    #[test]
    fn missing_variables_compare_as_false() {
        let vars = variables();
        assert!(!evaluate_condition("missing == 1", &vars));
        assert!(!evaluate_condition("missing != 1", &vars));
        assert!(!evaluate_condition("missing > 1", &vars));
        assert!(!evaluate_condition("missing < 1", &vars));
        assert!(!evaluate_condition("missing >= 0", &vars));
        assert!(!evaluate_condition("missing <= 0", &vars));
        assert!(!evaluate_condition("missing", &vars));
        assert!(!evaluate_condition("missing && true", &vars));
        assert!(!evaluate_condition("missing + 1 > 0", &vars));
        assert!(!evaluate_condition(
            "order.missing > 1",
            &map(&[("order", json!({"id": 1}))])
        ));
    }

    #[test]
    fn null_comparisons() {
        let vars = variables();
        assert!(evaluate_condition("nothing == null", &vars));
        assert!(!evaluate_condition("nothing != null", &vars));
        assert!(!evaluate_condition("x == null", &vars));
        assert!(evaluate_condition("x != null", &vars));
        // Fehlend ist nicht dasselbe wie null.
        assert!(!evaluate_condition("missing == null", &vars));
        assert!(!evaluate_condition("missing != null", &vars));
        assert!(evaluate_condition("null == null", &vars));
        assert!(!evaluate_condition("null != null", &vars));
        assert!(!evaluate_condition("nothing", &vars));
        assert!(evaluate_condition("!nothing", &vars));
    }

    #[test]
    fn arithmetic_operations() {
        let vars = map(&[("amount", json!(100))]);
        assert!(evaluate_condition("amount + 50 > 100", &vars));
        assert!(evaluate_condition("amount - 50 == 50", &vars));
        assert!(evaluate_condition("amount * 2 == 200", &vars));
        assert!(evaluate_condition("amount / 4 == 25", &vars));
        assert!(evaluate_condition("amount % 7 == 2", &vars));
        assert!(evaluate_condition("(amount + 50) / 3 == 50", &vars));
        assert!(evaluate_condition("-amount < 0", &vars));
        assert!(evaluate_condition("amount + -50 == 50", &vars));
        assert!(evaluate_condition("1 + 2 * 3 == 7", &HashMap::new()));
    }

    #[test]
    fn runtime_errors_return_false() {
        let vars = map(&[("amount", json!(100)), ("text", json!("abc"))]);
        assert!(!evaluate_condition("1 / 0 > 1", &vars));
        assert!(!evaluate_condition("1 % 0 == 0", &vars));
        assert!(!evaluate_condition("text * 2 == 0", &vars));
        assert!(!evaluate_condition("amount + text == 0", &vars));
        assert!(!evaluate_condition("text > 1", &vars));
        assert!(!evaluate_condition("-text < 0", &vars));
    }

    #[test]
    fn invalid_expressions_are_rejected() {
        for expr in [
            "a &&",
            "a ||",
            "(a",
            "a == ",
            "a = 1",
            "!",
            "a b",
            "a..b",
            "items[]",
            "items[x]",
            "\"unterminated",
            "'unterminated",
            "a ? b : c",
            "&& a",
            "()",
            "a &&& b",
            "true false",
            "items[1.5]",
            "items[-1]",
            "a.",
            "1 < 2 < 3",
        ] {
            assert!(
                validate_condition(expr).is_err(),
                "expression should be rejected: {expr}"
            );
        }
    }

    #[test]
    fn empty_expressions() {
        for expr in ["", "   ", "\t\n"] {
            assert!(!evaluate_condition(expr, &HashMap::new()));
            assert!(validate_condition(expr).is_err());
        }
        assert!(validate_condition("${}").is_err());
    }

    #[test]
    fn validation_reports_positions() {
        let err = validate_condition("a = 1").expect_err("single '=' must be rejected");
        assert!(err.contains("position 3"), "unexpected error: {err}");

        let err = validate_condition("a == 1 &&").expect_err("dangling operator must be rejected");
        assert!(err.contains("position"), "unexpected error: {err}");

        let err = validate_condition("a && (b == 1").expect_err("missing ')' must be rejected");
        assert!(err.contains("position 13"), "unexpected error: {err}");

        let err =
            validate_condition("a && ${b}").expect_err("mid-expression wrapper must be rejected");
        assert!(err.contains("position 6"), "unexpected error: {err}");
    }

    #[test]
    fn keyword_like_identifiers_are_not_keywords() {
        let vars = map(&[
            ("notify_email", json!(true)),
            ("order", json!(true)),
            ("android", json!(false)),
            ("nullify", json!(true)),
        ]);
        assert!(evaluate_condition("notify_email", &vars));
        assert!(evaluate_condition("order", &vars));
        assert!(!evaluate_condition("android", &vars));
        assert!(evaluate_condition("nullify", &vars));
        assert!(evaluate_condition("notify_email && order", &vars));
        assert!(evaluate_condition("notify_email and order", &vars));
        assert!(!evaluate_condition("android or nullify == false", &vars));
    }
}
