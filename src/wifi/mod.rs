mod netif;

use crate::config::RepeaterConfig;
use anyhow::Result;
use embedded_svc::wifi::{
    AccessPointConfiguration, AuthMethod, ClientConfiguration, Configuration,
};
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::modem::WifiModemPeripheral;
use esp_idf_svc::hal::peripheral::Peripheral;
use esp_idf_svc::handle::RawHandle;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sys;
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};
use heapless::String as HString;
use std::net::Ipv4Addr;

const FALLBACK_DNS: Ipv4Addr = Ipv4Addr::new(1, 1, 1, 1);
const RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(5);

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
        self.configure_wifi(config)?;
        self.wifi.start()?;
        self.configure_power_save();

        if config.sta_use_static {
            self.apply_static_ip_configuration(config)?;
        }

        self.connect_until_ready("Connecting to WiFi AP...")?;
        let ip_info = self.wifi.wifi().sta_netif().get_ip_info()?;
        self.configure_routing(config, ip_info.dns);
        self.configure_pm();

        Ok(ip_info.ip)
    }

    pub fn is_connected(&self) -> Result<bool> {
        Ok(self.wifi.is_connected()?)
    }

    pub fn reconnect(&mut self) -> Result<Ipv4Addr> {
        self.connect_until_ready("Reconnecting to WiFi AP...")?;
        let ip_info = self.wifi.wifi().sta_netif().get_ip_info()?;
        Ok(ip_info.ip)
    }

    fn configure_wifi(&mut self, config: &RepeaterConfig) -> Result<()> {
        self.wifi.set_configuration(&Configuration::Mixed(
            ClientConfiguration {
                ssid: self.to_heapless::<32>(&config.sta_ssid, "STA SSID")?,
                password: self.to_heapless::<64>(&config.sta_password, "STA Password")?,
                ..Default::default()
            },
            AccessPointConfiguration {
                ssid: self.to_heapless::<32>(&config.ap_ssid, "AP SSID")?,
                password: self.to_heapless::<64>(&config.ap_password, "AP Password")?,
                channel: config.ap_channel,
                auth_method: self.ap_auth_method(&config.ap_password),
                max_connections: 4,
                ..Default::default()
            },
        ))?;
        Ok(())
    }

    fn configure_power_save(&self) {
        unsafe {
            let ret = sys::esp_wifi_set_ps(sys::wifi_ps_type_t_WIFI_PS_MIN_MODEM);
            if ret != 0 {
                log::error!("Failed to set WiFi modem sleep: {}", ret);
            } else {
                log::info!("WiFi modem sleep (MIN_MODEM) enabled.");
            }
        }
    }

    fn configure_pm(&self) {
        unsafe {
            let pm_config = sys::esp_pm_config_t {
                max_freq_mhz: 80,
                min_freq_mhz: 40,
                light_sleep_enable: true,
            };
            let ret = sys::esp_pm_configure(
                &pm_config as *const sys::esp_pm_config_t as *const core::ffi::c_void,
            );
            if ret != 0 {
                log::error!("Failed to configure PM: {}", ret);
            } else {
                log::info!("PM configured: 80MHz max, 40MHz min, light sleep enabled.");
            }
        }
    }

    fn connect_until_ready(&mut self, attempt_message: &str) -> Result<()> {
        loop {
            log::info!("{}", attempt_message);
            match self.connect_once() {
                Ok(()) => return Ok(()),
                Err(e) => log::warn!("WiFi connection attempt failed: {}. Retrying.", e),
            }
            std::thread::sleep(RETRY_DELAY);
        }
    }

    fn connect_once(&mut self) -> Result<()> {
        self.wifi.connect()?;
        log::info!("Connected to AP. Waiting for IP...");

        if let Err(e) = self.wifi.wait_netif_up() {
            let _ = self.wifi.disconnect();
            return Err(e.into());
        }

        Ok(())
    }

    fn configure_routing(&self, config: &RepeaterConfig, sta_dns: Option<Ipv4Addr>) {
        unsafe {
            let sta = self.wifi.wifi().sta_netif().handle();
            let ap = self.wifi.wifi().ap_netif().handle();

            netif::set_mtu(sta, ap);
            netif::enable_napt_or_restart(ap);
            netif::install_mss_clamp(sta, ap);

            if !config.sta_use_static {
                let upstream_dns = sta_dns.unwrap_or(FALLBACK_DNS);
                log::info!("Upstream DNS: {}, offering {} to AP clients via DHCP", upstream_dns, FALLBACK_DNS);
                netif::set_dns_servers(ap, FALLBACK_DNS);
                netif::offer_dns_via_dhcp(ap, FALLBACK_DNS);
            } else {
                log::info!("Static IP mode DNS is already configured.");
            }
        }
    }

    fn ap_auth_method(&self, password: &str) -> AuthMethod {
        if password.is_empty() {
            AuthMethod::None
        } else {
            AuthMethod::WPA2Personal
        }
    }

    fn to_heapless<const N: usize>(&self, s: &str, label: &str) -> Result<HString<N>> {
        s.try_into()
            .map_err(|_| anyhow::anyhow!("{} too long", label))
    }

    fn apply_static_ip_configuration(&mut self, config: &RepeaterConfig) -> Result<()> {
        let ip = config.sta_static_ip.parse()?;
        let gw = config.sta_gateway.parse()?;
        let nm = config.sta_netmask.parse()?;

        unsafe {
            let sta = self.wifi.wifi().sta_netif().handle();
            let ap = self.wifi.wifi().ap_netif().handle();

            sys::esp_netif_dhcpc_stop(sta);
            netif::set_ip_info(sta, ip, gw, nm);
            netif::set_dns_servers(sta, gw);
            netif::set_dns_servers(ap, gw);
            netif::offer_dns_via_dhcp(ap, gw);
        }

        Ok(())
    }
}
