//! Lets cwRsync's ssh use the Windows OpenSSH agent.
//!
//! cwRsync speaks to a Cygwin socket (`!<socket >port secret`). The key that
//! already unlocks `ssh` lives in `\\.\pipe\openssh-ssh-agent`. This bridge
//! accepts the Cygwin handshake and forwards each agent message to that pipe.

#[cfg(windows)]
use std::fs::{self, OpenOptions};
#[cfg(windows)]
use std::io::{Read, Write};
#[cfg(windows)]
use std::net::TcpListener;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use std::path::PathBuf;
#[cfg(windows)]
use std::sync::Mutex;
#[cfg(windows)]
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(windows)]
static BRIDGE: Mutex<Option<String>> = Mutex::new(None);

const PIPE: &str = r"\\.\pipe\openssh-ssh-agent";
const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;
const FILE_ATTRIBUTE_SYSTEM: u32 = 0x4;

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn SetFileAttributesW(name: *const u16, attrs: u32) -> i32;
}

/// Cygwin path of a socket that forwards to the Windows agent, if the pipe is up.
pub fn windows_agent_bridge_sock() -> Option<String> {
    #[cfg(windows)]
    {
        let mut slot = BRIDGE.lock().ok()?;
        if let Some(sock) = slot.clone() {
            return Some(sock);
        }
        let sock = start_bridge()?;
        *slot = Some(sock.clone());
        Some(sock)
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[cfg(windows)]
fn start_bridge() -> Option<String> {
    // Fail fast when the Windows agent is not running.
    OpenOptions::new().read(true).write(true).open(PIPE).ok()?;

    let listener = TcpListener::bind("127.0.0.1:0").ok()?;
    let port = listener.local_addr().ok()?.port();
    let secret = random_secret();
    let path = socket_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok()?;
    }
    write_socket_file(&path, port, secret)?;

    std::thread::Builder::new()
        .name("agent-bridge".into())
        .spawn(move || {
            for conn in listener.incoming().flatten() {
                let mut conn = conn;
                std::thread::spawn(move || {
                    let _ = serve(&mut conn, secret);
                });
            }
        })
        .ok()?;

    Some(crate::host::cygdrive_path(&path))
}

#[cfg(windows)]
fn socket_path() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|dirs| {
        dirs.home_dir()
            .join(".ssh")
            .join("agent")
            .join("ratatidy-bridge")
    })
}

#[cfg(windows)]
fn random_secret() -> [u32; 4] {
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(1);
    [
        pid.wrapping_mul(0x9E37_79B1),
        nanos | 1,
        pid ^ nanos.rotate_left(7),
        nanos.wrapping_mul(0x85EB_CA6B) ^ pid,
    ]
}

#[cfg(windows)]
fn write_socket_file(path: &std::path::Path, port: u16, secret: [u32; 4]) -> Option<()> {
    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        SetFileAttributesW(wide.as_ptr(), FILE_ATTRIBUTE_NORMAL);
    }
    let body = format!(
        "!<socket >{port} s {:08X}-{:08X}-{:08X}-{:08X}\0",
        secret[0], secret[1], secret[2], secret[3]
    );
    fs::write(path, body).ok()?;
    let ok = unsafe { SetFileAttributesW(wide.as_ptr(), FILE_ATTRIBUTE_SYSTEM) };
    if ok == 0 {
        return None;
    }
    Some(())
}

#[cfg(windows)]
fn serve(client: &mut std::net::TcpStream, secret: [u32; 4]) -> std::io::Result<()> {
    use std::io::ErrorKind;
    client.set_nodelay(true)?;
    let mut got = [0u8; 16];
    client.read_exact(&mut got)?;
    let expect = secret_bytes(secret);
    if got != expect {
        return Err(std::io::Error::new(
            ErrorKind::PermissionDenied,
            "bad cygwin secret",
        ));
    }
    client.write_all(&got)?;
    let mut cred = [0u8; 12];
    client.read_exact(&mut cred)?;
    let pid = (std::process::id() as u32).to_le_bytes();
    client.write_all(&pid)?;
    client.write_all(&[0u8; 8])?;

    let mut pipe = OpenOptions::new().read(true).write(true).open(PIPE)?;
    loop {
        let Some(request) = read_frame(client)? else {
            break;
        };
        pipe.write_all(&request)?;
        pipe.flush()?;
        let Some(reply) = read_frame(&mut pipe)? else {
            break;
        };
        client.write_all(&reply)?;
    }
    Ok(())
}

#[cfg(windows)]
fn secret_bytes(secret: [u32; 4]) -> [u8; 16] {
    let mut out = [0u8; 16];
    for (i, part) in secret.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&part.to_le_bytes());
    }
    out
}

#[cfg(windows)]
fn read_frame(reader: &mut impl Read) -> std::io::Result<Option<Vec<u8>>> {
    let mut header = [0u8; 4];
    match reader.read_exact(&mut header) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }
    let length = u32::from_be_bytes(header);
    if length > 1024 * 1024 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "agent frame too large",
        ));
    }
    let mut body = vec![0u8; length as usize];
    reader.read_exact(&mut body)?;
    let mut frame = Vec::with_capacity(4 + body.len());
    frame.extend_from_slice(&header);
    frame.extend_from_slice(&body);
    Ok(Some(frame))
}

#[cfg(all(windows, test))]
mod tests {
    #[test]
    fn bridge_lists_keys_from_the_windows_agent() {
        let Some(sock) = super::windows_agent_bridge_sock() else {
            return;
        };
        let ssh_add =
            std::path::Path::new(r"C:\ProgramData\chocolatey\lib\rsync\tools\bin\ssh-add.exe");
        if !ssh_add.exists() {
            return;
        }
        let output = std::process::Command::new(ssh_add)
            .arg("-l")
            .env("SSH_AUTH_SOCK", &sock)
            .output()
            .expect("ssh-add");
        let text = String::from_utf8_lossy(&output.stdout);
        let err = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success() && text.contains("ED25519"),
            "stdout={text} stderr={err}"
        );
    }
}
