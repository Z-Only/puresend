//! 网络工具模块

use std::net::Ipv4Addr;

/// 获取本地所有有效的 IPv4 地址
///
/// 返回的地址列表按优先级排序：
/// - 私有网段（192.168.x.x、10.x.x.x、172.16-31.x.x）优先
/// - 公网 IP 次之
///
/// 如果没有找到任何有效 IP，返回 localhost 地址作为回退
pub fn get_local_ips() -> Vec<String> {
    use local_ip_address::list_afinet_netifas;

    let mut ips: Vec<(String, u8)> = Vec::new();

    // 枚举所有网络接口
    if let Ok(network_interfaces) = list_afinet_netifas() {
        for (_, ip_addr) in network_interfaces {
            // 只处理 IPv4 地址
            if let std::net::IpAddr::V4(ipv4) = ip_addr {
                // 过滤掉回环地址（127.x.x.x）
                if ipv4.is_loopback() {
                    continue;
                }

                // 过滤掉 link-local 地址（169.254.x.x）
                if is_link_local(ipv4) {
                    continue;
                }

                // 根据优先级分配权重
                let priority = get_ip_priority(ipv4);
                ips.push((ipv4.to_string(), priority));
            }
        }
    }

    // 按优先级排序（权重越小优先级越高）
    ips.sort_by_key(|(_, priority)| *priority);

    // 提取 IP 地址
    let result: Vec<String> = ips.into_iter().map(|(ip, _)| ip).collect();

    // 如果没有找到任何有效 IP，返回 localhost 作为回退
    if result.is_empty() {
        vec!["127.0.0.1".to_string()]
    } else {
        result
    }
}

/// 判断是否为 link-local 地址（169.254.x.x）
fn is_link_local(ip: Ipv4Addr) -> bool {
    ip.octets()[0] == 169 && ip.octets()[1] == 254
}

/// 获取 IP 地址的优先级权重
///
/// 返回值越小，优先级越高：
/// - 0: 192.168.x.x
/// - 1: 10.x.x.x
/// - 2: 172.16-31.x.x
/// - 3: 其他公网 IP
fn get_ip_priority(ip: Ipv4Addr) -> u8 {
    let octets = ip.octets();

    // 192.168.x.x - 优先级最高
    if octets[0] == 192 && octets[1] == 168 {
        return 0;
    }

    // 10.x.x.x
    if octets[0] == 10 {
        return 1;
    }

    // 172.16-31.x.x
    if octets[0] == 172 && octets[1] >= 16 && octets[1] <= 31 {
        return 2;
    }

    // 其他公网 IP
    3
}
