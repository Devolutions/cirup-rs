use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn normalize_windows_version(version: &str) -> (String, String) {
    let semver_no_pre = version.split_once('-').map_or(version, |(core, _)| core);
    let semver = semver_no_pre.split_once('+').map_or(semver_no_pre, |(core, _)| core);

    let mut parts = semver
        .split('.')
        .map(|part| part.parse::<u16>().unwrap_or(0))
        .collect::<Vec<_>>();

    while parts.len() < 4 {
        parts.push(0);
    }

    parts.truncate(4);

    let dots = parts.iter().map(u16::to_string).collect::<Vec<_>>().join(".");
    let commas = parts.iter().map(u16::to_string).collect::<Vec<_>>().join(",");

    (dots, commas)
}

fn generate_version_rc() -> String {
    let package_version = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".to_owned());
    let (version_dots, version_commas) = normalize_windows_version(package_version.as_str());

    let company_name = "Devolutions Inc.";
    let file_description = "cirup command-line tool";
    let product_name = "cirup-rs";
    let original_filename = "cirup.exe";
    let legal_copyright = "Copyright 2019-2026 Devolutions Inc.";

    format!(
        r#"#include <winver.h>

VS_VERSION_INFO VERSIONINFO
FILEVERSION {version_commas}
PRODUCTVERSION {version_commas}
FILEFLAGSMASK VS_FFI_FILEFLAGSMASK
FILEFLAGS 0x0L
FILEOS VOS_NT_WINDOWS32
FILETYPE VFT_APP
FILESUBTYPE VFT2_UNKNOWN
BEGIN
    BLOCK "StringFileInfo"
    BEGIN
        BLOCK "040904B0"
        BEGIN
            VALUE "CompanyName", "{company_name}"
            VALUE "FileDescription", "{file_description}"
            VALUE "FileVersion", "{version_dots}"
            VALUE "InternalName", "{original_filename}"
            VALUE "LegalCopyright", "{legal_copyright}"
            VALUE "OriginalFilename", "{original_filename}"
            VALUE "ProductName", "{product_name}"
            VALUE "ProductVersion", "{version_dots}"
        END
    END
    BLOCK "VarFileInfo"
    BEGIN
        VALUE "Translation", 0x409, 1200
    END
END
"#
    )
}

fn main() {
    println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is not set"));
    let version_rc_file = out_dir.join("version.rc");
    let version_rc_data = generate_version_rc();

    let mut file = File::create(&version_rc_file).expect("Failed to create version.rc file");
    file.write_all(version_rc_data.as_bytes())
        .expect("Failed to write version.rc file");

    let _ = embed_resource::compile(&version_rc_file, embed_resource::NONE);
}
