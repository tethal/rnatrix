use natrix_compiler::analyze::analyze;
use natrix_compiler::ast::Interpreter as AstInterpreter;
use natrix_compiler::bc::compiler::compile;
use natrix_compiler::ctx::CompilerContext;
use natrix_compiler::error::{AttachErrSpan, SourceResult};
use natrix_compiler::hir::opt::fold_constants;
use natrix_compiler::parser::parse;
use natrix_runtime::bc::Interpreter as BcInterpreter;
use natrix_runtime::ctx::RuntimeContext;
use natrix_runtime::value::Value;
use std::cell::RefCell;
use std::io::Read;
use std::rc::Rc;

enum Mode {
    Ast,
    Bytecode,
}

struct Config {
    mode: Mode,
    input: Input,
    dump_ast: bool,
    dump_hir: bool,
    args: Vec<String>,
}

enum Input {
    Files(Vec<String>),
    Stdin,
}

fn parse_args() -> Result<Config, String> {
    let args: Vec<String> = std::env::args().collect();

    let mut mode = Mode::Bytecode;
    let mut filenames = Vec::new();
    let mut dump_ast = false;
    let mut dump_hir = false;
    let mut program_args = Vec::new();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--ast" => mode = Mode::Ast,
            "--bc" => mode = Mode::Bytecode,
            "--dump-ast" => dump_ast = true,
            "--dump-hir" => dump_hir = true,
            "--" => {
                // Everything after -- goes to program args
                program_args.extend_from_slice(&args[i + 1..]);
                break;
            }
            arg if arg.starts_with("--") => {
                return Err(format!("Unknown option: {}", arg));
            }
            arg => {
                filenames.push(arg.to_string());
            }
        }
        i += 1;
    }

    let input = if filenames.is_empty() {
        Input::Stdin
    } else {
        Input::Files(filenames)
    };

    Ok(Config {
        mode,
        input,
        dump_ast,
        dump_hir,
        args: program_args,
    })
}

fn run(ctx: &mut CompilerContext, config: Config) -> SourceResult<()> {
    // Parse sources
    let ast = match config.input {
        Input::Files(paths) => {
            // Parse first file
            let source_id = ctx
                .sources
                .add_from_file(&paths[0])
                .expect("Unable to load source file");
            let mut program = parse(ctx, source_id)?;

            // Append remaining files
            for path in &paths[1..] {
                let source_id = ctx
                    .sources
                    .add_from_file(path)
                    .expect("Unable to load source file");
                let mut ast = parse(ctx, source_id)?;
                program.decls.append(&mut ast.decls);
            }

            program
        }
        Input::Stdin => {
            let mut buffer = String::new();
            std::io::stdin()
                .read_to_string(&mut buffer)
                .expect("Unable to read from stdin");
            let source_id = ctx.sources.add_from_string(&buffer);
            parse(ctx, source_id)?
        }
    };

    // Dump AST
    if config.dump_ast {
        println!("{:?}", ast.debug_with(&ctx));
    }

    // Prepare arguments
    let args = Value::from_list(Rc::new(RefCell::new(
        config
            .args
            .iter()
            .map(|a| Value::from_string(a.as_str().into()))
            .collect(),
    )));

    // Execute
    let mut rt = RuntimeContext::new();
    let result = match config.mode {
        Mode::Ast => {
            let mut interpreter = AstInterpreter::new(&ctx, &mut rt);
            interpreter.run(ast, vec![args])?
        }
        Mode::Bytecode => {
            let mut hir = analyze(&ctx, &ast)?;
            fold_constants(&mut hir)?;
            if config.dump_hir {
                println!("{:?}", hir.debug_with(&ctx));
            }

            let bc = compile(ctx, &hir)?;
            let mut interpreter = BcInterpreter::new(&mut rt);
            interpreter.run(&bc, vec![args]).err_at(hir.span)?
        }
    };
    if !result.is_null() {
        println!("{}", result);
    }
    Ok(())
}

fn main() {
    let config = match parse_args() {
        Ok(config) => config,
        Err(msg) => {
            eprintln!("Error: {}", msg);
            eprintln!();
            eprintln!("Usage: natrix [OPTIONS] [FILE...] [-- args]");
            eprintln!();
            eprintln!("Options:");
            eprintln!("  --ast        Use AST interpreter (default: bytecode)");
            eprintln!("  --bc         Use bytecode interpreter");
            eprintln!("  --dump-ast   Print AST after parsing");
            eprintln!("  --dump-hir   Print HIR after analysis (bytecode mode only)");
            eprintln!();
            eprintln!("If no FILE is not provided, reads from stdin.");
            std::process::exit(1);
        }
    };

    let mut ctx = CompilerContext::default();
    if let Err(err) = run(&mut ctx, config) {
        println!("{}", err.display_with(&ctx.sources));
        std::process::exit(1);
    }
}
