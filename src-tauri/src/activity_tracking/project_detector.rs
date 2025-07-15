use crate::error::Result;
use crate::models::{ProjectContext, ProjectType};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct ProjectDetector;

impl ProjectDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detect_project(
        &self,
        app_name: &str,
        window_title: &str,
    ) -> Result<Option<ProjectContext>> {
        if let Some(directory) = self.extract_directory(app_name, window_title) {
            if let Some(project_root) = self.find_project_root(&directory) {
                let project_name = project_root
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown")
                    .to_string();

                let project_type = self.detect_project_type(&project_root);
                let git_branch = self.get_git_branch(&project_root);

                return Ok(Some(ProjectContext {
                    project_name,
                    project_path: project_root.to_string_lossy().to_string(),
                    project_type,
                    git_branch,
                }));
            }
        }

        Ok(None)
    }

    fn extract_directory(&self, app_name: &str, window_title: &str) -> Option<PathBuf> {
        let app_lower = app_name.to_lowercase();

        // Terminal apps
        if app_lower.contains("terminal") || app_lower.contains("iterm") {
            self.get_terminal_directory(app_name)
        }
        // Code editors
        else if app_lower.contains("code") || app_lower.contains("sublime") {
            self.extract_path_from_title(window_title)
        } else {
            None
        }
    }

    fn get_terminal_directory(&self, app_name: &str) -> Option<PathBuf> {
        let script = match app_name.to_lowercase().as_str() {
            name if name.contains("terminal") => {
                r#"tell application "Terminal" to get the current settings of the selected tab of the front window"#
            }
            name if name.contains("iterm") => {
                r#"tell application "iTerm" to tell current session of current tab of current window to return working directory"#
            }
            _ => return None,
        };

        let output = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .ok()?;

        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Some(PathBuf::from(path))
        } else {
            None
        }
    }

    fn extract_path_from_title(&self, window_title: &str) -> Option<PathBuf> {
        // Try to extract path from window title
        // This is a simplified version - real implementation would be more robust
        if window_title.contains('/') {
            let parts: Vec<&str> = window_title.split(" — ").collect();
            if let Some(path_part) = parts.first() {
                return Some(PathBuf::from(path_part));
            }
        }
        None
    }

    fn find_project_root(&self, start_path: &Path) -> Option<PathBuf> {
        let mut current = start_path.to_path_buf();

        while current.parent().is_some() {
            if self.is_project_root(&current) {
                return Some(current);
            }
            current = current.parent()?.to_path_buf();
        }

        None
    }

    fn is_project_root(&self, path: &Path) -> bool {
        let indicators = [
            ".git",
            "Cargo.toml",
            "package.json",
            "pyproject.toml",
            "requirements.txt",
            "go.mod",
            "pom.xml",
            "build.gradle",
            ".project",
        ];

        indicators
            .iter()
            .any(|indicator| path.join(indicator).exists())
    }

    fn detect_project_type(&self, path: &Path) -> ProjectType {
        if path.join("Cargo.toml").exists() {
            ProjectType::Rust
        } else if path.join("package.json").exists() {
            if path.join("tsconfig.json").exists() {
                ProjectType::TypeScript
            } else {
                ProjectType::JavaScript
            }
        } else if path.join("pyproject.toml").exists() || path.join("requirements.txt").exists() {
            ProjectType::Python
        } else if path.join("go.mod").exists() {
            ProjectType::Go
        } else if path.join("pom.xml").exists() {
            ProjectType::Java
        } else if path.join("*.csproj").exists() {
            ProjectType::CSharp
        } else {
            ProjectType::Other("Unknown".to_string())
        }
    }

    fn get_git_branch(&self, path: &Path) -> Option<String> {
        let output = Command::new("git")
            .arg("branch")
            .arg("--show-current")
            .current_dir(path)
            .output()
            .ok()?;

        if output.status.success() {
            Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            None
        }
    }
}
