//! Small Markdown table helpers shared by the documentation checkers.

/// Split one pipe-table row, keeping escaped pipes inside a cell.
#[must_use]
pub fn cells(line: &str) -> Vec<String> {
    let protected = line.trim().replace("\\|", "\u{0}");
    let parts: Vec<&str> = protected.split('|').collect();
    if parts.len() < 2 {
        return Vec::new();
    }
    parts[1..parts.len() - 1]
        .iter()
        .map(|cell| cell.trim().replace('\u{0}', "|"))
        .collect()
}

/// Read the rows under an exact header line.
#[must_use]
pub fn rows(text: &str, header: &str) -> Vec<Vec<String>> {
    let Some(start) = text.find(header) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for line in text[start..].lines().skip(2) {
        if !line.trim_start().starts_with('|') {
            break;
        }
        let row = cells(line);
        if !row.is_empty() {
            found.push(row);
        }
    }
    found
}

/// Return the target of the first Markdown link in a table cell.
#[must_use]
pub fn link_target(cell: &str) -> Option<String> {
    let start = cell.find("](")? + 2;
    let end = cell[start..].find(')')? + start;
    Some(cell[start..end].trim().to_owned())
}

/// Strip backticks from a table cell.
#[must_use]
pub fn unquote(cell: &str) -> String {
    cell.trim().trim_matches('`').to_owned()
}

/// Whether a table has a first cell equal to `name`.
#[must_use]
pub fn has_row(text: &str, name: &str) -> bool {
    text.lines().any(|line| {
        let row = cells(line);
        row.first().is_some_and(|cell| unquote(cell) == name)
    })
}

/// Return the cell at `index` for the row whose first cell is `name`.
#[must_use]
pub fn row_cell(text: &str, name: &str, index: usize) -> Option<String> {
    text.lines().find_map(|line| {
        let row = cells(line);
        if row.first().is_some_and(|cell| unquote(cell) == name) {
            row.get(index).cloned()
        } else {
            None
        }
    })
}
