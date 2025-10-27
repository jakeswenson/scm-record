//! Common tests for semantic parsing (language-agnostic functionality).

use super::*;
use std::path::PathBuf;

#[test]
fn test_language_detection_rust() {
    let path = PathBuf::from("test.rs");
    assert_eq!(
        SupportedLanguage::from_path(&path),
        Some(SupportedLanguage::Rust)
    );
}

#[test]
fn test_language_detection_kotlin() {
    let path = PathBuf::from("test.kt");
    assert_eq!(
        SupportedLanguage::from_path(&path),
        Some(SupportedLanguage::Kotlin)
    );

    let path = PathBuf::from("test.kts");
    assert_eq!(
        SupportedLanguage::from_path(&path),
        Some(SupportedLanguage::Kotlin)
    );
}

#[test]
fn test_language_detection_java() {
    let path = PathBuf::from("test.java");
    assert_eq!(
        SupportedLanguage::from_path(&path),
        Some(SupportedLanguage::Java)
    );
}

#[test]
fn test_language_detection_hcl() {
    let path = PathBuf::from("main.tf");
    assert_eq!(
        SupportedLanguage::from_path(&path),
        Some(SupportedLanguage::Hcl)
    );

    let path = PathBuf::from("test.hcl");
    assert_eq!(
        SupportedLanguage::from_path(&path),
        Some(SupportedLanguage::Hcl)
    );
}

#[test]
fn test_language_detection_python() {
    let path = PathBuf::from("test.py");
    assert_eq!(
        SupportedLanguage::from_path(&path),
        Some(SupportedLanguage::Python)
    );
}

#[test]
fn test_language_detection_markdown() {
    let path = PathBuf::from("README.md");
    assert_eq!(
        SupportedLanguage::from_path(&path),
        Some(SupportedLanguage::Markdown)
    );
}

#[test]
fn test_language_detection_yaml() {
    let path = PathBuf::from("config.yaml");
    assert_eq!(
        SupportedLanguage::from_path(&path),
        Some(SupportedLanguage::Yaml)
    );

    let path = PathBuf::from("config.yml");
    assert_eq!(
        SupportedLanguage::from_path(&path),
        Some(SupportedLanguage::Yaml)
    );
}

#[test]
fn test_language_detection_unsupported() {
    let path = PathBuf::from("test.txt");
    assert_eq!(SupportedLanguage::from_path(&path), None);
}

#[test]
fn test_language_names() {
    assert_eq!(SupportedLanguage::Rust.name(), "Rust");
    assert_eq!(SupportedLanguage::Kotlin.name(), "Kotlin");
    assert_eq!(SupportedLanguage::Java.name(), "Java");
    assert_eq!(SupportedLanguage::Hcl.name(), "HCL");
    assert_eq!(SupportedLanguage::Python.name(), "Python");
    assert_eq!(SupportedLanguage::Markdown.name(), "Markdown");
    assert_eq!(SupportedLanguage::Yaml.name(), "YAML");
}

#[test]
fn test_filter_section_indices_by_range_exact_match() {
    let section_ranges = vec![
        SectionLineRange {
            section_index: 0,
            start_line: 0,
            end_line: 5,
        },
        SectionLineRange {
            section_index: 1,
            start_line: 10,
            end_line: 15,
        },
        SectionLineRange {
            section_index: 2,
            start_line: 20,
            end_line: 25,
        },
    ];

    let indices = filter_section_indices_by_range(&section_ranges, 10, 15);
    assert_eq!(indices, vec![1]);
}

#[test]
fn test_filter_section_indices_by_range_overlap() {
    let section_ranges = vec![
        SectionLineRange {
            section_index: 0,
            start_line: 0,
            end_line: 10,
        },
        SectionLineRange {
            section_index: 1,
            start_line: 8,
            end_line: 15,
        },
        SectionLineRange {
            section_index: 2,
            start_line: 20,
            end_line: 25,
        },
    ];

    // Range [5, 12) should overlap with sections 0 and 1
    let indices = filter_section_indices_by_range(&section_ranges, 5, 12);
    assert_eq!(indices, vec![0, 1]);
}

#[test]
fn test_filter_section_indices_by_range_no_overlap() {
    let section_ranges = vec![
        SectionLineRange {
            section_index: 0,
            start_line: 0,
            end_line: 5,
        },
        SectionLineRange {
            section_index: 1,
            start_line: 10,
            end_line: 15,
        },
    ];

    // Range [6, 9) doesn't overlap with any section
    let indices = filter_section_indices_by_range(&section_ranges, 6, 9);
    assert_eq!(indices, Vec::<usize>::new());
}

#[test]
fn test_filter_section_indices_by_range_contains_all() {
    let section_ranges = vec![
        SectionLineRange {
            section_index: 0,
            start_line: 5,
            end_line: 10,
        },
        SectionLineRange {
            section_index: 1,
            start_line: 15,
            end_line: 20,
        },
        SectionLineRange {
            section_index: 2,
            start_line: 25,
            end_line: 30,
        },
    ];

    // Range [0, 100) contains all sections
    let indices = filter_section_indices_by_range(&section_ranges, 0, 100);
    assert_eq!(indices, vec![0, 1, 2]);
}

#[test]
fn test_filter_section_indices_by_range_partial_overlap_start() {
    let section_ranges = vec![SectionLineRange {
        section_index: 0,
        start_line: 10,
        end_line: 20,
    }];

    // Range [5, 15) overlaps with section at the start
    let indices = filter_section_indices_by_range(&section_ranges, 5, 15);
    assert_eq!(indices, vec![0]);
}

#[test]
fn test_filter_section_indices_by_range_partial_overlap_end() {
    let section_ranges = vec![SectionLineRange {
        section_index: 0,
        start_line: 10,
        end_line: 20,
    }];

    // Range [15, 25) overlaps with section at the end
    let indices = filter_section_indices_by_range(&section_ranges, 15, 25);
    assert_eq!(indices, vec![0]);
}

#[test]
fn test_calculate_section_line_ranges() {
    use crate::{ChangeType, Section, SectionChangedLine};
    use std::borrow::Cow;

    let sections = vec![
        Section::Unchanged {
            lines: vec![
                Cow::Borrowed("line1\n"),
                Cow::Borrowed("line2\n"),
                Cow::Borrowed("line3\n"),
            ],
        },
        Section::Changed {
            lines: vec![
                SectionChangedLine {
                    is_checked: false,
                    change_type: ChangeType::Removed,
                    line: Cow::Borrowed("old\n"),
                },
                SectionChangedLine {
                    is_checked: false,
                    change_type: ChangeType::Added,
                    line: Cow::Borrowed("new1\n"),
                },
                SectionChangedLine {
                    is_checked: false,
                    change_type: ChangeType::Added,
                    line: Cow::Borrowed("new2\n"),
                },
            ],
        },
        Section::Unchanged {
            lines: vec![Cow::Borrowed("line4\n"), Cow::Borrowed("line5\n")],
        },
    ];

    let ranges = calculate_section_line_ranges(&sections);

    assert_eq!(ranges.len(), 3);
    assert_eq!(ranges[0].section_index, 0);
    assert_eq!(ranges[0].start_line, 0);
    assert_eq!(ranges[0].end_line, 3); // 3 lines

    assert_eq!(ranges[1].section_index, 1);
    assert_eq!(ranges[1].start_line, 3);
    assert_eq!(ranges[1].end_line, 5); // 2 added lines (removed doesn't count)

    assert_eq!(ranges[2].section_index, 2);
    assert_eq!(ranges[2].start_line, 5);
    assert_eq!(ranges[2].end_line, 7); // 2 lines
}

#[test]
fn test_try_add_semantic_containers_unsupported_language() {
    use crate::{File, FileMode};
    use std::borrow::Cow;

    let file = File {
        old_path: None,
        path: Cow::Borrowed(std::path::Path::new("test.txt")),
        file_mode: FileMode::FILE_DEFAULT,
        sections: Vec::new(),
        containers: None,
    };

    let enhanced_file = try_add_semantic_containers(file, "old", "new");

    // Should return unchanged for unsupported language
    assert!(enhanced_file.containers.is_none());
}
