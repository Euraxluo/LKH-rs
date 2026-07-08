use std::process::Command;

#[test]
fn cli_solves_tiny_fixture() {
    let output = Command::new(env!("CARGO_BIN_EXE_lkh"))
        .arg("--par")
        .arg("tests/fixtures/tiny.par")
        .output()
        .expect("run lkh binary");

    assert!(
        output.status.success(),
        "status: {:?}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Best cost:"), "stdout:\n{stdout}");
}
