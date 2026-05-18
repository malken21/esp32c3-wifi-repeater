use anyhow::Result;
use esp_idf_svc::hal::modem::WifiModemPeripheral;
use esp_idf_svc::hal::peripheral::Peripheral;
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use embedded_svc::wifi::{ClientConfiguration, Configuration, AccessPointConfiguration, AuthMethod};
use crate::config::RepeaterConfig;
use std::net::Ipv4Addr;
use esp_idf_svc::sys;
use esp_idf_svc::handle::RawHandle;
use heapless::String as HString;

pub struct WifiManager<'a> {
    wifi: BlockingWifi<EspWifi<'a>>,
}

impl<'a> WifiManager<'a> {
    pub fn new<M: WifiModemPeripheral>(
        modem: impl Peripheral<P = M> + 'a,
        sys_loop: EspSystemEventLoop,
        nvs: EspDefaultNvsPartition,
    ) -> Result<Self> {
        let esp_wifi = EspWifi::new(modem, sys_loop.clone(), Some(nvs))?;
        let wifi = BlockingWifi::wrap(esp_wifi, sys_loop)?;
        Ok(Self { wifi })
    }

    pub fn start(&mut self, config: &RepeaterConfig) -> Result<Ipv4Addr> {
        let auth_method = self.determine_auth_method(&config.ap_password);

        let sta_ssid = self.to_heapless_32(&config.sta_ssid, "STA SSID")?;
        let sta_pass = self.to_heapless_64(&config.sta_password, "STA Password")?;
        let ap_ssid = self.to_heapless_32(&config.ap_ssid, "AP SSID")?;
        let ap_pass = self.to_heapless_64(&config.ap_password, "AP Password")?;

        self.wifi.set_configuration(&Configuration::Mixed(
            ClientConfiguration {
                ssid: sta_ssid,
                password: sta_pass,
                ..Default::default()
            },
            AccessPointConfiguration {
                ssid: ap_ssid,
                password: ap_pass,
                channel: config.ap_channel,
                auth_method,
                ..Default::default()
            },
        ))?;

        self.wifi.start()?;

        if config.sta_use_static {
            self.apply_static_ip_configuration(config)?;
        }

        self.wifi.connect()?;

        self.wifi.wait_netif_up()?;
        let ip_info = self.wifi.wifi().sta_netif().get_ip_info()?;

        Ok(ip_info.ip)
    }

    fn determine_auth_method(&self, password: &str) -> AuthMethod {
        if password.is_empty() {
            AuthMethod::None
        } else {
            AuthMethod::WPA2Personal
        }
    }

    fn to_heapless_32(&self, s: &str, label: &str) -> Result<HString<32>> {
        s.try_into().map_err(|_| anyhow::anyhow!("{} too long", label))
    }

    fn to_heapless_64(&self, s: &str, label: &str) -> Result<HString<64>> {
        s.try_into().map_err(|_| anyhow::anyhow!("{} too long", label))
    }

    fn apply_static_ip_configuration(&mut self, config: &RepeaterConfig) -> Result<()> {
        let ip: Ipv4Addr = config.sta_static_ip.parse()?;
        let gw: Ipv4Addr = config.sta_gateway.parse()?;
        let nm: Ipv4Addr = config.sta_netmask.parse()?;

        unsafe {
            let sta_netif = self.wifi.wifi().sta_netif().handle();
            let ap_netif = self.wifi.wifi().ap_netif().handle();

            sys::esp_netif_dhcpc_stop(sta_netif);
            
            self.set_netif_ip_info(sta_netif, ip, gw, nm);
            self.set_netif_dns_server(sta_netif, gw);
            self.set_netif_dns_server(ap_netif, gw);
        }

        Ok(())
    }

    unsafe fn set_netif_ip_info(&self, handle: *mut sys::esp_netif_t, ip: Ipv4Addr, gw: Ipv4Addr, nm: Ipv4Addr) {
        let ip_info = sys::esp_netif_ip_info_t {
            ip: self.to_esp_ip4_addr(ip),
            gw: self.to_esp_ip4_addr(gw),
            netmask: self.to_esp_ip4_addr(nm),
        };
        sys::esp_netif_set_ip_info(handle, &ip_info);
    }

    unsafe fn set_netif_dns_server(&self, handle: *mut sys::esp_netif_t, dns_addr: Ipv4Addr) {
        let mut dns_info = sys::esp_netif_dns_info_t::default();
        dns_info.ip.u_addr.ip4.addr = u32::from(dns_addr).to_be();
        sys::esp_netif_set_dns_info(handle, sys::esp_netif_dns_type_t_ESP_NETIF_DNS_MAIN, &mut dns_info);
    }

    fn to_esp_ip4_addr(&self, addr: Ipv4Addr) -> sys::esp_ip4_addr {
        sys::esp_ip4_addr { addr: u32::from(addr).to_be() }
    }
}
