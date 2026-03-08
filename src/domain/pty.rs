use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;

use color_eyre::Result;
use color_eyre::eyre::eyre;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};

#[allow(dead_code)]
pub struct PtySession {
    child: Box<dyn portable_pty::Child + Send + Sync>,
    screen: Arc<Mutex<vt100::Parser>>,
    working_dir: String,
    writer: Box<dyn Write + Send>,
}

impl PtySession {
    pub fn spawn(cmd: &str, working_dir: &str, rows: u16, cols: u16) -> Result<Self> {
        Self::spawn_with_args(cmd, &[], working_dir, rows, cols)
    }

    pub fn spawn_with_args(
        cmd: &str,
        args: &[&str],
        working_dir: &str,
        rows: u16,
        cols: u16,
    ) -> Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| eyre!(e))?;

        let mut command = CommandBuilder::new(cmd);
        for arg in args {
            command.arg(arg);
        }
        command.cwd(working_dir);

        let child = pair.slave.spawn_command(command).map_err(|e| eyre!(e))?;
        drop(pair.slave);

        let mut reader = pair.master.try_clone_reader().map_err(|e| eyre!(e))?;
        let writer = pair.master.take_writer().map_err(|e| eyre!(e))?;

        let screen = Arc::new(Mutex::new(vt100::Parser::new(rows, cols, 1000)));
        let screen_clone = Arc::clone(&screen);

        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if let Ok(mut parser) = screen_clone.lock() {
                            parser.process(&buf[..n]);
                        }
                    }
                }
            }
        });

        Ok(Self {
            child,
            screen,
            working_dir: working_dir.to_owned(),
            writer,
        })
    }

    pub fn is_alive(&mut self) -> bool {
        self.child
            .try_wait()
            .ok()
            .is_none_or(|status| status.is_none())
    }

    pub fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    pub fn screen(&self) -> Arc<Mutex<vt100::Parser>> {
        Arc::clone(&self.screen)
    }

    #[allow(dead_code)]
    pub fn working_dir(&self) -> &str {
        &self.working_dir
    }

    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        self.writer.write_all(data)?;
        self.writer.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn spawn_echo_succeeds() {
        let session = PtySession::spawn("echo", "/tmp", 24, 80);
        assert!(session.is_ok());
    }

    #[test]
    fn screen_returns_parser() {
        let session = PtySession::spawn("echo", "/tmp", 24, 80).unwrap();
        let screen = session.screen();
        let parser = screen.lock().unwrap();
        assert_eq!(parser.screen().size(), (24, 80));
    }

    #[test]
    fn working_dir_returns_correct_path() {
        let session = PtySession::spawn("echo", "/tmp", 24, 80).unwrap();
        assert_eq!(session.working_dir(), "/tmp");
    }

    #[test]
    fn is_alive_for_short_lived_process() {
        let mut session = PtySession::spawn("echo", "/tmp", 24, 80).unwrap();
        thread::sleep(Duration::from_millis(100));
        assert!(!session.is_alive());
    }

    #[test]
    fn is_alive_for_long_lived_process() {
        let mut session = PtySession::spawn("cat", "/tmp", 24, 80).unwrap();
        assert!(session.is_alive());
        session.kill();
    }

    #[test]
    fn kill_terminates_process() {
        let mut session = PtySession::spawn("cat", "/tmp", 24, 80).unwrap();
        session.kill();
        thread::sleep(Duration::from_millis(100));
        assert!(!session.is_alive());
    }
}
