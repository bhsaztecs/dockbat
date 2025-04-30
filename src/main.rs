use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[derive(Clone, Debug)]
struct Inputs {
    warnings: Vec<String>,
    infiles: Vec<PathBuf>,
    libs: Vec<String>,
    outfile: PathBuf,
}

impl Inputs {
    fn builder() -> InputsBuilder {
        InputsBuilder::new()
    }

    fn to_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        args.extend(self.warnings.iter().map(|w| format!("-W{}", w)));

        for infile in &self.infiles {
            let path_str = infile.to_string_lossy();

            // Handle wildcards by expanding them
            if path_str.ends_with("/*") {
                let dir_path = path_str.trim_end_matches("/*");

                // Add directory as include path
                args.push(format!("-I{}", dir_path));

                // Add all source files from this directory
                if let Ok(entries) = std::fs::read_dir(dir_path) {
                    for entry in entries.filter_map(Result::ok) {
                        let path = entry.path();
                        if path.is_file() {
                            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                            if ext == "c" || ext == "cpp" {
                                args.push(path.display().to_string());
                            }
                        }
                    }
                }
            } else if path_str.ends_with(".h") || path_str.ends_with(".hpp") {
                args.push(format!("-I{}", infile.display()));
            } else if path_str.ends_with(".c") || path_str.ends_with(".cpp") {
                args.push(infile.display().to_string());
            }
        }

        args.extend(self.libs.iter().map(|l| format!("-l{}", l)));

        args.push("-o".to_string());
        args.push(self.outfile.display().to_string());

        args
    }
}

#[derive(Default)]
struct InputsBuilder {
    infiles: Option<Vec<PathBuf>>,
    libs: Option<Vec<String>>,
    warnings: Option<Vec<String>>,
    outfile: Option<PathBuf>,
}

impl InputsBuilder {
    fn new() -> Self {
        Self::default()
    }

    fn inputfiles<P: AsRef<Path>>(mut self, inputfiles: Vec<P>) -> Self {
        self.infiles = Some(
            inputfiles
                .into_iter()
                .map(|p| p.as_ref().to_path_buf())
                .collect(),
        );
        self
    }

    fn libraries<S: AsRef<str>>(mut self, libraries: Vec<S>) -> Self {
        self.libs = Some(
            libraries
                .into_iter()
                .map(|s| s.as_ref().to_string())
                .collect(),
        );
        self
    }

    fn warnings<S: AsRef<str>>(mut self, warnings: Vec<S>) -> Self {
        self.warnings = Some(
            warnings
                .into_iter()
                .map(|s| s.as_ref().to_string())
                .collect(),
        );
        self
    }

    fn output<P: AsRef<Path>>(mut self, file: P) -> Self {
        self.outfile = Some(file.as_ref().to_path_buf());
        self
    }

    fn build(self) -> Result<Inputs, &'static str> {
        Ok(Inputs {
            infiles: self.infiles.unwrap_or_default(),
            libs: self.libs.unwrap_or_default(),
            warnings: self.warnings.unwrap_or_default(),
            outfile: self.outfile.ok_or("Output file is required")?,
        })
    }
}

fn find_empty_file() -> io::Result<PathBuf> {
    let bin_dir = PathBuf::from("bin");
    let current = "out";
    let mut i = 0;

    loop {
        let file_path = bin_dir.join(format!("{}{}", current, i));
        if !file_path.exists() {
            return Ok(PathBuf::from(format!("{}{}", current, i)));
        }
        i += 1;
    }
}

fn run_command(mut command: Command) -> Result<Output, Box<dyn std::error::Error>> {
    let output = command.output()?;

    // Always process and print stdout and stderr
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Format and print compiler output
    if !stdout.is_empty() {
        let formatted = format_compiler_output(&stdout);
        print!("{}", formatted);
    }

    if !stderr.is_empty() {
        let formatted = format_compiler_output(&stderr);
        eprint!("{}", formatted);
    }

    if !output.status.success() {
        return Err("Command failed - see errors above".into());
    }

    Ok(output)
}

fn format_compiler_output(output: &str) -> String {
    // Replace escaped newlines
    let with_newlines = output.replace("\\n", "\n");

    // Add colors
    const RED: &str = "\x1b[31m";
    const YELLOW: &str = "\x1b[33m";
    const RESET: &str = "\x1b[0m";

    let colored = with_newlines
        .replace("error:", &format!("{}error:{}", RED, RESET))
        .replace("warning:", &format!("{}warning:{}", YELLOW, RESET));

    colored
}
fn initialize(save: bool) -> Result<(), Box<dyn std::error::Error>> {
    if save {
        let mut cmd = Command::new("git");
        cmd.arg("init");
        run_command(cmd)?;

        let mut cmd = Command::new("git");
        cmd.arg("add").arg(".");
        run_command(cmd)?;

        let mut cmd = Command::new("git");
        cmd.args(&["commit", "-m", "pre-initialize"]);
        run_command(cmd)?;
    }

    fs::create_dir_all("bin")?;
    fs::create_dir_all("src")?;

    let botball_path = PathBuf::from("bin").join("botball_user_program");
    File::create(&botball_path)?;

    let mut cmd = Command::new("docker");
    cmd.args(&["pull", "sillyfreak/wombat-cross"]);
    run_command(cmd)?;

    let src_path = PathBuf::from("src").join("main.cpp");
    let example_path = PathBuf::from("data").join("example.cpp");

    if example_path.exists() {
        let mut content = String::new();
        File::open(&example_path)?.read_to_string(&mut content)?;
        File::create(&src_path)?.write_all(content.as_bytes())?;
    } else {
        return Err("example.cpp not found in data directory".into());
    }

    println!("Run 'compile --nocopy executable' to validate.");
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum CompileType {
    Executable,
    Library,
    IndependentExecutable,
}

impl CompileType {
    fn to_args(self) -> Vec<String> {
        match self {
            CompileType::Executable => vec![],
            CompileType::Library => vec!["-fPIC".into(), "-shared".into()],
            CompileType::IndependentExecutable => vec!["-leden".into()],
        }
    }
}

fn compile(
    inputs: Inputs,
    ctype: CompileType,
    debug: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut args = vec![
        "docker",
        "run",
        "--rm",
        "--volume",
        "./develop:/home/kipr:rw",
        "sillyfreak/wombat-cross",
        "aarch64-linux-gnu-g++",
        "-std=c++17",
        "-I./include/*",
        "-I/usr/local/include/include/",
        "-L/usr/local/lib",
    ]
    .into_iter()
    .map(String::from)
    .collect::<Vec<_>>();

    if debug {
        args.push("-g".into());
    }
    args.extend(inputs.to_args());
    args.extend(ctype.to_args());
    let mut cmd = Command::new("sudo");
    cmd.args(&args);
    run_command(cmd)?;
    Ok(())
}

fn copy_files(to: &Path, from: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut ping_command = Command::new("ping");
    ping_command.args(&["-c", "1", "kipr@192.168.125.1"]);
    run_command(ping_command)?;

    let mut scp_command = Command::new("scp");
    scp_command
        .args(&["-r", "-q"])
        .arg(from)
        .arg(format!("kipr@192.168.125.1:{}", to.display()));
    run_command(scp_command)?;

    Ok(())
}

fn shell() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::new("ssh");
    cmd.arg("kipr@192.168.125.1");
    run_command(cmd)?;
    Ok(())
}
fn todev(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        if entry.path().to_string_lossy().contains("target")
            || entry.path().to_string_lossy().contains("git")
            || entry.path().to_string_lossy().contains("develop")
        {
            continue;
        }
        let ty = entry.file_type()?;
        if ty.is_dir() {
            todev(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn libsnwarns(args: &Vec<String>) -> (Vec<String>, Vec<String>) {
    let librariesandwarnings = args[3..].to_vec();
    let mut libraries = Vec::new();
    let mut warnings = Vec::new();
    let mut in_lib = false;
    let mut in_warn = false;

    for arg in librariesandwarnings {
        match arg.as_str() {
            "-l" => {
                in_lib = true;
                in_warn = false;
            }
            "-w" => {
                in_lib = false;
                in_warn = true;
            }
            arg => {
                if in_lib {
                    libraries.push(arg.to_string());
                } else if in_warn {
                    warnings.push(arg.to_string());
                }
            }
        }
    }

    if libraries.is_empty() {
        libraries = vec!["m".into(), "pthread".into(), "kipr".into(), "z".into()];
    }

    if warnings == vec!["actuallyall"] {
        warnings = vec![
            "all",
            "extra",
            "cast-align",
            "cast-qual",
            "ctor-dtor-privacy",
            "disabled-optimization",
            "format=2",
            "init-self",
            "logical-op",
            "missing-declarations",
            "missing-include-dirs",
            "noexcept",
            "old-style-cast",
            "overloaded-virtual",
            "redundant-decls",
            "shadow",
            "sign-conversion",
            "sign-promo",
            "strict-null-sentinel",
            "strict-overflow=5",
            "switch-default",
            "undef",
            "error",
            "no-unused",
        ]
        .iter()
        .map(|x| x.to_string())
        .collect();
    }
    (libraries, warnings)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<String> = env::args().collect();
    match args.get(1).ok_or("No command provided")?.as_str() {
        "compile" => {
            fs::create_dir_all("./develop/").expect(&format!("{}", line!()));
            fs::create_dir_all("./bin/").expect(&format!("{}", line!()));
            let nosave;
            let output = if args.contains(&"--nosave".to_string()) {
                args.retain(|x| x != "--nosave");
                nosave = true;
                find_empty_file()
                    .expect(&format!("{}", line!()))
                    .to_string_lossy()
                    .into_owned()
            } else {
                nosave = false;
                "botball_user_program".to_string()
            };

            File::create(format!("./bin/{}", output)).expect(&format!("{}", line!()));
            todev(".", "./develop/").expect(&format!("{}", line!()));

            let compile_type = match args.get(2).map(String::as_str) {
                Some("library") => CompileType::Library,
                Some("indexe") => CompileType::IndependentExecutable,
                Some("executable") => CompileType::Executable,
                _ => return Err("Invalid compile type.".into()),
            };

            let debug = if args.contains(&"--debug".to_string()) {
                args.retain(|x| x != "--debug");
                true
            } else {
                false
            };
            let (libraries, warnings) = libsnwarns(&args);

            let mut inputfiles: Vec<String> = Vec::new();
            inputfiles.push("./include/*".to_string());

            match compile_type {
                CompileType::Executable => inputfiles.push("./src/*".to_string()),
                CompileType::Library => {
                    for file in fs::read_dir("./src").expect(&format!("{}", line!())) {
                        let file = file.expect(&format!("{}", line!()));
                        let path = file.path();
                        if !path.to_string_lossy().contains("main") {
                            inputfiles.push(path.display().to_string());
                        }
                    }
                }
                CompileType::IndependentExecutable => inputfiles.push("./src/*".to_string()),
            }

            let inputs = Inputs::builder()
                .inputfiles(inputfiles)
                .output(Path::new("./bin/").join(&output))
                .libraries(libraries)
                .warnings(warnings)
                .build()
                .expect(&format!("{}", line!()));
            let compile_result = compile(inputs, compile_type, debug);
            let copy_result = fs::copy(
                format!("./develop/bin/{}", output),
                format!("./bin/{}", output),
            );

            if nosave {
                let _ = fs::remove_file(Path::new(&format!("./bin/{}", output)[..]));
            }
            let _ = fs::remove_dir_all("develop");

            match (compile_result, copy_result) {
                (Err(_), _) => Err("compile failed".into()),
                (_, Err(copy)) => Err(copy.into()),
                _ => Ok(()),
            }
        }
        "copy" => {
            let dest = Path::new(args.get(2).ok_or("Destination path required")?);
            let current_dir = env::current_dir()?;
            let src = args
                .get(3)
                .map(Path::new)
                .unwrap_or_else(|| current_dir.as_path());
            copy_files(dest, src)
        }
        "shell" => shell(),
        "initialize" => {
            println!("Save before initializing? (y/n)");
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            initialize(
                input
                    .trim()
                    .to_lowercase()
                    .chars()
                    .next()
                    .ok_or("Could not read input")?
                    == 'y',
            )
        }
        _ => match args.get(2) {
            Some(cmd) => match &cmd[..] {
                "initialize" => {
                    println!(
                        "run {} {}, you'll be prompted to save your changes.",
                        args.get(0).unwrap(),
                        "initialize"
                    );
                    Ok(())
                }
                "compile" => {
                    println!(
                        "usage: {} compile 1 2 ... 3 ... 4 ...
	1: {{executable | library | indexe}}: compile type
	2: (--nosave) and or (--debug): save the output, use debugging symbols
	3: -l {{list of libraries to link}}
	4: -w {{list of warnings to use}}
	3 & 4 can be swapped",
                        args.get(0).unwrap()
                    );
                    Ok(())
                }
                "copy" => {
                    println!(
                        "usage: {} copy WOMBATFOLDER LOCALFOLDER\n
					eg: {} copy Default/Eden $(pwd)",
                        args.get(0).unwrap(),
                        args.get(0).unwrap()
                    );
                    Ok(())
                }
                "shell" => {
                    println!("shell into wombat");
                    Ok(())
                }
                _ => Err("Invalid Command".into()),
            },
            _ => Ok(println!("options: initialize, compile, copy, shell")),
        },
    }
}
