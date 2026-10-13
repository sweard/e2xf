use e2xf::state::ConversionRequest;
use std::fs;
mod support;
use support::{settings, ORIGINAL};

#[test]
fn desktop_requests_convert_both_modes_and_keep_input_snapshots() {
    let root = tempfile::tempdir().unwrap();
    let normal_root = root.path().join("normal");
    let fast_root = root.path().join("fast");
    fs::create_dir_all(&normal_root).unwrap();
    fs::create_dir_all(&fast_root).unwrap();
    let mut normal = settings(&normal_root, false);
    let fast = settings(&fast_root, true);
    let request = ConversionRequest::new(&normal, &[]).unwrap();
    normal.config = "broken".into();
    assert_eq!(request.run().unwrap().files_updated, 2);
    assert_eq!(
        ConversionRequest::new(&fast, &[])
            .unwrap()
            .run()
            .unwrap()
            .files_updated,
        2
    );
    for folder in ["values", "values-zh"] {
        let xml = format!("module/res/{folder}/strings.xml");
        assert_eq!(
            fs::read_to_string(normal_root.join(&xml)).unwrap(),
            fs::read_to_string(fast_root.join(xml)).unwrap()
        );
    }
    assert!(
        fs::read_to_string(fast_root.join("module/res/values-zh/strings.xml"))
            .unwrap()
            .contains("你好")
    );
}

#[test]
fn core_errors_reach_the_desktop_without_overwriting_other_languages() {
    let root = tempfile::tempdir().unwrap();
    let settings = settings(root.path(), true);
    fs::write(
        root.path().join("module/res/values-zh/strings.xml"),
        "<resources><string>",
    )
    .unwrap();
    let error = ConversionRequest::new(&settings, &[])
        .unwrap()
        .run()
        .unwrap_err();
    assert!(!error.is_empty());
    assert_eq!(
        fs::read_to_string(root.path().join("module/res/values/strings.xml")).unwrap(),
        ORIGINAL
    );
}
