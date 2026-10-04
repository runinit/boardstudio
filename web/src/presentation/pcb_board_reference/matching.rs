//! Pure matching rules for routed-board model directory imports.

use std::collections::BTreeMap;

fn is_model_filename(filename: &str) -> bool {
    [".step", ".stp", ".stl", ".wrl"]
        .iter()
        .any(|extension| filename.to_ascii_lowercase().ends_with(extension))
}

fn model_basename(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// Return each model path paired with the index of its uniquely matching file.
pub(crate) fn unique_directory_matches(
    paths: &[String],
    filenames: &[String],
) -> Vec<(String, usize)> {
    let mut candidates = BTreeMap::<&str, Vec<usize>>::new();
    for (index, filename) in filenames.iter().enumerate() {
        if is_model_filename(filename) {
            candidates.entry(filename.as_str()).or_default().push(index);
        }
    }

    let mut matched = Vec::new();
    for path in paths {
        if let Some(files) = candidates.get(model_basename(path))
            && files.len() == 1
        {
            matched.push((path.clone(), files[0]));
        }
    }
    matched
}

#[cfg(test)]
mod tests {
    use super::unique_directory_matches;

    #[test]
    fn a_unique_directory_file_attaches_to_every_path_with_its_basename() {
        let paths = vec!["front/body.step".to_owned(), "rear/body.step".to_owned()];
        let filenames = vec!["body.step".to_owned()];

        assert_eq!(
            unique_directory_matches(&paths, &filenames),
            vec![
                ("front/body.step".to_owned(), 0),
                ("rear/body.step".to_owned(), 0),
            ]
        );
    }
}
