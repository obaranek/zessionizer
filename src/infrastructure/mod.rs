//! Infrastructure layer for filesystem and environment interactions.
//!
//! This module provides utilities for working with the Zellij plugin sandbox
//! environment, particularly path handling where the host filesystem is mounted
//! under `/host`.

pub mod paths;

pub use paths::{expand_tilde, get_data_dir, strip_host_prefix};

use std::path::{Path, PathBuf};

/// Finds a layout file that matches the project name in Zellij's layouts directory.
///
/// This function looks for layout files (with .kdl extension) in the standard Zellij
/// layouts directory (`~/.config/zellij/layouts`) that match the given project name.
/// For example, if the project name is "dotfiles", it will look for "dotfiles.kdl".
/// 
/// The function checks for multiple layout file extensions to support different formats.
///
/// # Parameters
/// 
/// * `project_name` - The name of the project to match against layout files
/// 
/// # Returns
/// 
/// Returns `Some(PathBuf)` containing the path to the matching layout file if found,
/// or `None` if no matching layout file exists.
/// 
/// # Examples
/// 
/// ```
/// use crate::infrastructure::find_layout_for_project;
/// 
/// // If ~/.config/zellij/layouts/dotfiles.kdl exists
/// let layout_path = find_layout_for_project("dotfiles");
/// assert!(layout_path.is_some());
/// ```
#[must_use]
pub fn find_layout_for_project(project_name: &str) -> Option<PathBuf> {
    // In Zellij's sandbox environment, the host's home directory is accessible via /host
    // We'll try multiple possible locations for layout files
    let possible_base_paths = [
        Path::new("/host").to_path_buf(),  // Standard host access point in sandbox
    ];
    
    // Try different layout file extensions
    let extensions = [".kdl", ".yaml", ".yml"];
    
    for base_path in &possible_base_paths {
        let layouts_dir = base_path.join(".config").join("zellij").join("layouts");
        
        // Check if the layouts directory exists first
        if layouts_dir.exists() {
            for ext in &extensions {
                let layout_path = layouts_dir.join(format!("{}{}", project_name, ext));
                
                // Check if the layout file exists in the sandbox environment
                if layout_path.exists() {
                    return Some(layout_path);
                }
            }
        }
    }
    
    None
}
