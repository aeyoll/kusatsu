use kusatsu_encrypt::{Encryption, EncryptionKey};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔐 Kusatsu Encryption Example");
    println!("=============================");

    // Generate a new encryption key
    let key = EncryptionKey::generate();
    let key_string = key.to_base64();
    println!("🔑 Generated encryption key: {}", key_string);

    // Sample data
    let original_data = b"Hello, World! This is a secret message that will be encrypted.";
    let filename = "secret.txt";

    println!("\n📄 Original data:");
    println!("   Content: {}", String::from_utf8_lossy(original_data));
    println!("   Size: {} bytes", original_data.len());

    // Encrypt the data
    println!("\n🔒 Encrypting data...");
    let encrypted_data = Encryption::encrypt(original_data, &key)?;

    println!(
        "   Encrypted size: {} bytes",
        encrypted_data.ciphertext.len()
    );
    println!("   Nonce length: {} bytes", encrypted_data.nonce.len());

    // Encrypt filename separately (demonstrating how the backend would do it)
    let encrypted_filename = Encryption::encrypt(filename.as_bytes(), &key)?;
    println!(
        "   Encrypted filename length: {} bytes",
        encrypted_filename.ciphertext.len()
    );

    // Simulate storing the key in a URL anchor (what would happen in real usage)
    let share_url = format!("https://kusatsu.io/files/12345#{}", key_string);
    println!("\n🔗 Shareable URL: {}", share_url);
    println!("   Note: The key after '#' is never sent to the server!");

    // Extract key from URL (simulating client-side decryption)
    let anchor_key_string = share_url.split('#').nth(1).unwrap();
    let decryption_key = EncryptionKey::from_base64(anchor_key_string)?;

    // Decrypt the data
    println!("\n🔓 Decrypting data...");
    let decrypted_data = Encryption::decrypt(&encrypted_data, &decryption_key)?;
    let decrypted_filename_bytes = Encryption::decrypt(&encrypted_filename, &decryption_key)?;
    let decrypted_filename = String::from_utf8(decrypted_filename_bytes)?;

    println!("   Filename: {}", decrypted_filename);
    println!("   Size: {} bytes", decrypted_data.len());
    println!("   Content: {}", String::from_utf8_lossy(&decrypted_data));

    // Verify the data matches
    if original_data == &decrypted_data[..] && filename == decrypted_filename {
        println!("\n✅ Success! Data encrypted and decrypted successfully.");
        println!("   Original and decrypted data match perfectly.");
    } else {
        println!("\n❌ Error! Data mismatch.");
    }

    // Demonstrate string encryption convenience method
    println!("\n📝 Testing string encryption convenience method...");
    let test_string = "This is a test string for encryption!";
    let encrypted_string = Encryption::encrypt_string(test_string, &key)?;
    let decrypted_string = Encryption::decrypt_string(&encrypted_string, &key)?;

    println!("   Original: {}", test_string);
    println!("   Encrypted (base64): {}", encrypted_string);
    println!("   Decrypted: {}", decrypted_string);

    if test_string == decrypted_string {
        println!("   ✅ String encryption works!");
    } else {
        println!("   ❌ String encryption failed!");
    }

    // Demonstrate wrong key failure
    println!("\n🚫 Testing with wrong key...");
    let wrong_key = EncryptionKey::generate();
    match Encryption::decrypt(&encrypted_data, &wrong_key) {
        Ok(_) => println!("   Unexpected success!"),
        Err(e) => println!("   Expected failure: {}", e),
    }

    println!("\n🎉 Example completed successfully!");

    Ok(())
}
