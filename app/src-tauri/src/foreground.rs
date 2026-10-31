//! Which program owns the window in front, for automatic profile switching.
//!
//! `names()` returns every name the foreground program goes by, best first; a profile
//! matches if any of them is in its app list (case-insensitive). The first one is what
//! the "detect" button in the Profiles page adds.
//!
//! - Windows: the executable of the foreground window (`cs2.exe`).
//! - Linux: the X11 `_NET_ACTIVE_WINDOW`. On Wayland this sees XWayland windows only,
//!   which covers games under Wine/Proton and most native games; a native Wayland window
//!   in front reads as "nothing", so the selected profile stays in effect. Names are the
//!   Windows `.exe` name for Wine programs (`cs2.exe`), then the Linux executable
//!   (`cs2`), then the window class.

#[cfg(windows)]
pub fn names() -> Vec<String> {
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION},
        UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
    };
    let exe = unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return vec![];
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            return vec![];
        }
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h.is_null() {
            return vec![];
        }
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut len);
        CloseHandle(h);
        if ok == 0 {
            return vec![];
        }
        String::from_utf16_lossy(&buf[..len as usize])
    };
    basename(&exe).into_iter().collect()
}

#[cfg(target_os = "linux")]
pub fn names() -> Vec<String> {
    let Some(win) = x11::active_window() else { return vec![] };
    let mut out = Vec::new();
    if let Some(pid) = win.pid {
        out = process_names(pid);
    }
    if let Some(class) = win.class {
        push_unique(&mut out, class);
    }
    out
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn names() -> Vec<String> {
    vec![]
}

/// Last path component, splitting on both `/` and `\` (Wine paths look like `C:\…\x.exe`).
fn basename(path: &str) -> Option<String> {
    let name = path.rsplit(['/', '\\']).next()?.trim();
    (!name.is_empty()).then(|| name.to_string())
}

fn push_unique(v: &mut Vec<String>, name: String) {
    if !name.is_empty() && !v.iter().any(|n| n.eq_ignore_ascii_case(&name)) {
        v.push(name);
    }
}

/// Names for a process from its `argv[0]` and `/proc/<pid>/exe`.
/// `argv0` and `exe` are passed in so the ordering rule can be tested.
fn names_from(argv0: Option<&str>, exe: Option<&str>) -> Vec<String> {
    let argv0 = argv0.and_then(basename);
    // `/proc/<pid>/exe` of a replaced binary ends in " (deleted)".
    let exe = exe.map(|e| e.trim_end_matches(" (deleted)")).and_then(basename);
    let mut out = Vec::new();
    // Wine and Proton set argv[0] to the Windows path of the program: that name is
    // the useful one (the executable itself is wine64-preloader or similar).
    if let Some(a) = argv0.as_ref().filter(|a| a.to_ascii_lowercase().ends_with(".exe")) {
        push_unique(&mut out, a.clone());
    }
    if let Some(e) = exe {
        push_unique(&mut out, e);
    }
    if let Some(a) = argv0 {
        push_unique(&mut out, a);
    }
    out
}

#[cfg(target_os = "linux")]
fn process_names(pid: u32) -> Vec<String> {
    let cmdline = std::fs::read(format!("/proc/{pid}/cmdline")).unwrap_or_default();
    let argv0 = cmdline.split(|b| *b == 0).next().map(|a| String::from_utf8_lossy(a).into_owned());
    let exe = std::fs::read_link(format!("/proc/{pid}/exe")).ok().map(|p| p.to_string_lossy().into_owned());
    names_from(argv0.as_deref(), exe.as_deref())
}

#[cfg(target_os = "linux")]
mod x11 {
    use std::{
        sync::Mutex,
        time::{Duration, Instant},
    };

    use x11rb::{
        connection::{Connection, RequestConnection},
        protocol::{
            res::{self, ClientIdMask, ClientIdSpec, ConnectionExt as _},
            xproto::{Atom, AtomEnum, ConnectionExt as _, Window},
        },
        rust_connection::RustConnection,
    };

    pub struct Active {
        pub pid: Option<u32>,
        pub class: Option<String>,
    }

    struct X {
        conn: RustConnection,
        root: Window,
        net_active_window: Atom,
        net_wm_pid: Atom,
        /// X-Resource gives the real PID of the client even when `_NET_WM_PID` is
        /// missing or comes from another PID namespace (Flatpak).
        has_res: bool,
    }

    enum Slot {
        Empty,
        Connected(Box<X>),
        /// No X server (pure Wayland without XWayland): try again later, not every second.
        Failed(Instant),
    }

    static SLOT: Mutex<Slot> = Mutex::new(Slot::Empty);
    const RETRY: Duration = Duration::from_secs(30);

    pub fn active_window() -> Option<Active> {
        let mut slot = SLOT.lock().unwrap_or_else(|e| e.into_inner());
        if let Slot::Failed(at) = *slot {
            if at.elapsed() < RETRY {
                return None;
            }
            *slot = Slot::Empty;
        }
        if let Slot::Empty = *slot {
            *slot = match connect() {
                Some(x) => Slot::Connected(Box::new(x)),
                None => Slot::Failed(Instant::now()),
            };
        }
        let Slot::Connected(x) = &*slot else { return None };
        match query(x) {
            Ok(active) => active,
            Err(_) => {
                // Server went away (logout, XWayland restarted): reconnect next time.
                *slot = Slot::Empty;
                None
            }
        }
    }

    fn connect() -> Option<X> {
        std::env::var_os("DISPLAY")?;
        let (conn, screen) = RustConnection::connect(None).ok()?;
        let root = conn.setup().roots.get(screen)?.root;
        let atom = |name: &[u8]| conn.intern_atom(false, name).ok()?.reply().ok().map(|r| r.atom);
        let net_active_window = atom(b"_NET_ACTIVE_WINDOW")?;
        let net_wm_pid = atom(b"_NET_WM_PID")?;
        let has_res = conn.extension_information(res::X11_EXTENSION_NAME).ok().flatten().is_some()
            && conn.res_query_version(1, 2).ok().and_then(|c| c.reply().ok()).is_some_and(|v| (v.server_major, v.server_minor) >= (1, 2));
        Some(X { conn, root, net_active_window, net_wm_pid, has_res })
    }

    type Res<T> = Result<T, Box<dyn std::error::Error>>;

    fn query(x: &X) -> Res<Option<Active>> {
        let reply = x.conn.get_property(false, x.root, x.net_active_window, AtomEnum::ANY, 0, 1)?.reply()?;
        let Some(win) = reply.value32().and_then(|mut v| v.next()).filter(|w| *w != 0) else {
            return Ok(None);
        };
        let pid = client_pid(x, win).or_else(|| wm_pid(x, win));
        Ok(Some(Active { pid, class: wm_class(x, win) }))
    }

    fn client_pid(x: &X, win: Window) -> Option<u32> {
        if !x.has_res {
            return None;
        }
        let spec = ClientIdSpec { client: win, mask: ClientIdMask::LOCAL_CLIENT_PID };
        let reply = x.conn.res_query_client_ids(&[spec]).ok()?.reply().ok()?;
        reply
            .ids
            .iter()
            .find(|id| id.spec.mask.contains(ClientIdMask::LOCAL_CLIENT_PID))
            .and_then(|id| id.value.first().copied())
            .filter(|p| *p != 0)
    }

    fn wm_pid(x: &X, win: Window) -> Option<u32> {
        let reply = x.conn.get_property(false, win, x.net_wm_pid, AtomEnum::ANY, 0, 1).ok()?.reply().ok()?;
        let pid = reply.value32()?.next();
        pid.filter(|p| *p != 0)
    }

    /// Second string of WM_CLASS (the class, e.g. "steam_app_730" for Proton games).
    fn wm_class(x: &X, win: Window) -> Option<String> {
        let reply = x.conn.get_property(false, win, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, 64).ok()?.reply().ok()?;
        let class = reply.value.split(|b| *b == 0).nth(1)?;
        let class = String::from_utf8_lossy(class).trim().to_string();
        (!class.is_empty()).then_some(class)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wine_program_name_comes_first() {
        let n = names_from(Some(r"C:\Program Files\Game\Game.exe"), Some("/usr/lib/wine/wine64-preloader"));
        assert_eq!(n, ["Game.exe", "wine64-preloader"]);
    }

    #[test]
    fn native_program() {
        assert_eq!(names_from(Some("./cs2"), Some("/home/u/.steam/cs2/game/bin/linuxsteamrt64/cs2")), ["cs2"]);
        assert_eq!(names_from(Some("/usr/bin/python3"), Some("/usr/bin/python3.12")), ["python3.12", "python3"]);
        assert_eq!(names_from(None, Some("/opt/app/thing (deleted)")), ["thing"]);
        assert!(names_from(Some(""), None).is_empty());
    }

    /// `cargo test -p openperipheral-app foreground::tests::show -- --ignored --nocapture`
    /// prints what auto-switching sees, five times, one second apart.
    #[test]
    #[ignore]
    fn show() {
        for _ in 0..5 {
            println!("{:?}", names());
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
}
