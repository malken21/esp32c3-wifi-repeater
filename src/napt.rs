extern "C" {
    pub fn ip_napt_enable(ip: u32, enable: i32);
}

pub fn enable_napt(ip: u32) {
    const ENABLE_FLAG: i32 = 1;
    unsafe {
        ip_napt_enable(ip, ENABLE_FLAG);
    }
}
