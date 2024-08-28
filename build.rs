use std::{env::current_dir, fs::{self, File}, io::Write};

fn main() {
    //Generate Starknet bindings
    let strk_abi_base = current_dir()
        .expect("failed to get current dir")
        .join("abis");
    let strk_bind_base = current_dir()
        .expect("failed to get current dir")
        .join("src/bindings");
    let strk_deployments = [
        ("Liquidate", "liquidate"),
    ];
    
    // create destination folders if they doesn't exist
    fs::create_dir_all(strk_bind_base.clone()).expect("error creating output folders");
    let mut file = File::create(strk_bind_base.join("mod.rs")).expect("failed to create mod.rs");

    for (abi_file, bind_out) in strk_deployments {
        let contract_files = strk_abi_base.join(format!("vesu_liquidate_{abi_file}.contract_class.json"));
        let contract_files = contract_files.to_str().unwrap();
        let abigen = cainome::rs::Abigen::new(abi_file, contract_files);

        abigen
        .generate()
        .expect(format!("Fail to generate bindings {}", contract_files).as_str())
        .write_to_file(&strk_bind_base.join(format!("{bind_out}.rs")).to_str().expect("valid utf8 path"))
        .expect(format!("Fail to write bindings to file in {:?}", strk_bind_base).as_str());
        
        file.write_all(format!("pub mod {};", bind_out).as_bytes()).expect("failed to write into mod.rs");
    }


}