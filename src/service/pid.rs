use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tracing::{error, info, instrument};

/// PID file manager
pub struct PidFile {
    path: PathBuf,
    pid: u32,
    locked: bool,
    #[allow(dead_code)]
    _lock: Option<Box<dyn std::any::Any + Send + Sync>>,
}

impl PidFile {
    /// Create a new PID file
    #[instrument(skip_all, fields(path = ?path.as_ref()))]
    pub fn create<P: AsRef<Path>>(path: P) -> crate::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let pid = std::process::id();

        info!("Creating PID file at {:?} with PID {}", path, pid);

        // Check if PID file already exists
        if path.exists() {
            // Try to read existing PID
            if let Ok(existing_pid) = Self::read_pid(&path) {
                // Check if process is still running
                if Self::is_process_running(existing_pid) {
                    return Err(crate::Error::Service(format!(
                        "Service already running with PID {existing_pid}"
                    )));
                }
                info!(
                    "Removing stale PID file for non-running process {}",
                    existing_pid
                );
                let _ = fs::remove_file(&path);
            }
        }

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                crate::Error::Service(format!("Failed to create PID directory: {e}"))
            })?;
        }

        // Write PID to file
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .map_err(|e| crate::Error::Service(format!("Failed to create PID file: {e}")))?;

        writeln!(file, "{pid}")
            .map_err(|e| crate::Error::Service(format!("Failed to write PID: {e}")))?;

        // Try to lock the file (advisory lock)
        let (locked, lock) = Self::lock_file(file);

        Ok(Self {
            path,
            pid,
            locked,
            _lock: lock,
        })
    }

    /// Read PID from file
    fn read_pid(path: &Path) -> crate::Result<u32> {
        let mut file = File::open(path)
            .map_err(|e| crate::Error::Service(format!("Failed to open PID file: {e}")))?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| crate::Error::Service(format!("Failed to read PID file: {e}")))?;

        contents
            .trim()
            .parse::<u32>()
            .map_err(|e| crate::Error::Service(format!("Invalid PID in file: {e}")))
    }

    /// Check if a process is running
    fn is_process_running(pid: u32) -> bool {
        #[cfg(unix)]
        {
            use nix::sys::signal::{self, Signal};
            use nix::unistd::Pid;

            // Send signal 0 to check if process exists
            match signal::kill(Pid::from_raw(pid as i32), Signal::SIGCONT) {
                Ok(()) => true,
                Err(nix::errno::Errno::ESRCH) => false, // No such process
                Err(nix::errno::Errno::EPERM) => true, // Process exists but we don't have permission
                Err(_) => false,
            }
        }

        #[cfg(not(unix))]
        {
            // On non-Unix systems, we can't reliably check
            false
        }
    }

    /// Try to lock the PID file (advisory lock)
    fn lock_file(file: File) -> (bool, Option<Box<dyn std::any::Any + Send + Sync>>) {
        #[cfg(unix)]
        {
            use nix::fcntl::{Flock, FlockArg};

            match Flock::lock(file, FlockArg::LockExclusiveNonblock) {
                Ok(flock) => {
                    info!("PID file locked successfully");
                    // Box the lock to store it
                    (
                        true,
                        Some(Box::new(flock) as Box<dyn std::any::Any + Send + Sync>),
                    )
                }
                Err((_file, e)) => {
                    error!("Failed to lock PID file: {}", e);
                    (false, None)
                }
            }
        }

        #[cfg(not(unix))]
        {
            // On non-Unix systems, we can't lock
            let _ = file;
            (false, None)
        }
    }

    /// Remove the PID file
    pub fn remove(&mut self) -> crate::Result<()> {
        info!("Removing PID file at {:?}", self.path);

        // Unlock if locked
        if self.locked {
            self.unlock();
        }

        fs::remove_file(&self.path)
            .map_err(|e| crate::Error::Service(format!("Failed to remove PID file: {e}")))?;

        Ok(())
    }

    /// Unlock the PID file
    fn unlock(&mut self) {
        #[cfg(unix)]
        {
            use nix::fcntl::{Flock, FlockArg};

            if let Ok(file) = File::open(&self.path) {
                let _ = Flock::lock(file, FlockArg::Unlock);
                self.locked = false;
                info!("PID file unlocked");
            }
        }
    }

    /// Get the PID
    #[must_use] pub const fn pid(&self) -> u32 {
        self.pid
    }

    /// Get the path
    #[must_use] pub fn path(&self) -> &Path {
        &self.path
    }

    /// Check if the PID file is locked
    #[must_use] pub const fn is_locked(&self) -> bool {
        self.locked
    }

    /// Get standard PID file path for the service
    #[must_use] pub fn standard_path() -> PathBuf {
        // Try different standard locations
        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            // User runtime directory (systemd style)
            PathBuf::from(runtime_dir).join("rust-sci-hub-mcp.pid")
        } else if let Some(home) = dirs::home_dir() {
            // User home directory
            home.join(".local").join("run").join("rust-sci-hub-mcp.pid")
        } else {
            // Fallback to temp directory
            std::env::temp_dir().join("rust-sci-hub-mcp.pid")
        }
    }
}

impl Drop for PidFile {
    fn drop(&mut self) {
        // Try to remove PID file on drop
        if self.path.exists() {
            let _ = self.remove();
        }
    }
}

impl std::fmt::Debug for PidFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PidFile")
            .field("path", &self.path)
            .field("pid", &self.pid)
            .field("locked", &self.locked)
            .field("has_lock", &self._lock.is_some())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_pid_file_creation() {
        let temp_dir = TempDir::new().unwrap();
        let pid_path = temp_dir.path().join("test.pid");

        let pid_file = PidFile::create(&pid_path).unwrap();
        assert!(pid_path.exists());
        assert_eq!(pid_file.pid(), std::process::id());
    }

    #[test]
    fn test_pid_file_removal() {
        let temp_dir = TempDir::new().unwrap();
        let pid_path = temp_dir.path().join("test.pid");

        let mut pid_file = PidFile::create(&pid_path).unwrap();
        assert!(pid_path.exists());

        pid_file.remove().unwrap();
        assert!(!pid_path.exists());
    }

    #[test]
    fn test_standard_path() {
        let path = PidFile::standard_path();
        assert!(path.to_string_lossy().contains("rust-sci-hub-mcp.pid"));
    }

    #[test]
    fn test_process_running_check() {
        // Current process should be running
        assert!(PidFile::is_process_running(std::process::id()));

        // Very high PID unlikely to be running
        assert!(!PidFile::is_process_running(999_999));
    }
}
