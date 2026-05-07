fn main() {
    println!("cargo:rerun-if-env-changed=DEP_OPENSSL_VERSION_NUMBER");
    println!("cargo:rerun-if-env-changed=DEP_OPENSSL_LIBRESSL_VERSION_NUMBER");

    if let Ok(version) = std::env::var("DEP_OPENSSL_VERSION_NUMBER") {
        println!("cargo:rustc-env=FQ_OPENSSL_SOURCE_VERSION_NUMBER={version}");
    }
    if let Ok(version) = std::env::var("DEP_OPENSSL_LIBRESSL_VERSION_NUMBER") {
        println!("cargo:rustc-env=FQ_OPENSSL_LIBRESSL_VERSION_NUMBER={version}");
    }
}
