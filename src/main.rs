use asar::{AsarReader, Result};
use std::env;
use std::fs;
use std::fs::File;
use std::io::{Error, ErrorKind, Read, Write};
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    let file: &String = &args[1];

    let compressed_file: &str = "output.7z";

    let path: &Path = Path::new(file);
    if !path.exists() {
        eprintln!("[-] Error: {} not found.", file);
        std::process::exit(1);
    }

    extract_nsis_exe(file, compressed_file);

    extract_7z(compressed_file);

    if let Err(err) = extract_asar(file, "Decompressed") {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
}

fn extract_nsis_exe(file_path: &str, output_path: &str) {
    let header_7z: Vec<u8> = vec![0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C];

    // lzmaSignature + Use of wildcard 0xFF for incomplete bytes in the end hex
    let lzma_signature = vec![
        0x23, 0x03, 0x01, 0x01, 0x05, 0x5D, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    ];

    println!("[*] Starting dump of compressed data...");

    if let Err(err) = dump(file_path, &header_7z, &lzma_signature, output_path) {
        eprintln!("[!] Error: {}", err);
        std::process::exit(1);
    }

    println!("  [+] Content dumped into {}", output_path);
}

fn find_hex_string(data: &[u8], hex_string: &[u8]) -> Option<usize> {
    let hex_len: usize = hex_string.len();
    for i in 0..data.len() - hex_len {
        if data[i..].starts_with(hex_string) {
            return Some(i);
        }
    }
    None
}

fn find_partial_hex_string(data: &[u8], hex_string: &[u8]) -> Option<usize> {
    let hex_len: usize = hex_string.len();
    for i in 0..data.len() - hex_len {
        let match_len = data[i..]
            .iter()
            .zip(hex_string.iter())
            .take_while(|(a, b)| a == b || **b == 0xFF)
            .count();
        if match_len == hex_len {
            return Some(i);
        }
    }
    None
}

fn dump(
    file_path: &str,
    start_hex: &[u8],
    end_hex: &[u8],
    output_file_path: &str,
) -> std::io::Result<()> {
    let mut file: File = File::open(file_path)?;
    let mut buffer: Vec<u8> = Vec::new();
    file.read_to_end(&mut buffer)?;

    let start_index = find_hex_string(&buffer, start_hex).ok_or(Error::new(
        ErrorKind::NotFound,
        "Start hex string not found",
    ))?;
    let end_index: usize = find_partial_hex_string(&buffer, end_hex)
        .ok_or(Error::new(ErrorKind::NotFound, "End hex string not found"))?;

    let content_between_hex_strings: &[u8] = &buffer[start_index..end_index + end_hex.len()];

    let mut output_file: File = File::create(output_file_path)?;
    output_file.write_all(content_between_hex_strings)?;

    Ok(())
}

fn extract_7z(compressed_file: &str) {
    println!("[*] Starting extraction of compressed data...");

    sevenz_rust::decompress_file(compressed_file, "Decompressed").expect("complete");

    // Cleanup
    let _ = fs::remove_file(compressed_file);

    println!("  [+] Extracted {}", compressed_file);
}

fn extract_asar(original_filename: &str, output_path: &str) -> Result<()> {
    println!("[*] Extracting asar content...");

    let asar_path: String = output_path.to_owned() + "/resources/app.asar";

    let asar_file: Vec<u8> = fs::read(asar_path)?;
    let asar: AsarReader<'_> = AsarReader::new(&asar_file, None)?;

    let output_folder: &str = &(original_filename.to_owned() + "_extracted");

    fs::create_dir_all(output_folder)?;

    for (path, file_info) in asar.files() {
        let file_path: String = path.to_string_lossy().into_owned();
        let output_path: std::path::PathBuf = Path::new(output_folder).join(&file_path);

        // Create directories if necessary
        if let Some(parent_dir) = output_path.parent() {
            fs::create_dir_all(parent_dir)?;
        }

        let mut file: File = fs::File::create(&output_path)?;
        file.write_all(file_info.data())?;
    }

    let _ = fs::remove_dir_all(output_path);

    println!("  [+] Done.");

    println!(
        "[*] You can find the extracted files in the ./{} folder.",
        output_folder
    );

    Ok(())
}
