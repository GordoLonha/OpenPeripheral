//! Profiles: each one bundles lighting, key bindings and EQ, plus the programs
//! that activate it automatically while they are in the foreground.

use serde::{Deserialize, Serialize};

use crate::{effects::Lighting, eq::Eq, remap::Bindings};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub lighting: Lighting,
    #[serde(default)]
    pub bindings: Bindings,
    /// Mouse button name (see op_core::m65::BUTTONS) -> action.
    #[serde(default)]
    pub mouse_bindings: Bindings,
    #[serde(default)]
    pub eq: Eq,
    /// Executable names (e.g. "cs2.exe") that switch to this profile while focused.
    #[serde(default)]
    pub apps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Store {
    /// Profile the user picked; auto-switching returns here when the game closes.
    pub selected: String,
    pub profiles: Vec<Profile>,
}

/// What the UI sees: the store plus which profile is in effect right now.
#[derive(Debug, Clone, Serialize)]
pub struct Overview {
    pub selected: String,
    pub active: String,
    /// Executable that caused an automatic switch, if any.
    pub auto_app: Option<String>,
    pub profiles: Vec<ProfileInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileInfo {
    pub id: String,
    pub name: String,
    pub apps: Vec<String>,
}

impl Store {
    pub fn new_default(lighting: Lighting, bindings: Bindings, eq: Eq) -> Self {
        Self {
            selected: "default".into(),
            profiles: vec![Profile { id: "default".into(), name: "Padrão".into(), lighting, bindings, mouse_bindings: Default::default(), eq, apps: vec![] }],
        }
    }

    pub fn get(&self, id: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.id == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Profile> {
        self.profiles.iter_mut().find(|p| p.id == id)
    }

    /// Profile whose app list contains `exe` (case-insensitive).
    pub fn for_app(&self, exe: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.apps.iter().any(|a| a.eq_ignore_ascii_case(exe)))
    }

    pub fn create(&mut self, name: &str, copy_from: &str) -> String {
        let base = self.get(copy_from).or(self.profiles.first()).cloned();
        let mut id = slug(name);
        let mut n = 2;
        while self.get(&id).is_some() {
            id = format!("{}-{n}", slug(name));
            n += 1;
        }
        let mut p = base.unwrap_or_else(|| Profile {
            id: String::new(),
            name: String::new(),
            lighting: Lighting::default(),
            bindings: Bindings::new(),
            mouse_bindings: Bindings::new(),
            eq: Eq::default(),
            apps: vec![],
        });
        p.id = id.clone();
        p.name = name.trim().to_string();
        p.apps.clear();
        self.profiles.push(p);
        id
    }

    pub fn delete(&mut self, id: &str) -> bool {
        if self.profiles.len() <= 1 {
            return false; // always keep one
        }
        self.profiles.retain(|p| p.id != id);
        if self.get(&self.selected).is_none() {
            self.selected = self.profiles[0].id.clone();
        }
        true
    }

    /// Cycle to the next profile (for the "switch profile" key action).
    pub fn next_after(&self, id: &str) -> String {
        let i = self.profiles.iter().position(|p| p.id == id).unwrap_or(0);
        self.profiles[(i + 1) % self.profiles.len()].id.clone()
    }
}

fn slug(name: &str) -> String {
    let s: String = name
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let s = s.trim_matches('-').to_string();
    if s.is_empty() { "perfil".into() } else { s }
}

/// Executable name of the foreground window, e.g. "cs2.exe".
#[cfg(windows)]
pub fn foreground_exe() -> Option<String> {
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION},
        UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
    };
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            return None;
        }
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h.is_null() {
            return None;
        }
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut len);
        CloseHandle(h);
        if ok == 0 {
            return None;
        }
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        path.rsplit(['\\', '/']).next().map(str::to_string)
    }
}

#[cfg(not(windows))]
pub fn foreground_exe() -> Option<String> {
    None // TODO: X11 _NET_ACTIVE_WINDOW / compositor-specific on Wayland
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_copy_delete_cycle() {
        let mut s = Store::new_default(Lighting::default(), Bindings::new(), Eq::default());
        let id = s.create("CS 2", "default");
        assert_eq!(id, "cs-2");
        assert_eq!(s.create("CS 2", "default"), "cs-2-2");
        s.get_mut(&id).unwrap().apps.push("cs2.exe".into());
        assert_eq!(s.for_app("CS2.EXE").unwrap().id, "cs-2");
        assert_eq!(s.next_after("default"), "cs-2");
        assert!(s.delete("cs-2"));
        assert!(s.delete("cs-2-2"));
        assert!(!s.delete("default"), "last profile is kept");
    }
}
