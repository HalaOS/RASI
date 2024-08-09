//! Utilities that communicate with the hard hat network.

use std::{
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    thread::spawn,
};

use crate::errors::{Error, Result};

/// A type represents a hard hat project item.
pub struct HardHatProject {
    #[allow(unused)]
    /// hardhat project root directory path.
    path: PathBuf,
    /// The hardhat network node associated with the project.
    network: Arc<Mutex<RawHardHatNetwork>>,
}

impl<P: AsRef<Path>> From<P> for HardHatProject {
    fn from(value: P) -> Self {
        Self {
            path: value.as_ref().to_owned(),
            network: Arc::new(Mutex::new(RawHardHatNetwork {
                root_path: value.as_ref().to_owned(),
                child_process: None,
            })),
        }
    }
}

impl HardHatProject {
    /// try start hardhat network.
    ///
    /// if the network node is already started, returns the [`HardHatNetwork`] instance immediately
    pub fn start_network(&self) -> Result<HardHatNetwork> {
        {
            let mut network = self.network.lock().unwrap();

            network.restart()?;
        }

        Ok(HardHatNetwork {
            inner: self.network.clone(),
        })
    }

    /// Invoke `hardhat compile` command, and wait to exit.
    pub fn compile(&self) -> Result<()> {
        let mut child_process = Command::new("npx")
            .arg("hardhat")
            .arg("compile")
            .current_dir(&self.path)
            .stdout(Stdio::piped())
            .spawn()?;

        let stdout = child_process.stdout.take().unwrap();

        let stdout_reader = BufReader::new(stdout);
        let stdout_lines = stdout_reader.lines();

        for line in stdout_lines {
            if let Ok(line) = line {
                println!("{}", line);
            } else {
                break;
            }
        }

        let status = child_process.wait()?;

        if !status.success() {
            return Err(Error::Other(format!(
                "hardhat compile exit with errorcode: {}",
                status
            )));
        }

        Ok(())
    }
}

#[derive(Default)]
struct RawHardHatNetwork {
    root_path: PathBuf,
    child_process: Option<Child>,
}

impl RawHardHatNetwork {
    fn restart(&mut self) -> Result<()> {
        self.stop()?;

        log::trace!("start hardhat node in directory: {:#?}", self.root_path);

        let mut child_process = Command::new("npx")
            .arg("hardhat")
            .arg("node")
            .current_dir(&self.root_path)
            .stdout(Stdio::piped())
            .spawn()?;

        let stdout = child_process.stdout.take().unwrap();

        spawn(move || {
            let stdout_reader = BufReader::new(stdout);
            let stdout_lines = stdout_reader.lines();

            log::trace!("spawn stdout lines");

            for line in stdout_lines {
                if let Ok(line) = line {
                    println!("{}", line);
                } else {
                    break;
                }
            }
        });

        self.child_process = Some(child_process);

        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        if let Some(mut child_process) = self.child_process.take() {
            child_process.kill()?;
            child_process.wait()?;
        }

        Ok(())
    }
}

pub struct HardHatNetwork {
    inner: Arc<Mutex<RawHardHatNetwork>>,
}

impl HardHatNetwork {
    /// Restart the hardhat network child process.
    pub fn restart(&self) -> Result<()> {
        self.inner.lock().unwrap().restart()?;

        Ok(())
    }

    /// Stop the child process
    pub fn stop(&self) -> Result<()> {
        self.inner.lock().unwrap().stop()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{thread::sleep, time::Duration};

    use super::*;

    #[ignore]
    #[test]
    fn test_hardhat() {
        pretty_env_logger::init();
        let root_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/hardhat");

        let project = HardHatProject::from(root_path);

        let network = project.start_network().unwrap();

        sleep(Duration::from_secs(2));

        network.stop().unwrap();

        project.compile().unwrap();
    }
}
