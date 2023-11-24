fn main() {}
/*use uniffi_bindgen::bindings::TargetLanguage;

fn main() {
    println!("cargo:rerun-if-changed=src/client.udl");

    use camino::Utf8PathBuf;
    let r = uniffi::generate_scaffolding("src/client.udl");
    match r {
        Ok(_) => println!("Generated scaffolding done"),
        Err(e) => {
            panic!("Error generating scaffolding: {}", e);
        }
    }
    // build bindings for kotlin, swift and python
    for lang in vec![
        TargetLanguage::Kotlin,
        TargetLanguage::Swift,
        TargetLanguage::Python,
    ] {
        let r = uniffi::generate_bindings(
            Utf8PathBuf::from("src/client.udl").as_ref(),
            None,
            vec![lang],
            Some(Utf8PathBuf::from(format!("bindings_{}", lang)).as_ref()),
            None,
            false,
        );
        match r {
            Ok(_) => println!("Generated bindings done"),
            Err(e) => {
                panic!("Error generating bindings: {}", e);
            }
        }
    }
}
*/
