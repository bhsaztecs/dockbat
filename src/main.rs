#![allow(unused)]
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
        let mut args: Vec<String> = Vec::new();
        for warning in self.warnings {
            args.push(format!("-W{}", warning));
        }
        for infile in self.infiles {
            let path_str = infile.to_string_lossy().to_string();
            if path_str.contains(".h") || path_str.contains(".hpp") {
                args.push(format!("-I{}", infile.display()));
            } else if path_str.contains(".c") || path_str.contains(".cpp") {
                args.push(format!("{}", infile.display()));
            }
        }
        for lib in self.libs {
            args.push(format!("-l{}", lib));
        }
        args.push(format!("-o {}", self.outfile.display()));
        args
    }
}
struct InputsBuilder {
    infiles: Option<Vec<std::path::PathBuf>>,
    libs: Option<Vec<String>>,
    warnings: Option<Vec<String>>,
    outfile: Option<std::path::PathBuf>,
}
impl InputsBuilder {
    fn new() -> InputsBuilder {
        InputsBuilder {
            infiles: None,
            libs: None,
            warnings: None,
            outfile: None,
        }
    }
    fn inputfiles(mut self, inputfiles: Vec<&str>) -> InputsBuilder {
        self.infiles = Some(
            inputfiles
                .iter()
                .map(|s| std::path::PathBuf::from(s))
                .collect(),
        );
        self
    }
    fn libraries(mut self, libraries: Vec<&str>) -> InputsBuilder {
        self.libs = Some(libraries.iter().map(|s| s.to_string()).collect());
        self
    }
    fn warnings(mut self, warnings: Vec<&str>) -> InputsBuilder {
        self.warnings = Some(warnings.iter().map(|s| s.to_string()).collect());
        self
    }
    fn output(mut self, file: &str) -> InputsBuilder {
        self.outfile = Some(std::path::PathBuf::from(file));
        self
    }
    fn build(self) -> Inputs {
        let infiles = match self.infiles {
            Some(files) => files,
            _ => Vec::new(),
        };
        let libs = match self.libs {
            Some(libraries) => libraries,
            _ => Vec::new(),
        };
        let warnings = match self.warnings {
            Some(warns) => warns,
            _ => Vec::new(),
        };
        let outfile = match self.outfile {
            Some(file) => file,
            _ => panic!("Output file is required"),
        };

        Inputs {
            infiles,
            libs,
            warnings,
            outfile,
        }
    }
}

fn findemptyfile() -> String {
    let paths = std::fs::read_dir("./bin/").unwrap();
    let current = "out";
    let mut i = 0;
    for path_result in paths {
        let path = path_result.unwrap();
        let file_path = path
            .path()
            .parent()
            .unwrap()
            .join(format!("{}{}", current, i));

        if file_path.exists() {
            i += 1;
        } else {
            return file_path.to_string_lossy().to_string();
        }
    }
    format!("./bin/{}{}", current, i)
}
fn initialize(save: bool) -> Result<(), String> {
    if save {
        // Initialize git repository
        if let Err(e) = Command::new("git").arg("init").output() {
            return Err(format!("Failed to initialize git repository: {}", e));
        }

        if let Err(e) = Command::new("git").arg("add").arg(".").output() {
            return Err(format!("Failed to add files to git: {}", e));
        }

        if let Err(e) = Command::new("git")
            .args(&["commit", "-m", "pre-initialize"])
            .output()
        {
            return Err(format!("Failed to commit files: {}", e));
        }
    }

    // Create bin directory if it doesn't exist
    if !Path::new("bin").exists() {
        fs::create_dir_all("bin").map_err(|e| format!("Failed to create bin directory: {}", e))?;
    }

    // Create empty botball_user_program file
    let botball_path = Path::new("bin").join("botball_user_program");
    File::create(&botball_path)
        .map_err(|e| format!("Failed to create botball_user_program: {}", e))?;

    // Pull Docker image - use platform-specific command execution
    let status = if cfg!(target_os = "windows") {
        Command::new("docker")
            .args(&["pull", "sillyfreak/wombat-cross"])
            .status()
    } else {
        Command::new("docker")
            .args(&["pull", "sillyfreak/wombat-cross"])
            .status()
    };

    if let Err(e) = status {
        return Err(format!("Failed to pull Docker image: {}", e));
    }

    // Create src directory if it doesn't exist
    if !Path::new("src").exists() {
        fs::create_dir_all("src").map_err(|e| format!("Failed to create src directory: {}", e))?;
    }

    // Create main.cpp file
    let src_path = Path::new("src").join("main.cpp");

    // Copy example.cpp content to main.cpp
    let example_path = Path::new("data").join("example.cpp");

    // Check if example file exists
    if example_path.exists() {
        let mut example_file =
            File::open(&example_path).map_err(|e| format!("Failed to open example.cpp: {}", e))?;

        let mut content = String::new();
        example_file
            .read_to_string(&mut content)
            .map_err(|e| format!("Failed to read example.cpp: {}", e))?;

        let mut main_file =
            File::create(&src_path).map_err(|e| format!("Failed to create main.cpp: {}", e))?;

        main_file
            .write_all(content.as_bytes())
            .map_err(|e| format!("Failed to write to main.cpp: {}", e))?;
    } else {
        return Err("example.cpp not found in data directory".to_string());
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
        let args = match self {
            CompileType::Executable => [].to_vec(),
            CompileType::Library => ["-fPIC", "-shared"].to_vec(),
            CompileType::IndependentExecutable => ["-L./bin", "-leden"].to_vec(),
        };
        args.iter().map(|x| x.to_string()).collect()
    }
}
fn compile(inputs: Inputs, ctypes: CompileType, debug: bool) -> Result<(), String> {
    let mut args: Vec<String> = Vec::new();
    args.extend(
        [
            "run",
            "-it",
            "--rm",
            "--volume",
            ".develop:/home/kipr:rw",
            "sillyfreak/wombat-cross",
            "aarch64-linux-gnu-g++",
            "-std=c++17",
        ]
        .iter()
        .map(|x| x.to_string()),
    );
    if debug {
        args.push("-g".to_string())
    }
    args.extend(inputs.to_args());
    args.extend(ctypes.to_args());
    if let Err(e) = Command::new("docker").args(&args).status() {
        return Err(format!("Error on compile: {}", e));
    }
    Ok(())
}
fn copy_files(from: &std::path::Path, to: &std::path::Path) -> Result<(), String> {
    if let Err(e) = Command::new("ping").arg("-c").arg("1").output() {
        return Err(format!("Error on ping: {}", e));
    }
    if let Err(e) = Command::new("scp")
        .arg("-r")
        .arg("-q")
        .arg(from.as_os_str())
        .arg(format!("\"kipr@192.168.125.1:{}\"", to.display()))
        .output()
    {
        return Err(format!("Error on send: {}", e));
    }
    Ok(())
}
fn shell() -> Result<(), String> {
    if let Err(e) = Command::new("ssh").arg("kipr@192.168.125.1").output() {
        return Err(format!("Error: {}", e));
    }
    Ok(())
}

fn main() {
    let input = Inputs::builder()
        .inputfiles(["main.cpp", "blah.h", "foo.hpp", "bar.c"].to_vec())
        .libraries(["kipr", "z", "m", "pthread"].to_vec())
        .output("botball_user_program")
        .warnings(["all"].to_vec())
        .build();
    compile(input, CompileType::Executable, true);
    //println!("{:?}", input.to_args());
}
