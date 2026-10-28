pub mod ast;
pub mod checker;
pub mod diag;
pub mod env;
pub mod interp;
pub mod lexer;
pub mod parser;
pub mod span;
pub mod token;
pub mod types;
pub mod value;

use std::env::args;
use std::fs;
use std::process;

use checker::TypeChecker;
use diag::{
    Diagnostic, Stage, EXIT_COMPILE_ERROR, EXIT_INTERNAL_ERROR, EXIT_IO_ERROR, EXIT_RUNTIME_ERROR,
    EXIT_SUCCESS,
};
use interp::Interpreter;
use lexer::Lexer;
use parser::Parser;
use span::Span;

fn main() {
    let args: Vec<String> = args().collect();
    if args.len() != 2 {
        eprintln!("usage: esttro <file.est>");
        process::exit(EXIT_IO_ERROR);
    }

    let file_path = &args[1];
    let source = read_source(file_path);
    let checked_ast = compile(file_path, &source);
    execute(file_path, checked_ast);
    process::exit(EXIT_SUCCESS);
}

fn read_source(file_path: &str) -> String {
    match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(err) => {
            let diag = Diagnostic::new(
                file_path,
                Span::new(0, 0, 1, 1),
                Stage::Io,
                format!("failed to read file: {}", err),
            );
            eprintln!("{}", diag);
            process::exit(EXIT_IO_ERROR);
        }
    }
}

fn compile(file_path: &str, source: &str) -> ast::CheckedProgram {
    let tokens = match Lexer::new(file_path, source).tokenize() {
        Ok(t) => t,
        Err(diags) => emit_and_exit(diags),
    };

    let ast = match Parser::new(file_path, tokens).parse() {
        Ok(a) => a,
        Err(diags) => emit_and_exit(diags),
    };

    match TypeChecker::new(file_path).check(ast) {
        Ok(checked) => checked,
        Err(diags) => emit_and_exit(diags),
    }
}

fn emit_and_exit(diags: Vec<Diagnostic>) -> ! {
    for d in diags {
        eprintln!("{}", d);
    }
    process::exit(EXIT_COMPILE_ERROR);
}

fn execute(file_path: &str, checked_ast: ast::CheckedProgram) {
    let file_path_str = file_path.to_string();
    let runner = std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            let mut interpreter = Interpreter::new(&file_path_str);
            if let Err(diag) = interpreter.run(checked_ast) {
                eprintln!("{}", diag);
                process::exit(EXIT_RUNTIME_ERROR);
            }
        });

    let ok = match runner {
        Ok(handle) => handle.join().is_ok(),
        Err(_) => false,
    };
    if !ok {
        process::exit(EXIT_INTERNAL_ERROR);
    }
}
