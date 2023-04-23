pub fn device_register(client_id: String, username: String, password: String) {
    println!("device {client_id} {username} {password} registered");
}

pub fn device_register_batch(file_path: String) {
    println!("devices from {file_path} auth file registered");
}
