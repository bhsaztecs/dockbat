#![allow(unused)]
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::Command;
struct Inputs {
    infiles: Vec<std::fs::File>,
    libs: Vec<String>,
    warnings: Vec<String>,
    outfile: std::fs::File,
}
impl Inputs {
    fn builder() -> InputsBuilder {
        InputsBuilder::new()
    }
    fn inputs_to_args(self) -> Vec<String> {
        todo!()
    }
}
struct InputsBuilder {
    infiles: Option<Vec<std::fs::File>>,
    libs: Option<Vec<String>>,
    warnings: Option<Vec<String>>,
    outfile: Option<std::fs::File>,
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
    fn inputfiles(mut self, inputfiles: Vec<std::fs::File>) -> InputsBuilder {
        self.infiles = Some(inputfiles);
        self
    }
    fn libraries(mut self, libraries: Vec<String>) -> InputsBuilder {
        self.libs = Some(libraries);
        self
    }
    fn warnings(mut self, warnings: Vec<String>) -> InputsBuilder {
        self.warnings = Some(warnings);
        self
    }
    fn output(mut self, file: std::fs::File) -> InputsBuilder {
        self.outfile = Some(file);
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
fn library(inputs: Inputs) -> Result<(), String> {
    todo!()
}
fn executable(inputs: Inputs) -> Result<(), String> {
    todo!()
}
fn library_executable(inputs: Inputs) -> Result<(), String> {
    todo!()
}
fn copy_files(from: &std::path::Path, to: &std::path::Path) -> Result<(), String> {
    todo!()
}
fn shell() -> Result<(), String> {
    todo!()
}

fn main() {
    println!("Hello, world!");
}
