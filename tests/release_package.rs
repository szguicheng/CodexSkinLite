use std::path::Path;

#[test]
fn release_package_declares_icon_and_developer_id_signing() {
    let plist = std::fs::read_to_string("resources/Info.plist").unwrap();
    let script = std::fs::read_to_string("scripts/package-app.sh").unwrap();

    assert!(Path::new("resources/CodexSkinLite-icon.png").exists());
    assert!(plist.contains("<key>CFBundleIconFile</key>"));
    assert!(plist.contains("<string>CodexSkinLite.icns</string>"));
    assert!(script.contains("iconutil -c icns"));
    assert!(script.contains("Developer ID Application"));
    assert!(script.contains("codesign --force"));
}
