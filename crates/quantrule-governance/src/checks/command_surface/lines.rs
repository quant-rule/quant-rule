//! Markdown headings, tables, fences, and backticked names.

use std::path::Path;

use super::{read, relative};

pub(super) fn fenced_blocks(root: &Path, path: &Path) -> Result<Vec<String>, String> {
    let mut blocks = Vec::new();
    let mut current = Vec::new();
    let mut in_fence = false;
    for line in read(path, root)?.split('\n') {
        if line.trim().starts_with("```") {
            if in_fence {
                blocks.push(current.join("\n"));
                current.clear();
            }
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            current.push(line.to_owned());
        }
    }
    if in_fence {
        return Err(format!(
            "{} has an unterminated code fence",
            relative(root, path)
        ));
    }
    Ok(blocks)
}

pub(super) fn section(root: &Path, text: &str, title: &str) -> Result<String, String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut start = None;
    let mut level = 0;
    for (index, line) in lines.iter().enumerate() {
        let current = heading_level(line);
        if current > 0 && line.to_lowercase().contains(&title.to_lowercase()) {
            start = Some(index + 1);
            level = current;
            break;
        }
    }
    let Some(start) = start else {
        return Err(format!("no heading contains {title:?}"));
    };
    let mut end = lines.len();
    for (index, line) in lines.iter().enumerate().skip(start) {
        let current = heading_level(line);
        if current > 0 && current <= level {
            end = index;
            break;
        }
    }
    let _ = root;
    Ok(lines[start..end].join("\n"))
}

fn heading_level(line: &str) -> usize {
    let mut count = 0;
    for character in line.chars() {
        if character == '#' && count < 6 {
            count += 1;
        } else {
            break;
        }
    }
    if count == 0 || !line[count..].starts_with(char::is_whitespace) {
        return 0;
    }
    count
}

pub(super) fn table_cells(line: &str) -> Vec<String> {
    let mut text = line.trim();
    if let Some(rest) = text.strip_prefix('|') {
        text = rest;
    }
    if text.ends_with('|') && !text.ends_with("\\|") {
        text = &text[..text.len() - 1];
    }
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut escaped = false;
    for character in text.chars() {
        if escaped {
            current.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '|' {
            cells.push(current.trim().to_owned());
            current.clear();
        } else {
            current.push(character);
        }
    }
    if escaped {
        current.push('\\');
    }
    cells.push(current.trim().to_owned());
    cells
}

pub(super) fn backticked(line: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find('`') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('`') else {
            break;
        };
        values.push(after[..end].to_owned());
        rest = &after[end + 1..];
    }
    values
}
