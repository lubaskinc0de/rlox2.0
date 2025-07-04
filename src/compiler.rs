use crate::{
    alias::{StoredChunk, StoredValue, VoidResult},
    chunk::OpCode,
    errors::ParsingError,
    parser::Parser,
    rc_refcell,
    scanner::Scanner,
    token::{Token, TokenType},
    value::Value,
};

use strum_macros::FromRepr;

pub struct Compiler {
    parser: Parser,
    scanner: Scanner,
    current_chunk: Option<StoredChunk>,
    debug_mode: bool,
}

#[derive(Copy, Clone, FromRepr, Debug)]
#[allow(clippy::upper_case_acronyms)]
enum Precedence {
    NONE,
    Assignment,
    Or,
    And,
    Eq,
    Comp,
    Term,
    Factor,
    Unary,
    Call,
    Primary,
}

type ParseFn = fn(&mut Compiler) -> VoidResult;

#[derive(Debug)]
struct ParseRule {
    prefix: Option<ParseFn>,
    infix: Option<ParseFn>,
    precedence: Precedence,
}

use Precedence::*;
const RULES: [ParseRule; 41] = [
    /* TOKEN_LEFT_PAREN */
    ParseRule {
        prefix: Some(Compiler::grouping),
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_RIGHT_PAREN */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_LEFT_BRACE */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_RIGHT_BRACE */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_COMMA */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_DOT */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_MINUS */
    ParseRule {
        prefix: Some(Compiler::unary),
        infix: Some(Compiler::binary),
        precedence: Term,
    },
    /* TOKEN_PLUS */
    ParseRule {
        prefix: None,
        infix: Some(Compiler::binary),
        precedence: Term,
    },
    /* TOKEN_SEMICOLON */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_SLASH */
    ParseRule {
        prefix: None,
        infix: Some(Compiler::binary),
        precedence: Factor,
    },
    /* TOKEN_STAR */
    ParseRule {
        prefix: None,
        infix: Some(Compiler::binary),
        precedence: Factor,
    },
    /* TOKEN_BANG */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_BANG_EQUAL */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_EQUAL */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_EQUAL_EQUAL */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_GREATER */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_GREATER_EQUAL */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_LESS */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_LESS_EQUAL */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_SLASH_EQUAL */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_IDENTIFIER */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_STRING */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_NUMBER */
    ParseRule {
        prefix: Some(Compiler::number),
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_AND */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_CLASS */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_ELSE */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_FALSE */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_FOR */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_FUN */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_IF */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_NIL */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_OR */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_PRINT */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_RETURN */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_SUPER */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_THIS */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_TRUE */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_VAR */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_WHILE */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_ERROR */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
    /* TOKEN_EOF */
    ParseRule {
        prefix: None,
        infix: None,
        precedence: NONE,
    },
];

impl Compiler {
    pub fn from_source(source: String, debug_mode: bool) -> Self {
        let scanner = Scanner::new(source);
        let parser = Parser::new();
        Self {
            parser,
            scanner,
            current_chunk: None,
            debug_mode,
        }
    }

    pub fn compile(&mut self, chunk: StoredChunk) -> VoidResult {
        self.current_chunk = Some(chunk.clone());

        self.advance()?;
        self.expression()?;
        self.consume(TokenType::EOF, "Expected end of expression".to_owned())
    }

    fn previous(&self) -> Option<&Token> {
        self.parser.previous.as_ref()
    }

    fn current(&self) -> Option<&Token> {
        self.parser.current.as_ref()
    }

    fn debug_string(&self) -> String {
        match (self.current(), self.previous()) {
            (None, None) => String::from("current: None, previous: None"),
            (None, Some(prev)) => format!("current: None, previous: {prev}"),
            (Some(curr), None) => format!("current: {curr}, previous: None"),
            (Some(curr), Some(prev)) => {
                format!("current: {curr}, previous: {prev}")
            }
        }
    }

    fn advance(&mut self) -> VoidResult {
        self.parser.previous = self.current().cloned();
        let new_token = self.scanner.scan_token();

        let message: Option<String> = match new_token.token_type {
            TokenType::Error => Some(new_token.message.clone().unwrap()),
            _ => None,
        };

        self.parser.current = Some(new_token);
        if self.debug_mode {
            println!("Called advance(), {}", self.debug_string(),);
        }

        match self.current().unwrap().token_type {
            TokenType::Error => self.error_at_current(message.unwrap()),
            _ => Ok(()),
        }
    }

    fn error_at_current(&self, message: String) -> VoidResult {
        self.error_at(self.current().unwrap(), message)
    }

    fn error(&self, message: String) -> VoidResult {
        self.error_at(self.previous().unwrap(), message)
    }

    fn error_at(&self, token: &Token, message: String) -> VoidResult {
        print!("[line {}] Error", token.line);
        match token.token_type {
            TokenType::EOF => print!(" at end"),
            TokenType::Error => {}
            _ => print!(
                " at '{}'",
                self.scanner.substr(token.start, token.start + token.length)
            ),
        };
        println!(": {message}");
        Err(ParsingError {}.into())
    }

    fn consume(&mut self, token_type: TokenType, message: String) -> VoidResult {
        if self.current().unwrap().token_type == token_type {
            self.advance()
        } else {
            self.error_at_current(message)
        }
    }

    fn emit_op_code(&self, op_code: OpCode) {
        if self.debug_mode {
            println!("Emitted opcode: {op_code}")
        }
        self.current_chunk
            .as_ref()
            .unwrap()
            .borrow_mut()
            .push(op_code);
    }

    fn emit_const(&self, value: StoredValue) {
        self.emit_op_code(OpCode::Const {
            line: self.line(),
            const_idx: self.make_const(value),
        });
    }

    fn make_const(&self, value: StoredValue) -> usize {
        self.current_chunk
            .as_ref()
            .unwrap()
            .borrow_mut()
            .push_const(value)
    }

    fn line(&self) -> usize {
        self.previous().unwrap().line
    }

    fn expression(&mut self) -> VoidResult {
        if self.debug_mode {
            println!("Called expression(), {}", self.debug_string());
        }
        self.parse_precedence(Precedence::Assignment)
    }

    fn number(&mut self) -> VoidResult {
        let value = Value::Float(
            self.previous()
                .unwrap()
                .literal
                .as_ref()
                .unwrap()
                .parse::<f64>()
                .unwrap(),
        );
        if self.debug_mode {
            println!("Called number() for {}", value);
        }
        self.emit_const(rc_refcell!(value));
        Ok(())
    }

    fn grouping(&mut self) -> VoidResult {
        self.expression()?;
        self.consume(TokenType::RightParen, "Expected ')'".to_owned())
    }

    fn unary(&mut self) -> VoidResult {
        let op_type = &self.previous().unwrap().token_type.clone();
        self.parse_precedence(Precedence::Unary)?;

        if self.debug_mode {
            println!("Called unary for op {:?}, {}", op_type, self.debug_string(),)
        }

        if op_type == &TokenType::MINUS {
            self.emit_op_code(OpCode::Negate { line: self.line() })
        }
        Ok(())
    }

    fn next_precedence(&self, variant: Precedence) -> Precedence {
        let code = variant as usize;
        Precedence::from_repr(code + 1).unwrap_or(Precedence::Assignment)
    }

    fn get_rule(&self, token_type: &TokenType) -> &ParseRule {
        let idx = *token_type as usize;

        (RULES.get(idx).unwrap()) as _
    }

    fn binary(&mut self) -> VoidResult {
        let op_type = &self.previous().unwrap().token_type.clone();
        let rule = self.get_rule(op_type);
        let next_precedence = self.next_precedence(rule.precedence);

        if self.debug_mode {
            println!(
                "Called binary {:?}, {}, next precedence = {:?}",
                op_type,
                self.debug_string(),
                next_precedence
            )
        }

        self.parse_precedence(next_precedence)?;

        match op_type {
            TokenType::PLUS => {
                self.emit_op_code(OpCode::Add { line: self.line() });
                Ok(())
            }
            TokenType::MINUS => {
                self.emit_op_code(OpCode::Sub { line: self.line() });
                Ok(())
            }
            TokenType::SLASH => {
                self.emit_op_code(OpCode::Div { line: self.line() });
                Ok(())
            }
            TokenType::STAR => {
                self.emit_op_code(OpCode::Mul { line: self.line() });
                Ok(())
            }
            _ => panic!("Unsupported binary token"),
        }
    }

    fn parse_precedence(&mut self, precedence: Precedence) -> VoidResult {
        if self.debug_mode {
            println!(
                "Called parse_precedence() with precedence = {:?}, {}",
                precedence,
                self.debug_string(),
            )
        }
        self.advance()?;
        let Some(prefix_rule) = self.get_rule(&self.previous().unwrap().token_type).prefix else {
            return self.error("Expected expression".to_owned());
        };

        prefix_rule(self)?;

        let current_token_precedence = self
            .get_rule(&self.current().unwrap().token_type)
            .precedence as usize;

        if !(precedence as usize <= current_token_precedence) {
            println!(
                "Skipping infix rule loop, {}, precedence: {:?}({}), current precedence: {:?}({})",
                self.debug_string(),
                precedence,
                precedence as usize,
                self.get_rule(&self.current().unwrap().token_type)
                    .precedence,
                self.get_rule(&self.current().unwrap().token_type)
                    .precedence as usize,
            );
        }

        let _: () = while (precedence as usize)
            <= (self
                .get_rule(&self.current().unwrap().token_type)
                .precedence as usize)
        {
            if self.debug_mode {
                println!(
                    "Inside infix rule loop, precedence: {:?}({}), current precedence: {:?}({}), {}",
                    precedence,
                    precedence as usize,
                    self.get_rule(&self.current().unwrap().token_type)
                        .precedence,
                    self.get_rule(&self.current().unwrap().token_type)
                        .precedence as usize,
                    self.debug_string()
                )
            }
            self.advance()?;
            let Some(infix_rule) = self.get_rule(&self.previous().unwrap().token_type).infix else {
                continue;
            };
            if self.debug_mode {
                println!("Calling infix rule for {}", self.previous().unwrap())
            }
            infix_rule(self)?;
        };
        Ok(())
    }
}
