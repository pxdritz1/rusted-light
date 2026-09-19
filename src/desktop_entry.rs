use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct AppEntry {
    pub name: String,
    pub exec: String,
    pub icon: Option<String>,
}

impl AppEntry {
    #[allow(dead_code)]
    pub fn launch(&self) {
        let exec = clean_exec(&self.exec);
        if exec.is_empty() {
            return;
        }
        let _ = Command::new("sh").arg("-c").arg(exec).spawn();
    }
}

pub fn clean_exec(exec: &str) -> String {
    exec.split_whitespace()
        .filter(|token| !token.starts_with('%'))
        .collect::<Vec<_>>()
        .join(" ")
}

fn xdg_app_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(data_home) = dirs::data_dir() {
        dirs.push(data_home.join("applications"));
    }

    let xdg_data_dirs = std::env::var("XDG_DATA_DIRS")
        .unwrap_or_else(|_| "/usr/local/share:/usr/share".to_string());
    for dir in xdg_data_dirs.split(':') {
        dirs.push(PathBuf::from(dir).join("applications"));
    }

    dirs.push(PathBuf::from("/var/lib/flatpak/exports/share/applications"));
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".local/share/flatpak/exports/share/applications"));
    }

    dirs
}

fn parse_desktop_file(path: &PathBuf) -> Option<AppEntry> {
    let content = fs::read_to_string(path).ok()?;

    let mut name = None;
    let mut exec = None;
    let mut icon = None;
    let mut no_display = false;
    let mut hidden = false;
    let mut is_application = true;
    let mut in_main_section = false;

    for line in content.lines() {
        let line = line.trim();

        if line.starts_with('[') {
            in_main_section = line == "[Desktop Entry]";
            continue;
        }
        if !in_main_section || line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            match key.trim() {
                "Name" => name = Some(value.trim().to_string()),
                "Exec" => exec = Some(value.trim().to_string()),
                "Icon" => icon = Some(value.trim().to_string()),
                "NoDisplay" => no_display = value.trim().eq_ignore_ascii_case("true"),
                "Hidden" => hidden = value.trim().eq_ignore_ascii_case("true"),
                "Type" => is_application = value.trim() == "Application",
                _ => {}
            }
        }
    }

    if no_display || hidden || !is_application {
        return None;
    }

    Some(AppEntry {
        name: name?,
        exec: exec?,
        icon,
    })
}

pub fn scan_applications() -> Vec<AppEntry> {
    let mut seen = HashSet::new();
    let mut apps = Vec::new();

    for dir in xdg_app_dirs() {
        let Ok(read_dir) = fs::read_dir(&dir) else {
            continue;
        };

        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
                continue;
            }

            if let Some(app) = parse_desktop_file(&path) {
                if seen.insert(app.name.clone()) {
                    apps.push(app);
                }
            }
        }
    }

    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}

pub fn filter_apps<'a>(apps: &'a [AppEntry], query: &str) -> Vec<&'a AppEntry> {
    if query.is_empty() {
        return apps.iter().collect();
    }

    let q = query.to_lowercase();
    let mut starts_with: Vec<&AppEntry> = Vec::new();
    let mut contains: Vec<&AppEntry> = Vec::new();

    for app in apps {
        let name = app.name.to_lowercase();
        if name.starts_with(&q) {
            starts_with.push(app);
        } else if name.contains(&q) {
            contains.push(app);
        }
    }

    starts_with.extend(contains);
    starts_with
}
