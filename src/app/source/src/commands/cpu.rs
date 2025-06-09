use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;

// all the instructions our tiny cpu understands
#[derive(Debug, Clone, Copy)]
pub enum Instruction {
    Push(i32),     // push value onto stack
    Pop,           // remove top value
    Add,           // add two values
    Sub,           // subtract (a-b where b is top)
    Mul,           // multiply two values
    Div,           // divide (a/b where b is top)
    Mod,           // modulo (a%b where b is top)
    Dup,           // duplicate top value
    Swap,          // swap top two values
    Load(usize),   // load from memory address
    Store(usize),  // store to memory address
    Jump(usize),   // unconditional jump
    JumpIf(usize), // jump if top != 0
    JumpIfZ(usize), // jump if top == 0
    Cmp,           // compare: 1 if a>b, 0 if a==b, -1 if a<b
    Print,         // print top as number
    PrintChar,     // print top as ascii char
    Read,          // read int from input (stubbed for now)
    Halt,          // stop execution
}

pub struct CpuCommand;

impl Command for CpuCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        match args.get(0).map(|s| s.as_str()) {
            Some("run") => {
                // run an assembly file through our vm
                if let Some(filename) = args.get(1) {
                    // handle relative/absolute paths
                    let path = if filename.starts_with('/') {
                        filename.to_string()
                    } else {
                        format!("{}/{}", ctx.cwd, filename)
                    };
                    
                    // read and parse the assembly file
                    let file_content = ctx.vfs.read_file(&path)
                        .map_err(|e| format!("Error reading file: {}", e))?;
                    
                    let content = String::from_utf8(file_content.to_vec())
                        .map_err(|_| "File contains invalid UTF-8".to_string())?;
                    
                    // assemble source to bytecode
                    let program = assemble(&content)
                        .map_err(|e| format!("Assembly error: {}", e))?;
                    
                    // run it and return output
                    Ok(run(&program))
                } else {
                    Err("Usage: cpu run <filename>".to_string())
                }
            },
            Some("new") => {
                // create new assembly file with basic template
                if let Some(filename) = args.get(1) {
                    let path = if filename.starts_with('/') {
                        filename.to_string()
                    } else {
                        format!("{}/{}", ctx.cwd, filename)
                    };
                    
                    // basic template to get people started
                    let template = "# Sample Assembly Program\n\
                                   # Use 'cpu run <filename>' to execute\n\
                                   \n\
                                   # Push values onto stack\n\
                                   push 10\n\
                                   push 20\n\
                                   \n\
                                   # Add them\n\
                                   add\n\
                                   \n\
                                   # Print result\n\
                                   print\n\
                                   \n\
                                   # Exit program\n\
                                   halt\n";
                    
                    ctx.vfs.create_file(&path, template.as_bytes().to_vec())
                        .map_err(|e| format!("Error creating file: {}", e))?;
                    
                    Ok(format!("Created new assembly file: {}", filename))
                } else {
                    Err("Usage: cpu new <filename>".to_string())
                }
            },
            Some("help") => {
                // quick reference for all instructions
                Ok(String::from(
                    "CPU Assembly Language Help:\n\
                     Commands:\n\
                     - cpu run <filename>  : Run an assembly program\n\
                     - cpu new <filename>  : Create a new assembly program\n\
                     - cpu help            : Show this help\n\
                     - cpu docs            : Show assembly language documentation\n\
                     \n\
                     Assembly Instructions:\n\
                     - push <n>     : Push value onto stack\n\
                     - pop          : Remove top value from stack\n\
                     - add          : Add top two values\n\
                     - sub          : Subtract (a-b where b is top of stack)\n\
                     - mul          : Multiply top two values\n\
                     - div          : Divide (a/b where b is top of stack)\n\
                     - mod          : Modulo (a%b where b is top of stack)\n\
                     - dup          : Duplicate top value\n\
                     - swap         : Swap top two values\n\
                     - load <addr>  : Load value from memory address\n\
                     - store <addr> : Store value to memory address\n\
                     - jump <addr>  : Jump to instruction address\n\
                     - jumpif <addr>: Jump if top of stack is non-zero\n\
                     - jumpifz <addr>: Jump if top of stack is zero\n\
                     - cmp          : Compare top two values (pushes 1 if a>b, 0 if a==b, -1 if a<b)\n\
                     - print        : Print top value as number\n\
                     - printchar    : Print top value as ASCII character\n\
                     - read         : Read integer from input\n\
                     - halt         : Stop execution"
                ))
            },
            Some("docs") => {
                // more detailed docs with examples
                Ok(String::from(
                    "CPU Assembly Language Documentation\n\
                     ===============================\n\
                     \n\
                     This is a simple stack-based assembly language. Programs operate on a stack\n\
                     and have access to memory for storing values.\n\
                     \n\
                     Example Programs:\n\
                     \n\
                     1. Calculate the factorial of 5:\n\
                     ```\n\
                     # Initialize result to 1\n\
                     push 1\n\
                     # Initialize counter to 5\n\
                     push 5\n\
                     # Start of loop\n\
                     # Duplicate counter\n\
                     dup\n\
                     # Multiply result by counter\n\
                     mul\n\
                     # Decrement counter\n\
                     push 1\n\
                     swap\n\
                     sub\n\
                     # Duplicate counter to check if we're done\n\
                     dup\n\
                     # If counter > 0, jump back to start of loop\n\
                     push 3\n\
                     jumpif\n\
                     # Remove counter from stack\n\
                     pop\n\
                     # Print result\n\
                     print\n\
                     halt\n\
                     ```\n\
                     \n\
                     2. Print 'Hello, World!':\n\
                     ```\n\
                     push 72  # H\n\
                     printchar\n\
                     push 101 # e\n\
                     printchar\n\
                     push 108 # l\n\
                     printchar\n\
                     push 108 # l\n\
                     printchar\n\
                     push 111 # o\n\
                     printchar\n\
                     push 44  # ,\n\
                     printchar\n\
                     push 32  # space\n\
                     printchar\n\
                     push 87  # W\n\
                     printchar\n\
                     push 111 # o\n\
                     printchar\n\
                     push 114 # r\n\
                     printchar\n\
                     push 108 # l\n\
                     printchar\n\
                     push 100 # d\n\
                     printchar\n\
                     push 33  # !\n\
                     printchar\n\
                     halt\n\
                     ```"
                ))
            },
            _ => {
                Err("Usage: cpu [run|new|help|docs]".to_string())
            }
        }
    }
}

// two-pass assembler: collect labels first, then parse instructions
pub fn assemble(src: &str) -> Result<Vec<Instruction>, String> {
    let mut program = Vec::new();
    
    // first pass - find all labels and their positions
    let mut labels = std::collections::HashMap::new();
    let mut cleaned_lines = Vec::new();
    
    for (i, line) in src.lines().enumerate() {
        let line = line.trim();
        // skip empty lines and comments
        if line.is_empty() || line.starts_with('#') { continue; }
        
        // check for label definitions (name:)
        if let Some(label_end) = line.find(':') {
            let label = line[..label_end].trim();
            labels.insert(label.to_string(), cleaned_lines.len());
            
            // if there's an instruction after the label, keep it
            if line.len() > label_end + 1 {
                let instruction = line[label_end+1..].trim();
                if !instruction.is_empty() {
                    cleaned_lines.push((i, instruction.to_string()));
                }
            }
        } else {
            cleaned_lines.push((i, line.to_string()));
        }
    }
    
    // second pass - parse instructions and resolve label references
    for (i, line) in cleaned_lines {
        let parts: Vec<_> = line.split_whitespace().collect();
        match parts.as_slice() {
            ["push", n] => {
                let val = n.parse().map_err(|_| format!("Invalid number at line {}", i+1))?;
                program.push(Instruction::Push(val));
            }
            ["pop"] => program.push(Instruction::Pop),
            ["add"] => program.push(Instruction::Add),
            ["sub"] => program.push(Instruction::Sub),
            ["mul"] => program.push(Instruction::Mul),
            ["div"] => program.push(Instruction::Div),
            ["mod"] => program.push(Instruction::Mod),
            ["dup"] => program.push(Instruction::Dup),
            ["swap"] => program.push(Instruction::Swap),
            ["load", addr] => {
                let addr = parse_address(addr, &labels, i)
                    .map_err(|e| format!("Invalid address at line {}: {}", i+1, e))?;
                program.push(Instruction::Load(addr));
            }
            ["store", addr] => {
                let addr = parse_address(addr, &labels, i)
                    .map_err(|e| format!("Invalid address at line {}: {}", i+1, e))?;
                program.push(Instruction::Store(addr));
            }
            ["jump", target] => {
                let addr = parse_address(target, &labels, i)
                    .map_err(|e| format!("Invalid jump target at line {}: {}", i+1, e))?;
                program.push(Instruction::Jump(addr));
            }
            ["jumpif", target] => {
                let addr = parse_address(target, &labels, i)
                    .map_err(|e| format!("Invalid jump target at line {}: {}", i+1, e))?;
                program.push(Instruction::JumpIf(addr));
            }
            ["jumpifz", target] => {
                let addr = parse_address(target, &labels, i)
                    .map_err(|e| format!("Invalid jump target at line {}: {}", i+1, e))?;
                program.push(Instruction::JumpIfZ(addr));
            }
            ["cmp"] => program.push(Instruction::Cmp),
            ["print"] => program.push(Instruction::Print),
            ["printchar"] => program.push(Instruction::PrintChar),
            ["read"] => program.push(Instruction::Read),
            ["halt"] => program.push(Instruction::Halt),
            _ => return Err(format!("Unknown instruction at line {}: {}", i+1, line)),
        }
    }
    Ok(program)
}

// resolve address - could be label name or numeric address
fn parse_address(addr: &str, labels: &std::collections::HashMap<String, usize>, line: usize) 
    -> Result<usize, String> {
    // try label lookup first
    if let Some(&addr) = labels.get(addr) {
        return Ok(addr);
    }
    // fallback to parsing as number
    addr.parse().map_err(|_| format!("Invalid address at line {}", line+1))
}

// virtual machine executor - runs the compiled program
pub fn run(program: &[Instruction]) -> String {
    let mut stack = Vec::new();
    let mut memory = vec![0; 1024]; // 1kb memory - should be plenty
    let mut output = String::new();
    let mut pc = 0; // program counter
    
    // main execution loop
    while pc < program.len() {
        match program[pc] {
            Instruction::Push(n) => stack.push(n),
            Instruction::Pop => { stack.pop(); },
            Instruction::Add => {
                if let (Some(b), Some(a)) = (stack.pop(), stack.pop()) {
                    stack.push(a + b);
                }
            }
            Instruction::Sub => {
                if let (Some(b), Some(a)) = (stack.pop(), stack.pop()) {
                    stack.push(a - b);
                }
            }
            Instruction::Mul => {
                if let (Some(b), Some(a)) = (stack.pop(), stack.pop()) {
                    stack.push(a * b);
                }
            }
            Instruction::Div => {
                if let (Some(b), Some(a)) = (stack.pop(), stack.pop()) {
                    if b == 0 {
                        output.push_str("Error: Division by zero\n");
                        break;
                    }
                    stack.push(a / b);
                }
            }
            Instruction::Mod => {
                if let (Some(b), Some(a)) = (stack.pop(), stack.pop()) {
                    if b == 0 {
                        output.push_str("Error: Modulo by zero\n");
                        break;
                    }
                    stack.push(a % b);
                }
            }
            Instruction::Dup => {
                if let Some(&a) = stack.last() {
                    stack.push(a);
                }
            }
            Instruction::Swap => {
                let len = stack.len();
                if len >= 2 {
                    stack.swap(len - 1, len - 2);
                }
            }
            Instruction::Load(addr) => {
                if addr < memory.len() {
                    stack.push(memory[addr]);
                } else {
                    output.push_str(&format!("Error: Memory access out of bounds: {}\n", addr));
                    break;
                }
            }
            Instruction::Store(addr) => {
                if let Some(val) = stack.pop() {
                    if addr < memory.len() {
                        memory[addr] = val;
                    } else {
                        output.push_str(&format!("Error: Memory access out of bounds: {}\n", addr));
                        break;
                    }
                }
            }
            Instruction::Jump(addr) => {
                if addr < program.len() {
                    pc = addr;
                    continue; // skip pc increment
                } else {
                    output.push_str(&format!("Error: Jump target out of bounds: {}\n", addr));
                    break;
                }
            }
            Instruction::JumpIf(addr) => {
                if let Some(val) = stack.pop() {
                    if val != 0 {
                        if addr < program.len() {
                            pc = addr;
                            continue; // skip pc increment
                        } else {
                            output.push_str(&format!("Error: Jump target out of bounds: {}\n", addr));
                            break;
                        }
                    }
                }
            }
            Instruction::JumpIfZ(addr) => {
                if let Some(val) = stack.pop() {
                    if val == 0 {
                        if addr < program.len() {
                            pc = addr;
                            continue; // skip pc increment
                        } else {
                            output.push_str(&format!("Error: Jump target out of bounds: {}\n", addr));
                            break;
                        }
                    }
                }
            }
            Instruction::Cmp => {
                if let (Some(b), Some(a)) = (stack.pop(), stack.pop()) {
                    if a > b {
                        stack.push(1);
                    } else if a == b {
                        stack.push(0);
                    } else {
                        stack.push(-1);
                    }
                }
            }
            Instruction::Print => {
                if let Some(val) = stack.last() {
                    output.push_str(&format!("{}\n", val));
                }
            }
            Instruction::PrintChar => {
                if let Some(val) = stack.pop() {
                    if val >= 0 && val <= 127 {
                        output.push(char::from_u32(val as u32).unwrap_or('?'));
                    } else {
                        output.push('?'); // invalid ascii
                    }
                }
            }
            Instruction::Read => {
                // would need browser integration for real input
                // just push 0 for now
                stack.push(0);
            }
            Instruction::Halt => break,
        }
        pc += 1;
    }
    
    if !output.is_empty() {
        output
    } else {
        // if program didn't output anything, show final stack state
        format!("Final stack: {:?}\n", stack)
    }
}
