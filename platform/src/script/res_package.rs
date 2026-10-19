//! Optional resource index for desktop packages with a custom disk layout.
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Component, Path};

pub fn resource_key(path: &str) -> u64 {
    path.as_bytes().iter().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

pub fn read_index(root: &Path) -> io::Result<HashMap<u64, String>> {
    let source = fs::read_to_string(root.join("resources.index"))?;
    parse_index(&source)
}

fn parse_index(source: &str) -> io::Result<HashMap<u64, String>> {
    let mut paths = HashMap::new();
    for line in source.lines().filter(|line| !line.is_empty()) {
        let invalid = || io::Error::new(io::ErrorKind::InvalidData, "invalid packaged resource index");
        let (key, path) = line.split_once('\t').ok_or_else(invalid)?;
        let key = u64::from_str_radix(key, 16).map_err(|_| invalid())?;
        if path.is_empty()
            || path.contains('\\')
            || path.contains(':')
            || Path::new(path).components().any(|part| !matches!(part, Component::Normal(_)))
            || paths.insert(key, path.to_owned()).is_some()
        {
            return Err(invalid());
        }
    }
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::{parse_index, resource_key};

    #[test]
    fn resolves_relocated_font_and_icon_paths() {
        let font = "makepad_widgets/resources/IBMPlexSans-Text.ttf";
        let icon = "makepad_component/resources/icons/eye.svg";
        let index = format!("{:016x}\tfonts/IBMPlexSans-Text.ttf\n{:016x}\tcomponent/resources/icons/eye.svg\n", resource_key(font), resource_key(icon));
        let paths = parse_index(&index).unwrap();
        assert_eq!(paths[&resource_key(font)], "fonts/IBMPlexSans-Text.ttf");
        assert_eq!(paths[&resource_key(icon)], "component/resources/icons/eye.svg");
        assert!(!paths.contains_key(&resource_key("missing")));
    }

    #[test]
    fn rejects_escaping_paths_and_duplicate_keys() {
        for path in ["../outside.ttf", "/outside.ttf", "C:/outside.ttf", "..\\outside.ttf"] {
            assert!(parse_index(&format!("1\t{path}\n")).is_err());
        }
        assert!(parse_index("1\tfonts/a.ttf\n1\tfonts/b.ttf\n").is_err());
        assert!(parse_index("invalid row").is_err());
    }

    #[test]
    fn key_matches_packager_fnv1a_specification() {
        assert_eq!(resource_key(""), 0xcbf29ce484222325);
        assert_eq!(resource_key("hello"), 0xa430d84680aabd0b);
    }
}
