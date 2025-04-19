use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::Command;

#[derive(Debug)]
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

    fn to_args(&self) -> Vec<String> {
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

#[derive(Default, Debug)]
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
    let bin_dir = Path::new("./bin");
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

#[derive(Debug)]
enum CompileType {
    Executable,
    Library,
    IndependentExecutable,
}

impl CompileType {
    fn to_args(&self) -> Vec<String> {
        match self {
            CompileType::Executable => Vec::new(),
            CompileType::Library => vec!["-fPIC".into(), "-shared".into()],
            CompileType::IndependentExecutable => vec!["-L./bin".into(), "-leden".into()],
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
    ]
    .into_iter()
    .map(String::from)
    .collect::<Vec<_>>();

    if debug {
        args.push("-g".into());
    }

    args.extend(inputs.to_args());
    args.extend(ctype.to_args());

    let status = Command::new("sudo").args(&args).status()?;
    if !status.success() {
        return Err("Compilation failed".into());
    }
    Ok(())
}

fn copy_files(to: &Path, from: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let status = Command::new("ping").arg("-c").arg("1").status()?;
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
                    if path.ends_with("develop") {
                        continue;
                    }
                    fs::create_dir_all(format!("develop/{}", path.display()))?;
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

            let inputs = Inputs::builder()
                .inputfiles(vec!["src/main.cpp"])
                .output(&output)
                .libraries(libraries)
                .warnings(warnings)
                .build()?;

            let res = compile(inputs, compile_type, debug);

            fs::copy(
                &output,
                format!(
                    "bin/{}",
                    Path::new(&output).file_name().unwrap().to_string_lossy()
                ),
            )?;

            if args.contains(&"--nosave".to_string()) {
                fs::remove_file(&output)?;
            }

            fs::remove_dir_all("develop")?;
            res
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
        _ => Err("Invalid command".into()),
    }
}
