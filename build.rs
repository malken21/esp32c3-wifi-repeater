fn main() {
    embuild::espidf::sysenv::output();
    println!("cargo:rerun-if-env-changed=REPEATER_STA_SSID");
    println!("cargo:rerun-if-env-changed=REPEATER_STA_PASS");
}
