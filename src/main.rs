use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::Command;
struct Inputs {
    warnings: Vec<String>,
    infiles: Vec<std::path::PathBuf>,
    libs: Vec<String>,
    outfile: std::path::PathBuf,
}
impl Inputs {
    fn builder() -> InputsBuilder {
        InputsBuilder::new()
    }
    fn to_args(self) -> Vec<String> {
        let mut args = Vec::new();

        args.extend(self.warnings.iter().map(|w| format!("-W{}", w)));

        for infile in &self.infiles {
            let path_str = infile.to_string_lossy();
            if path_str.ends_with(".h") || path_str.ends_with(".hpp") {
                args.push(format!("-I{}", infile.display()));
            } else if path_str.ends_with(".c") || path_str.ends_with(".cpp") {
                args.push(infile.display().to_string());
            }
        }

        args.extend(self.libs.iter().map(|l| format!("-l{}", l)));
        args.push(format!("-o {}", self.outfile.display()));
        args
    }
}

#[derive(Default)]
struct InputsBuilder {
    infiles: Option<Vec<std::path::PathBuf>>,
    libs: Option<Vec<String>>,
    warnings: Option<Vec<String>>,
    outfile: Option<std::path::PathBuf>,
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

fn find_empty_file() -> io::Result<String> {
    let bin_dir = Path::new("");
    let current = "out";
    let mut i = 0;

    loop {
        let file_path = bin_dir.join(format!("{}{}", current, i));
        if !file_path.exists() {
            return Ok(file_path.to_string_lossy().into_owned());
        }
        i += 1;
    }
}
fn initialize(save: bool) -> Result<(), Box<dyn std::error::Error>> {
    if save {
        let status = Command::new("git").arg("init").status()?;
        if !status.success() {
            return Err("Failed to initialize git repository".into());
        }

        let status = Command::new("git").arg("add").arg(".").status()?;
        if !status.success() {
            return Err("Failed to add files to git".into());
        }

        let status = Command::new("git")
            .args(&["commit", "-m", "pre-initialize"])
            .status()?;
        if !status.success() {
            return Err("Failed to commit files".into());
        }
    }
    fs::create_dir_all("bin")?;
    let botball_path = Path::new("bin").join("botball_user_program");
    File::create(&botball_path)?;
    let docker_status = Command::new("docker")
        .args(&["pull", "sillyfreak/wombat-cross"])
        .status()?;
    if !docker_status.success() {
        return Err("Failed to pull Docker image".into());
    }
    fs::create_dir_all("src")?;
    let src_path = Path::new("src").join("main.cpp");
    let example_path = Path::new("data").join("example.cpp");
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
            CompileType::IndependentExecutable => {
                vec!["-leden".into()]
            }
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
        "-it",
        "--rm",
        "--volume",
        ".develop:/home/kipr:rw",
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
    print!("sudo ");
    for arg in &args {
        print!("{} ", arg);
    }
    println!("");
    let status = Command::new("sudo").args(&args).status()?;
    if !status.success() {
        return Err("Compilation failed".into());
    }
    Ok(())
}
fn copy_files(to: &Path, from: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("ping -c 1 kipr@192.168.125.1");
    println!(
        "scp -r -q {} kipr@192.168.125.1:{}",
        from.display(),
        to.display()
    );
    let status = Command::new("ping")
        .arg("-c")
        .arg("1")
        .arg("kipr@192.168.125.1")
        .status()?;
    if !status.success() {
        return Err("Ping failed".into());
    }
    let status = Command::new("scp")
        .args(&["-r", "-q"])
        .arg(from)
        .arg(format!("kipr@192.168.125.1:{}", to.display()))
        .status()?;
    if !status.success() {
        return Err("SCP failed".into());
    }
    Ok(())
}
fn shell() -> Result<(), Box<dyn std::error::Error>> {
    println!("ssh kipr@192.168.125.1");
    let status = Command::new("ssh").arg("kipr@192.168.125.1").status()?;
    if !status.success() {
        return Err("SSH failed".into());
    }
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<String> = env::args().collect();
    match args.get(1).ok_or("No command provided")?.as_str() {
        "compile" => {
            fs::create_dir_all("develop")?;
            fs::create_dir_all("bin")?;
            let entries = fs::read_dir(".")?;
            for entry in entries {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    if path.to_string_lossy().contains("develop")
                        || path.to_string_lossy().contains("git")
                        || path.to_string_lossy().contains("target")
                    {
                        continue;
                    }
                    Command::new("cp")
                        .args(&[
                            "-r",
                            &path.display().to_string(),
                            &format!("develop/{}", path.display()),
                        ])
                        .status()?;
                } else {
                    Command::new("cp")
                        .args(&[&path.display().to_string(), "develop/"])
                        .status()?;
                }
            }
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
            let output = if args.contains(&"--nosave".to_string()) {
                args.retain(|x| x != "--nosave");
                find_empty_file()?
            } else {
                "bin/botball_user_program".to_string()
            };
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
            let mut inputfiles: Vec<String> = Vec::new();
            inputfiles.push("./include/*".to_string());
            match compile_type {
                CompileType::Executable => inputfiles.push("./src/*".to_string()),
                CompileType::Library => {
                    for file in fs::read_dir("./src")? {
                        let file = file?;
                        let path = file.path();
                        if !path.to_string_lossy().contains("main") {
                            inputfiles.push(path.display().to_string());
                        }
                    }
                }
                CompileType::IndependentExecutable => inputfiles.push("./src/*".to_string()),
            }
            let inputs = Inputs::builder()
                .inputfiles(vec!["src/main.cpp"])
                .output(Path::new("bin/").join(&output))
                .libraries(libraries)
                .warnings(warnings)
                .build()?;

            let compres = compile(inputs, compile_type, debug);
            let copyres = fs::copy(
                format!("./develop/bin/{}", output),
                format!("./bin/{}", output),
            );
            if args.contains(&"--nosave".to_string()) {
                let _ = fs::remove_file(Path::new("bin/").join(output));
            }
            let _ = fs::remove_dir_all("develop");
            match (compres, copyres) {
                (Err(comp), _) => Err(comp),
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
                    .nth(0)
                    .expect("couldnt read input. try again")
                    == 'y',
            )
        }
        "help" => match args.get(2) {
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
        _ => Err("InvalidCommand".into()),
    }
}
