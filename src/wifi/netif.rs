use esp_idf_svc::sys;
use std::net::Ipv4Addr;

const BACKUP_DNS: Ipv4Addr = Ipv4Addr::new(1, 1, 1, 1);
const DHCP_OFFER_DNS: u8 = 0x02;
const HTTP_FRIENDLY_MTU: u16 = 1360;

pub(super) unsafe fn set_mtu(sta_netif: *mut sys::esp_netif_t, ap_netif: *mut sys::esp_netif_t) {
    let sta_lwip = sys::esp_netif_get_netif_impl(sta_netif) as *mut sys::netif;
    let ap_lwip = sys::esp_netif_get_netif_impl(ap_netif) as *mut sys::netif;

    if !sta_lwip.is_null() && !ap_lwip.is_null() {
        (*sta_lwip).mtu = HTTP_FRIENDLY_MTU;
        (*ap_lwip).mtu = HTTP_FRIENDLY_MTU;
        log::info!("MTU set to 1360 for raw lwIP netifs.");
    } else {
        log::error!("Failed to get raw lwIP netifs.");
    }
}

pub(super) unsafe fn enable_napt_or_restart(ap_netif: *mut sys::esp_netif_t) {
    for _ in 0..10 {
        if sys::esp_netif_is_netif_up(ap_netif) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    let napt_err = sys::esp_netif_napt_enable(ap_netif as *mut _);
    if napt_err == 0 {
        log::info!("NAPT enabled successfully on AP interface.");
    } else {
        log::error!("NAPT enable failed (err={}). Ignoring to prevent reboot loop.", napt_err);
    }
}

pub(super) unsafe fn set_ip_info(
    handle: *mut sys::esp_netif_t,
    ip: Ipv4Addr,
    gw: Ipv4Addr,
    nm: Ipv4Addr,
) {
    let ip_info = sys::esp_netif_ip_info_t {
        ip: to_esp_ip4_addr(ip),
        gw: to_esp_ip4_addr(gw),
        netmask: to_esp_ip4_addr(nm),
    };
    sys::esp_netif_set_ip_info(handle, &ip_info);
}

pub(super) unsafe fn set_dns_servers(handle: *mut sys::esp_netif_t, primary_dns: Ipv4Addr) {
    set_dns_slot(
        handle,
        sys::esp_netif_dns_type_t_ESP_NETIF_DNS_MAIN,
        primary_dns,
    );
    set_dns_slot(
        handle,
        sys::esp_netif_dns_type_t_ESP_NETIF_DNS_BACKUP,
        BACKUP_DNS,
    );
}

unsafe fn set_dns_slot(
    handle: *mut sys::esp_netif_t,
    slot: sys::esp_netif_dns_type_t,
    addr: Ipv4Addr,
) {
    let mut dns_info = sys::esp_netif_dns_info_t::default();
    dns_info.ip.u_addr.ip4.addr = u32::from(addr).to_be();
    dns_info.ip.type_ = sys::lwip_ip_addr_type_IPADDR_TYPE_V4 as u8;
    sys::esp_netif_set_dns_info(handle, slot, &mut dns_info);
}

pub(super) unsafe fn offer_dns_via_dhcp(ap_netif: *mut sys::esp_netif_t, dns_addr: Ipv4Addr) {
    sys::esp_netif_dhcps_stop(ap_netif);

    let mut offer = DHCP_OFFER_DNS;
    let ret = sys::esp_netif_dhcps_option(
        ap_netif,
        sys::esp_netif_dhcp_option_mode_t_ESP_NETIF_OP_SET,
        sys::esp_netif_dhcp_option_id_t_ESP_NETIF_DOMAIN_NAME_SERVER,
        &mut offer as *mut _ as *mut _,
        core::mem::size_of_val(&offer) as u32,
    );

    if ret != 0 {
        log::error!("Failed to enable DHCP DNS offer flag: {}", ret);
    } else {
        log::info!("DHCP DNS offering enabled (DNS: {})", dns_addr);
    }

    sys::esp_netif_dhcps_start(ap_netif);
}

fn to_esp_ip4_addr(addr: Ipv4Addr) -> sys::esp_ip4_addr {
    sys::esp_ip4_addr {
        addr: u32::from(addr).to_be(),
    }
}

const MSS_CLAMP: u16 = HTTP_FRIENDLY_MTU - 40;

static mut ORIG_STA_OUTPUT: sys::netif_output_fn = None;
static mut ORIG_AP_OUTPUT: sys::netif_output_fn = None;

unsafe fn clamp_syn_mss(p: *mut sys::pbuf) {
    if p.is_null() {
        return;
    }
    let len = (*p).len as usize;
    if len < 40 {
        return;
    }
    let d = (*p).payload as *mut u8;

    let ihl = ((*d & 0x0f) as usize) * 4;
    if ihl < 20 || len < ihl + 20 || *d.add(9) != 6 {
        return;
    }

    let tcp = d.add(ihl);
    if *tcp.add(13) & 0x02 == 0 {
        return;
    }
    let doff = ((*tcp.add(12) >> 4) as usize) * 4;
    if doff < 20 || len < ihl + doff {
        return;
    }

    let mut i = 20usize;
    while i + 1 < doff {
        let k = *tcp.add(i);
        if k == 0 {
            break;
        }
        if k == 1 {
            i += 1;
            continue;
        }
        let l = *tcp.add(i + 1) as usize;
        if l < 2 || i + l > doff {
            break;
        }
        if k == 2 && l == 4 {
            let mp = tcp.add(i + 2) as *mut u16;
            let old_mss = u16::from_be(*mp);
            if old_mss > MSS_CLAMP {
                *mp = MSS_CLAMP.to_be();
                let cp = tcp.add(16) as *mut u16;
                let hc = u16::from_be(*cp);
                let s = (!hc as u32)
                    .wrapping_add(!old_mss as u32)
                    .wrapping_add(MSS_CLAMP as u32);
                let s = (s & 0xffff).wrapping_add(s >> 16);
                let s = (s & 0xffff).wrapping_add(s >> 16);
                *cp = (!(s as u16)).to_be();
            }
            return;
        }
        i += l;
    }
}

unsafe extern "C" fn sta_output_mss(
    netif: *mut sys::netif,
    p: *mut sys::pbuf,
    ipaddr: *const sys::ip4_addr_t,
) -> sys::err_t {
    clamp_syn_mss(p);
    if let Some(f) = ORIG_STA_OUTPUT {
        f(netif, p, ipaddr)
    } else {
        -17
    }
}

unsafe extern "C" fn ap_output_mss(
    netif: *mut sys::netif,
    p: *mut sys::pbuf,
    ipaddr: *const sys::ip4_addr_t,
) -> sys::err_t {
    clamp_syn_mss(p);
    if let Some(f) = ORIG_AP_OUTPUT {
        f(netif, p, ipaddr)
    } else {
        -17
    }
}

pub(super) unsafe fn install_mss_clamp(
    sta_netif: *mut sys::esp_netif_t,
    ap_netif: *mut sys::esp_netif_t,
) {
    let sta = sys::esp_netif_get_netif_impl(sta_netif) as *mut sys::netif;
    let ap = sys::esp_netif_get_netif_impl(ap_netif) as *mut sys::netif;

    if !sta.is_null() {
        ORIG_STA_OUTPUT = (*sta).output;
        (*sta).output = Some(sta_output_mss);
        log::info!("TCP MSS clamp ({}) installed on STA output.", MSS_CLAMP);
    }
    if !ap.is_null() {
        ORIG_AP_OUTPUT = (*ap).output;
        (*ap).output = Some(ap_output_mss);
        log::info!("TCP MSS clamp ({}) installed on AP output.", MSS_CLAMP);
    }
}
